#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretReference {
    pub environment_key: String,
}

impl SecretReference {
    pub fn new(environment_key: impl Into<String>) -> Self {
        Self {
            environment_key: environment_key.into(),
        }
    }
}
