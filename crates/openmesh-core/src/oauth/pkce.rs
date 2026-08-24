//! PKCE and OAuth state generation shared by the desktop and CLI hosts.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum PkceError {
    #[error("secure random source failed")]
    Random(#[source] rand::Error),
    #[error("PKCE verifier must contain 43 to 128 RFC 7636 characters")]
    InvalidVerifier,
}

/// A PKCE verifier/challenge pair. The verifier is intentionally redacted from
/// debug output because it is a short-lived OAuth secret.
#[derive(Clone, PartialEq, Eq)]
pub struct PkceCodes {
    pub verifier: String,
    pub challenge: String,
}

impl fmt::Debug for PkceCodes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PkceCodes")
            .field("verifier", &"<redacted>")
            .field("challenge", &self.challenge)
            .finish()
    }
}

impl PkceCodes {
    pub fn generate() -> Result<Self, PkceError> {
        let mut bytes = [0_u8; 32];
        OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(PkceError::Random)?;
        let verifier = URL_SAFE_NO_PAD.encode(bytes);
        Self::from_verifier(verifier)
    }

    pub fn from_verifier(verifier: impl Into<String>) -> Result<Self, PkceError> {
        let verifier = verifier.into();
        if !(43..=128).contains(&verifier.len())
            || verifier
                .bytes()
                .any(|byte| !matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'))
        {
            return Err(PkceError::InvalidVerifier);
        }
        let digest = Sha256::digest(verifier.as_bytes());
        Ok(Self {
            verifier,
            challenge: URL_SAFE_NO_PAD.encode(digest),
        })
    }
}

pub fn generate_state() -> Result<String, PkceError> {
    let mut bytes = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(PkceError::Random)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_rfc7636_compatible() {
        let codes = PkceCodes::generate().expect("PKCE");
        assert!((43..=128).contains(&codes.verifier.len()));
        assert_eq!(codes.challenge.len(), 43);
        assert!(!format!("{codes:?}").contains(&codes.verifier));
    }

    #[test]
    fn known_verifier_has_expected_s256_challenge() {
        let codes = PkceCodes::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")
            .expect("known verifier");
        assert_eq!(
            codes.challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn invalid_verifier_is_rejected() {
        assert!(matches!(
            PkceCodes::from_verifier("too-short"),
            Err(PkceError::InvalidVerifier)
        ));
    }
}
