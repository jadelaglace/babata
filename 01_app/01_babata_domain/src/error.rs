use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("{field} must not be empty")]
    Empty { field: &'static str },
    #[error("invalid {field}: {value}")]
    Invalid { field: &'static str, value: String },
    #[error("logical paths must be relative and stay under the data root")]
    UnsafeLogicalPath,
    #[error("metadata must be a JSON object")]
    MetadataMustBeObject,
}
