use crate::DomainError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256 as Sha256Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogicalPath(String);

impl LogicalPath {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DomainError> {
        let value = value.as_ref().replace('\\', "/");
        if value.is_empty()
            || value.starts_with('/')
            || value.contains(':')
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(DomainError::UnsafeLogicalPath);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256(String);

impl Sha256 {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DomainError> {
        let value = value.as_ref();
        if value.len() != 64
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
            || value.bytes().any(|byte| byte.is_ascii_uppercase())
        {
            return Err(DomainError::Invalid {
                field: "sha256",
                value: value.to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256Hasher::digest(bytes)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Sha256 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UtcTimestamp(String);

impl UtcTimestamp {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DomainError> {
        let value = value.as_ref();
        if value.trim().is_empty() {
            return Err(DomainError::Empty { field: "timestamp" });
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Metadata(serde_json::Map<String, serde_json::Value>);

impl Metadata {
    pub fn empty() -> Self {
        Self(serde_json::Map::new())
    }
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match serde_json::from_str(value).map_err(|_| DomainError::MetadataMustBeObject)? {
            serde_json::Value::Object(object) => Ok(Self(object)),
            _ => Err(DomainError::MetadataMustBeObject),
        }
    }
    pub fn to_json(&self) -> String {
        serde_json::Value::Object(self.0.clone()).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextPayload(String);

impl TextPayload {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Empty { field: "text" });
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn hash(&self) -> Sha256 {
        Sha256::of_bytes(self.0.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_reject_escape_attempts() {
        assert!(LogicalPath::parse("../raw.sqlite").is_err());
        assert!(LogicalPath::parse("C:/data/raw.sqlite").is_err());
    }

    #[test]
    fn hashes_must_be_lowercase_sha256() {
        assert!(Sha256::parse("A".repeat(64)).is_err());
        assert_eq!(Sha256::of_bytes(b"babata").as_str().len(), 64);
    }

    #[test]
    fn metadata_requires_object() {
        assert!(Metadata::parse("[]").is_err());
        assert_eq!(Metadata::parse("{}").unwrap().to_json(), "{}");
    }
}
