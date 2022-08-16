use std::fmt;

use ring::{aead, hkdf};

use crate::tls::rustls::{
    cipher::{make_nonce, Iv, MessageDecrypter, MessageEncrypter},
    error::Error,
    msgs::{
        base::Payload,
        enums::{CipherSuite, ContentType, ProtocolVersion},
        fragmenter::MAX_FRAGMENT_LEN,
        message::{BorrowedPlainMessage, OpaqueMessage, PlainMessage},
    },
    suites::{BulkAlgorithm, CipherSuiteCommon, SupportedCipherSuite},
};

pub mod key_schedule;
use key_schedule::{derive_traffic_iv, derive_traffic_key};
use puffin::codec::Codec;

/// The TLS1.3 ciphersuite TLS_CHACHA20_POLY1305_SHA256
pub static TLS13_CHACHA20_POLY1305_SHA256: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(TLS13_CHACHA20_POLY1305_SHA256_INTERNAL);

pub static TLS13_CHACHA20_POLY1305_SHA256_INTERNAL: &Tls13CipherSuite = &Tls13CipherSuite {
    common: CipherSuiteCommon {
        suite: CipherSuite::TLS13_CHACHA20_POLY1305_SHA256,
        bulk: BulkAlgorithm::Chacha20Poly1305,
        aead_algorithm: &ring::aead::CHACHA20_POLY1305,
    },
    hkdf_algorithm: ring::hkdf::HKDF_SHA256,
    confidentiality_limit: u64::MAX,
    integrity_limit: 1 << 36,
};

/// The TLS1.3 ciphersuite TLS_AES_256_GCM_SHA384
pub static TLS13_AES_256_GCM_SHA384: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&Tls13CipherSuite {
        common: CipherSuiteCommon {
            suite: CipherSuite::TLS13_AES_256_GCM_SHA384,
            bulk: BulkAlgorithm::Aes256Gcm,
            aead_algorithm: &ring::aead::AES_256_GCM,
        },
        hkdf_algorithm: ring::hkdf::HKDF_SHA384,
        confidentiality_limit: 1 << 23,
        integrity_limit: 1 << 52,
    });

/// The TLS1.3 ciphersuite TLS_AES_128_GCM_SHA256
pub static TLS13_AES_128_GCM_SHA256: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(TLS13_AES_128_GCM_SHA256_INTERNAL);

pub static TLS13_AES_128_GCM_SHA256_INTERNAL: &Tls13CipherSuite = &Tls13CipherSuite {
    common: CipherSuiteCommon {
        suite: CipherSuite::TLS13_AES_128_GCM_SHA256,
        bulk: BulkAlgorithm::Aes128Gcm,
        aead_algorithm: &ring::aead::AES_128_GCM,
    },
    hkdf_algorithm: ring::hkdf::HKDF_SHA256,
    confidentiality_limit: 1 << 23,
    integrity_limit: 1 << 52,
};

/// A TLS 1.3 cipher suite supported by rustls.
pub struct Tls13CipherSuite {
    /// Common cipher suite fields.
    pub common: CipherSuiteCommon,
    pub hkdf_algorithm: ring::hkdf::Algorithm,
    pub confidentiality_limit: u64,
    pub integrity_limit: u64,
}

impl Tls13CipherSuite {
    pub fn derive_encrypter(&self, secret: &hkdf::Prk) -> Box<dyn MessageEncrypter> {
        let key = derive_traffic_key(secret, self.common.aead_algorithm);
        let iv = derive_traffic_iv(secret);

        Box::new(Tls13MessageEncrypter {
            enc_key: aead::LessSafeKey::new(key),
            iv,
        })
    }

    /// Derive a `MessageDecrypter` object from the concerned TLS 1.3
    /// cipher suite.
    pub fn derive_decrypter(&self, secret: &hkdf::Prk) -> Box<dyn MessageDecrypter> {
        let key = derive_traffic_key(secret, self.common.aead_algorithm);
        let iv = derive_traffic_iv(secret);

        Box::new(Tls13MessageDecrypter {
            dec_key: aead::LessSafeKey::new(key),
            iv,
        })
    }

