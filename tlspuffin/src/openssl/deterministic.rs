use std::os::raw::c_int;

use log::warn;

#[cfg(feature = "deterministic")]
extern "C" {
    fn make_openssl_deterministic();
    fn RAND_seed(buf: *mut u8, num: c_int);
}

#[cfg(feature = "deterministic")]
pub fn determinism_set_reseed_openssl() {
    println!("Making OpenSSL fully deterministic: reset rand and reseed to a constant...");
    unsafe {
        make_openssl_deterministic();
    }
    determinism_reseed_openssl();
}

#[cfg(feature = "deterministic")]
pub fn determinism_reseed_openssl() {
    println!(" - Reseed RAND for OpenSSL");
    unsafe {
        let mut seed: [u8; 4] = 42u32.to_le().to_ne_bytes();
        let buf = seed.as_mut_ptr();
        RAND_seed(buf, 4);
    }
}

#[cfg(test)]
mod tests {
    use crate::openssl::deterministic::determinism_set_reseed_openssl;
    use openssl::rand::rand_bytes;

    #[test]
    #[cfg(feature = "openssl111-binding")]
    fn test_openssl_no_randomness_simple() {
        determinism_set_reseed_openssl();
        let mut buf1 = [0; 2];
        rand_bytes(&mut buf1).unwrap();
        assert_eq!(buf1, [179, 16]);
    }
}
