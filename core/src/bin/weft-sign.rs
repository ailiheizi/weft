use anyhow::{bail, Context, Result};
use ed25519_dalek::SigningKey;
use weft_core::app::{sign_package_message, signature_message};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|arg| arg.as_str()) == Some("digest") {
        if args.len() != 2 {
            bail!("usage: weft-sign digest <source>");
        }

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let digest = weft_core::api::generations::package_digest(&repo_root, &args[1]);
        println!("digest={}", digest);
        return Ok(());
    }

    if args.len() != 5 {
        bail!("usage: weft-sign <private-key-hex-32-bytes> <name> <version> <sha512> <source>");
    }

    let signing_key = parse_signing_key(&args[0])?;
    let message = signature_message(&args[1], &args[2], &args[3], &args[4]);
    let signature = sign_package_message(&signing_key, &message);

    println!("message={}", message);
    println!("signature={}", signature);
    Ok(())
}

fn parse_signing_key(hex_key: &str) -> Result<SigningKey> {
    let key_bytes = decode_hex(hex_key).context("private key must be valid hex")?;
    let secret_key: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("private key must decode to exactly 32 bytes"))?;

    Ok(SigningKey::from_bytes(&secret_key))
}

fn decode_hex(hex: &str) -> Result<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        bail!("hex input must have even length");
    }

    (0..hex.len())
        .step_by(2)
        .map(|idx| u8::from_str_radix(&hex[idx..idx + 2], 16).context("invalid hex digit"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_signing_key;

    #[test]
    fn parses_32_byte_private_key_hex() {
        let key =
            parse_signing_key("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .expect("expected valid private key");

        assert_eq!(key.to_bytes()[0], 0);
        assert_eq!(key.to_bytes()[31], 31);
    }
}
