//! Runtime-neutral wire primitives and serde support for generated Lenso contracts.

use std::collections::BTreeMap;

use ::serde::{Serialize, de::DeserializeOwned};
use base64::{Engine as _, engine::general_purpose::STANDARD};

/// Signed 64-bit integer encoded as a decimal string on the wire.
pub type Int64 = String;
/// Unsigned 64-bit integer encoded as a decimal string on the wire.
pub type Uint64 = String;
/// RFC 3339 timestamp encoded as a string on the wire.
pub type Timestamp = String;
/// ISO 8601 duration encoded as a string on the wire.
pub type Duration = String;
/// Distinguishes a missing field from an explicit `null` value.
pub type OptionalValue<T> = Option<Option<T>>;

/// Shared native bytes encoded as canonical padded Base64 on the wire.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Bytes(bytes::Bytes);

impl Bytes {
    /// Wraps an owned or shared byte buffer without copying when possible.
    pub fn new(value: impl Into<bytes::Bytes>) -> Self {
        Self(value.into())
    }

    /// Returns the byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Returns the shared byte buffer without copying.
    #[must_use]
    pub fn into_shared(self) -> bytes::Bytes {
        self.0
    }

    /// Returns an owned vector containing the bytes.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into())
    }
}

impl From<&[u8]> for Bytes {
    fn from(value: &[u8]) -> Self {
        Self(bytes::Bytes::copy_from_slice(value))
    }
}

impl From<bytes::Bytes> for Bytes {
    fn from(value: bytes::Bytes) -> Self {
        Self(value)
    }
}

impl From<Bytes> for bytes::Bytes {
    fn from(value: Bytes) -> Self {
        value.0
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(value: Bytes) -> Self {
        value.into_vec()
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::ops::Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(self.as_slice()))
    }
}

impl<'de> ::serde::Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let encoded = <String as ::serde::Deserialize>::deserialize(deserializer)?;
        STANDARD
            .decode(encoded)
            .map(Self::from)
            .map_err(|_| ::serde::de::Error::custom("bytes must be canonical padded base64"))
    }
}

/// A forward-compatible Capability-defined error unknown to this binding version.
#[derive(Clone, Debug, PartialEq, Serialize, ::serde::Deserialize)]
pub struct UnknownDomainError {
    /// Stable Domain Error code.
    pub code: String,
    /// Optional opaque error payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Additional fields preserved for forward compatibility.
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Encodes a typed value after enforcing the portable JSON number profile.
pub fn encode_portable_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    validate_portable_json_value(&value).map_err(portable_json_error)?;
    serde_json::to_string(&value)
}

/// Decodes a typed value after enforcing the portable JSON number profile.
pub fn decode_portable_json<T: DeserializeOwned>(wire: &str) -> Result<T, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(wire)?;
    validate_portable_json_value(&value).map_err(portable_json_error)?;
    serde_json::from_value(value)
}

/// Validates recursively that ordinary JSON numbers are portable across runtimes.
pub fn validate_portable_json_value(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Number(number) => {
            let safe = number.as_i64().is_some_and(|value| {
                (-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&value)
            }) || number
                .as_u64()
                .is_some_and(|value| value <= 9_007_199_254_740_991)
                || (number.is_f64()
                    && number.as_f64().is_some_and(|value| {
                        value.is_finite()
                            && (value.abs() <= 9_007_199_254_740_991.0 || value.fract() != 0.0)
                    }));
            if !safe {
                return Err("wire JSON contains an unsafe number".to_owned());
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_portable_json_value(value)?;
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                validate_portable_json_value(value)?;
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {}
    }
    Ok(())
}

fn portable_json_error(detail: String) -> serde_json::Error {
    serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, detail))
}

/// Serde helpers used by generated field attributes.
pub mod serde {
    /// Deserializes a required field while allowing generated structs to reject omission.
    pub fn deserialize_required<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: ::serde::Deserializer<'de>,
        T: ::serde::Deserialize<'de>,
    {
        <T as ::serde::Deserialize>::deserialize(deserializer)
    }

    /// Preserves the distinction between a missing field and an explicit `null`.
    #[allow(clippy::option_option)]
    pub fn deserialize_optional_value<'de, D, T>(
        deserializer: D,
    ) -> Result<Option<Option<T>>, D::Error>
    where
        D: ::serde::Deserializer<'de>,
        T: ::serde::Deserialize<'de>,
    {
        Ok(Some(<Option<T> as ::serde::Deserialize>::deserialize(
            deserializer,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_preserve_canonical_padded_base64_wire_behavior() {
        for length in 0..=258 {
            let value = Bytes::from(
                (0..length)
                    .map(|index| u8::try_from(index % 256).unwrap())
                    .collect::<Vec<_>>(),
            );
            let wire = serde_json::to_string(&value).unwrap();
            assert_eq!(serde_json::from_str::<Bytes>(&wire).unwrap(), value);
        }
        for wire in [r#""not base64""#, r#""AQI""#, r#""AQJ=""#] {
            assert!(serde_json::from_str::<Bytes>(wire).is_err());
        }
    }

    #[test]
    fn shared_bytes_round_trip_without_copying() {
        let source = bytes::Bytes::from_static(b"shared");
        let contract = Bytes::from(source.clone());
        assert_eq!(contract.as_slice().as_ptr(), source.as_ptr());
        let restored = contract.into_shared();
        assert_eq!(restored.as_ptr(), source.as_ptr());
    }

    #[test]
    fn portable_json_rejects_unsafe_integer_values() {
        let error = decode_portable_json::<serde_json::Value>("9007199254740992").unwrap_err();
        assert!(error.to_string().contains("unsafe number"));
    }
}
