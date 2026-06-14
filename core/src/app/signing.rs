use anyhow::{anyhow, Result};
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn verify_package_signature(signature: &str, message: &str) -> Result<()> {
    let (key, signature) = parse_ed25519_signature(signature)?;

    key.verify(message.as_bytes(), &signature)
        .map_err(|e| anyhow!("signature verification failed: {}", e))
}

pub fn verify_package_signature_for_source(
    signature: &str,
    message: &str,
    source_authority: &str,
    source_public_keys: &[String],
) -> Result<()> {
    let trusted_public_key = signature_public_key(signature)?;
    if !source_public_keys.is_empty()
        && !source_public_keys
            .iter()
            .any(|candidate| candidate == &trusted_public_key)
    {
        let authority = if source_authority.trim().is_empty() {
            "<unspecified>"
        } else {
            source_authority
        };
        return Err(anyhow!(
            "signature public key is not trusted by source authority '{}'",
            authority
        ));
    }

    verify_package_signature(signature, message)
}

fn signature_public_key(signature: &str) -> Result<String> {
    let (key, _) = parse_ed25519_signature(signature)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(key.as_bytes()))
}

fn parse_ed25519_signature(signature: &str) -> Result<(VerifyingKey, Signature)> {
    let parts: Vec<&str> = signature.split(':').collect();
    if parts.len() != 3 || parts[0] != "ed25519" {
        return Err(anyhow!("unsupported signature format"));
    }

    let public_key = base64::engine::general_purpose::STANDARD
        .decode(parts[1])
        .map_err(|e| anyhow!("invalid signature public key: {}", e))?;
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(parts[2])
        .map_err(|e| anyhow!("invalid signature payload: {}", e))?;

    let key = VerifyingKey::from_bytes(
        &public_key
            .try_into()
            .map_err(|_| anyhow!("invalid verifying key length"))?,
    )
    .map_err(|e| anyhow!("invalid verifying key: {}", e))?;

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| anyhow!("invalid signature bytes: {}", e))?;

    Ok((key, signature))
}

pub fn signature_message(name: &str, version: &str, sha512: &str, source: &str) -> String {
    format!("{}:{}:{}:{}", name, version, sha512, source)
}

pub fn sign_package_message(signing_key: &SigningKey, message: &str) -> String {
    let public_key =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let signature = base64::engine::general_purpose::STANDARD
        .encode(signing_key.sign(message.as_bytes()).to_bytes());

    format!("ed25519:{}:{}", public_key, signature)
}

#[cfg(test)]
mod tests {
    use super::{
        sign_package_message, signature_message, verify_package_signature,
        verify_package_signature_for_source,
    };
    use base64::Engine;
    use ed25519_dalek::SigningKey;

    #[test]
    fn builtin_prefix_is_not_crypto_signature() {
        let result = verify_package_signature("builtin:official", "payload");
        assert!(result.is_err());
    }

    #[test]
    fn signature_message_is_stable() {
        let msg = signature_message("pkg", "1.0.0", "abc", "local://pkg");
        assert_eq!(msg, "pkg:1.0.0:abc:local://pkg");
    }

    #[test]
    fn sign_and_verify_round_trip_matches_core_format() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let message = signature_message("pkg", "1.0.0", "abc", "local://pkg");

        let signature = sign_package_message(&signing_key, &message);

        assert!(signature.starts_with("ed25519:"));
        verify_package_signature(&signature, &message).expect("signature should verify");
    }

    #[test]
    fn verify_package_signature_rejects_tampered_ed25519_payload() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let message = signature_message("pkg", "1.0.0", "abc", "local://pkg");
        let signature = sign_package_message(&signing_key, &message);
        let mut parts: Vec<String> = signature.split(':').map(str::to_string).collect();
        let mut signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&parts[2])
            .expect("signature bytes decode");

        signature_bytes[0] ^= 0x01;
        parts[2] = base64::engine::general_purpose::STANDARD.encode(signature_bytes);

        let tampered_signature = parts.join(":");
        let result = verify_package_signature(&tampered_signature, &message);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("signature verification failed"));
    }

    #[test]
    fn verify_package_signature_rejects_mismatched_message_digest() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let signed_message = signature_message("pkg", "1.0.0", "abc", "local://pkg");
        let mismatched_message = signature_message("pkg", "1.0.0", "def", "local://pkg");
        let signature = sign_package_message(&signing_key, &signed_message);

        let result = verify_package_signature(&signature, &mismatched_message);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("signature verification failed"));
    }

    #[test]
    fn verify_package_signature_for_source_rejects_mismatched_message_digest() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let signed_message = signature_message("pkg", "1.0.0", "abc", "local://pkg");
        let mismatched_message = signature_message("pkg", "1.0.0", "def", "local://pkg");
        let signature = sign_package_message(&signing_key, &signed_message);
        let allowed_key = signature
            .split(':')
            .nth(1)
            .expect("public key segment exists")
            .to_string();

        let error = verify_package_signature_for_source(
            &signature,
            &mismatched_message,
            "test-authority",
            &[allowed_key],
        )
        .expect_err("signature should be rejected when message digest does not match");

        assert!(error.to_string().contains("signature verification failed"));
    }

    #[test]
    fn verify_package_signature_for_source_accepts_declared_public_key() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let message = signature_message("pkg", "1.0.0", "abc", "local://pkg");
        let signature = sign_package_message(&signing_key, &message);

        let allowed_key = signature
            .split(':')
            .nth(1)
            .expect("public key segment exists")
            .to_string();

        verify_package_signature_for_source(&signature, &message, "test-authority", &[allowed_key])
            .expect("signature should verify against declared authority key");
    }

    #[test]
    fn verify_package_signature_for_source_rejects_undeclared_public_key() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let other_signing_key = SigningKey::from_bytes(&[8; 32]);
        let message = signature_message("pkg", "1.0.0", "abc", "local://pkg");
        let signature = sign_package_message(&signing_key, &message);
        let other_signature = sign_package_message(&other_signing_key, &message);
        let other_public_key = other_signature
            .split(':')
            .nth(1)
            .expect("public key segment exists")
            .to_string();

        let error = verify_package_signature_for_source(
            &signature,
            &message,
            "test-authority",
            &[other_public_key],
        )
        .expect_err("signature should be rejected when authority key does not match");

        assert!(error
            .to_string()
            .contains("not trusted by source authority 'test-authority'"));
    }
}
