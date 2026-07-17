use crate::ApiError;

pub fn verify_token(expected: &str, supplied: Option<&str>) -> Result<(), ApiError> {
    if expected.is_empty() || supplied != Some(expected) {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_or_wrong_tokens_are_rejected() {
        assert!(super::verify_token("install-token", None).is_err());
        assert!(super::verify_token("install-token", Some("wrong")).is_err());
        assert!(super::verify_token("install-token", Some("install-token")).is_ok());
    }
}