    /// Which hash function to use with this suite.
    pub fn hash_algorithm(&self) -> &'static ring::digest::Algorithm {
        self.hkdf_algorithm.hmac_algorithm().digest_algorithm()
    }

    /// Can a session using suite self resume from suite prev?
    pub fn can_resume_from(&self, prev: &'static Self) -> Option<&'static Self> {
        (prev.hash_algorithm() == self.hash_algorithm()).then(|| prev)
    }
}

impl From<&'static Tls13CipherSuite> for SupportedCipherSuite {
    fn from(s: &'static Tls13CipherSuite) -> Self {
        Self::Tls13(s)
    }
}

impl PartialEq for Tls13CipherSuite {
    fn eq(&self, other: &Self) -> bool {
        self.common.suite == other.common.suite
    }
}

impl fmt::Debug for Tls13CipherSuite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tls13CipherSuite")
            .field("suite", &self.common.suite)
            .field("bulk", &self.common.bulk)
            .finish()
    }
}

struct Tls13MessageEncrypter {
    enc_key: aead::LessSafeKey,
    iv: Iv,
}

struct Tls13MessageDecrypter {
    dec_key: aead::LessSafeKey,
    iv: Iv,
}

fn unpad_tls13(v: &mut Vec<u8>) -> ContentType {
    loop {
        match v.pop() {
            Some(0) => {}
            Some(content_type) => return ContentType::from(content_type),
            None => return ContentType::Unknown(0),
        }
    }
}

fn make_tls13_aad(len: usize) -> ring::aead::Aad<[u8; TLS13_AAD_SIZE]> {
    ring::aead::Aad::from([
        0x17, // ContentType::ApplicationData
        0x3,  // ProtocolVersion (major)
        0x3,  // ProtocolVersion (minor)
        (len >> 8) as u8,
        len as u8,
    ])
}

// https://datatracker.ietf.org/doc/html/rfc8446#section-5.2
const TLS13_AAD_SIZE: usize = 1 + 2 + 2;

impl MessageEncrypter for Tls13MessageEncrypter {
    fn encrypt(&self, msg: BorrowedPlainMessage, seq: u64) -> Result<OpaqueMessage, Error> {
        let total_len = msg.payload.len() + 1 + self.enc_key.algorithm().tag_len();
        let mut payload = Vec::with_capacity(total_len);
        payload.extend_from_slice(msg.payload);
        msg.typ.encode(&mut payload);

        let nonce = make_nonce(&self.iv, seq);
        let aad = make_tls13_aad(total_len);

        self.enc_key
            .seal_in_place_append_tag(nonce, aad, &mut payload)
            .map_err(|_| Error::General("encrypt failed".to_string()))?;

        Ok(OpaqueMessage {
            typ: ContentType::ApplicationData,
            version: ProtocolVersion::TLSv1_2,
            payload: Payload::new(payload),
        })
    }
}

impl MessageDecrypter for Tls13MessageDecrypter {
    fn decrypt(&self, mut msg: OpaqueMessage, seq: u64) -> Result<PlainMessage, Error> {
        let payload = &mut msg.payload.0;
        if payload.len() < self.dec_key.algorithm().tag_len() {
            return Err(Error::DecryptError);
        }

        let nonce = make_nonce(&self.iv, seq);
        let aad = make_tls13_aad(payload.len());
        let plain_len = self
            .dec_key
            .open_in_place(nonce, aad, payload)
            .map_err(|_| Error::DecryptError)?
            .len();

        payload.truncate(plain_len);

        if payload.len() > MAX_FRAGMENT_LEN + 1 {
            return Err(Error::PeerSentOversizedRecord);
        }

        msg.typ = unpad_tls13(payload);
        if msg.typ == ContentType::Unknown(0) {
            let msg = "peer sent bad TLSInnerPlaintext".to_string();
            return Err(Error::PeerMisbehavedError(msg));
        }

        if payload.len() > MAX_FRAGMENT_LEN {
            return Err(Error::PeerSentOversizedRecord);
        }

        msg.version = ProtocolVersion::TLSv1_3;
        Ok(msg.into_plain_message())
    }
}
