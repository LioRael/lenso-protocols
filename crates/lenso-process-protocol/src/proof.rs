use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::{HandshakeIdentity, HandshakeParams, ProtocolError, decode_base64url_32};

const HOST_PROOF_DOMAIN: &[u8] = b"lenso-process-host-v1";
const CHILD_PROOF_DOMAIN: &[u8] = b"lenso-process-child-v1";
const MAX_SAFE_JSON_INTEGER: u64 = 9_007_199_254_740_991;

/// Returns RFC 8785-compatible canonical bytes for the restricted proof profile.
pub fn canonicalize_proof_value(value: &Value) -> Result<Vec<u8>, ProtocolError> {
    let mut output = Vec::new();
    write_canonical(value, &mut output)?;
    Ok(output)
}

/// Computes the raw SHA-256 digest bound by host and child handshake proofs.
pub fn handshake_params_digest(params: &HandshakeParams) -> Result<[u8; 32], ProtocolError> {
    #[derive(Serialize)]
    struct ProofPayload<'a> {
        identity: &'a HandshakeIdentity,
        host_nonce: &'a str,
    }

    params.identity.validate()?;
    decode_base64url_32("host_nonce", &params.host_nonce)?;
    let value = serde_json::to_value(ProofPayload {
        identity: &params.identity,
        host_nonce: &params.host_nonce,
    })
    .map_err(|error| ProtocolError::new(format!("cannot encode handshake proof: {error}")))?;
    let canonical = canonicalize_proof_value(&value)?;
    Ok(Sha256::digest(canonical).into())
}

/// Returns the exact bytes authenticated by `host_proof`.
#[must_use]
pub fn host_proof_message(handshake_digest: &[u8; 32]) -> Vec<u8> {
    proof_message(HOST_PROOF_DOMAIN, handshake_digest, None)
}

/// Returns the exact bytes authenticated by `child_proof`.
pub fn child_proof_message(
    handshake_digest: &[u8; 32],
    session: &str,
) -> Result<Vec<u8>, ProtocolError> {
    let session = decode_base64url_32("session", session)?;
    Ok(proof_message(
        CHILD_PROOF_DOMAIN,
        handshake_digest,
        Some(&session),
    ))
}

fn proof_message(domain: &[u8], digest: &[u8; 32], suffix: Option<&[u8; 32]>) -> Vec<u8> {
    let mut message = Vec::with_capacity(domain.len() + 1 + 32 + suffix.map_or(0, |_| 32));
    message.extend_from_slice(domain);
    message.push(0);
    message.extend_from_slice(digest);
    if let Some(suffix) = suffix {
        message.extend_from_slice(suffix);
    }
    message
}

fn write_canonical(value: &Value, output: &mut Vec<u8>) -> Result<(), ProtocolError> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(true) => output.extend_from_slice(b"true"),
        Value::Bool(false) => output.extend_from_slice(b"false"),
        Value::String(value) => output.extend_from_slice(
            serde_json::to_string(value)
                .map_err(|error| ProtocolError::new(error.to_string()))?
                .as_bytes(),
        ),
        Value::Number(number) => {
            let encoded = if let Some(value) = number.as_u64() {
                if value > MAX_SAFE_JSON_INTEGER {
                    return Err(ProtocolError::new(
                        "proof JSON integer exceeds the portable safe range",
                    ));
                }
                value.to_string()
            } else if let Some(value) = number.as_i64() {
                if value.unsigned_abs() > MAX_SAFE_JSON_INTEGER {
                    return Err(ProtocolError::new(
                        "proof JSON integer exceeds the portable safe range",
                    ));
                }
                value.to_string()
            } else {
                return Err(ProtocolError::new(
                    "floating-point values are forbidden in proof JSON",
                ));
            };
            output.extend_from_slice(encoded.as_bytes());
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_canonical(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.encode_utf16().cmp(right.encode_utf16()));
            output.push(b'{');
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend_from_slice(
                    serde_json::to_string(key)
                        .map_err(|error| ProtocolError::new(error.to_string()))?
                        .as_bytes(),
                );
                output.push(b':');
                write_canonical(value, output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}
