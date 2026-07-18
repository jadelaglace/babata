use crate::ApiError;

pub fn verify_token(expected: &str, supplied: Option<&str>) -> Result<(), ApiError> {
    let supplied = supplied.unwrap_or_default();
    if expected.is_empty()
        || expected.len() != supplied.len()
        || expected
            .bytes()
            .zip(supplied.bytes())
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            != 0
    {
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
