# WEFT package signing demo

`weft-sign` is a minimal repo-local helper that emits the same signature shape verified by core:

`ed25519:<public-key-base64>:<signature-base64>`

The signed message is exactly:

`name:version:sha512:source`

## Run the demo

From the repository root:

```powershell
cargo run -p weft-core --bin weft-sign -- 000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f demo 1.0.0 abcdef packages/demo
```

Expected output shape:

```text
message=demo:1.0.0:abcdef:packages/demo
signature=ed25519:<public-key-base64>:<signature-base64>
```

This is intentionally minimal: it accepts a 32-byte Ed25519 private key in hex, builds the exact core message format, and prints the verifiable signature string.

## Focused verification

```powershell
cargo test -p weft-core signing
cargo test -p weft-core weft_sign
```

The `core/src/app/signing.rs` round-trip test proves the produced signature verifies with the same `verify_package_signature(...)` path used by core policy checks.
