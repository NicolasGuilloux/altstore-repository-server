use sha2::{Digest, Sha256};

/// Generate a deterministic token for an IPA file
/// The token is generated from a hash of app_name, filename, and an optional secret
/// This ensures tokens remain consistent across server restarts
pub fn generate_download_token(app_name: &str, filename: &str, secret: Option<&str>) -> String {
    let mut hasher = Sha256::new();

    // Hash the inputs
    hasher.update(app_name.as_bytes());
    hasher.update(b"|"); // separator
    hasher.update(filename.as_bytes());

    // Add secret if provided for additional security
    if let Some(secret_key) = secret {
        hasher.update(b"|");
        hasher.update(secret_key.as_bytes());
    }

    let result = hasher.finalize();

    // Use first 16 bytes (128 bits) for a shorter token
    // Encode as base64url (URL-safe, no padding)
    base64_url_encode(&result[..16])
}

/// Encode bytes as base64url (URL-safe base64 without padding)
fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_deterministic() {
        let token1 = generate_download_token("YourApp", "app_1.0.ipa", None);
        let token2 = generate_download_token("YourApp", "app_1.0.ipa", None);
        assert_eq!(token1, token2, "Tokens should be deterministic");
    }

    #[test]
    fn test_generate_token_different_inputs() {
        let token1 = generate_download_token("YourApp", "app_1.0.ipa", None);
        let token2 = generate_download_token("YourApp", "app_2.0.ipa", None);
        assert_ne!(
            token1, token2,
            "Different filenames should produce different tokens"
        );

        let token3 = generate_download_token("OtherApp", "app_1.0.ipa", None);
        assert_ne!(
            token1, token3,
            "Different app names should produce different tokens"
        );
    }

    #[test]
    fn test_generate_token_with_secret() {
        let token1 = generate_download_token("YourApp", "app_1.0.ipa", Some("secret123"));
        let token2 = generate_download_token("YourApp", "app_1.0.ipa", Some("secret456"));
        assert_ne!(
            token1, token2,
            "Different secrets should produce different tokens"
        );

        let token3 = generate_download_token("YourApp", "app_1.0.ipa", None);
        assert_ne!(
            token1, token3,
            "Token with secret should differ from token without"
        );
    }

    #[test]
    fn test_token_format() {
        let token = generate_download_token("YourApp", "app_1.0.ipa", None);
        // Should be URL-safe base64 (no padding, no special chars except - and _)
        assert!(!token.contains('='), "Should not contain padding");
        assert!(!token.contains('+'), "Should not contain +");
        assert!(!token.contains('/'), "Should not contain /");
        // 16 bytes -> 128 bits -> ~22 base64 characters
        assert!(
            token.len() >= 20 && token.len() <= 24,
            "Token length should be around 22 chars"
        );
    }
}
