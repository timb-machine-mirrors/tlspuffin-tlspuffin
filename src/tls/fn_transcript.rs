use crate::agent::AgentName;
use crate::tls::error::FnError;
use crate::trace::AgentClaimer;
use rustls::hash_hs::{HandshakeHash, HandshakeHashBuffer};
use rustls::tls13;
use security_claims::{Claim, ClaimType};

fn into_transcript(
    claim: Option<&(AgentName, Claim)>,
    typ: ClaimType,
) -> Result<HandshakeHash, FnError> {
    if let Some((_, claim)) = claim {
        let algorithm = tls13::TLS13_AES_128_GCM_SHA256.hash_algorithm(); // TODO update
        let claim_transcript = &claim.transcript.data[..claim.transcript.length as usize];
        let hash = HandshakeHash::new_override(Vec::from(
            claim_transcript,
        ), algorithm);
        return Ok(hash);
    }

    Err(FnError::Unknown(format!(
        "Failed to find {:?} transcript",
        typ
    )))
}

fn find_transcript(claims: &AgentClaimer, typ: ClaimType) -> Result<HandshakeHash, FnError> {
    let claim = claims.find_last_claim(typ);
    into_transcript(claim, typ)
}

pub fn fn_server_hello_transcript(claims: &AgentClaimer) -> Result<HandshakeHash, FnError> {
    find_transcript(claims, ClaimType::CLAIM_TRANSCRIPT_CH_SH)
}

pub fn fn_server_finished_transcript(claims: &AgentClaimer) -> Result<HandshakeHash, FnError> {
    find_transcript(claims, ClaimType::CLAIM_TRANSCRIPT_CH_SERVER_FIN)
}
pub fn fn_client_finished_transcript(claims: &AgentClaimer) -> Result<HandshakeHash, FnError> {
    find_transcript(claims, ClaimType::CLAIM_TRANSCRIPT_CH_CLIENT_FIN)
}
