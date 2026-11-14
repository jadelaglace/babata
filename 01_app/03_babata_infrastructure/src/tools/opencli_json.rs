use babata_application::ApplicationError;
use serde_json::Value;

/// Normalize the documented legacy array and v2 ok/data response shapes.
pub fn decode_output(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<Value, ApplicationError> {
    let value = serde_json::from_slice::<Value>(stdout)
        .or_else(|_| serde_json::from_slice::<Value>(stderr))
        .map_err(|_| {
            ApplicationError::Integrity("OpenCLI returned a non-JSON response".to_owned())
        })?;
    if !success || value.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(ApplicationError::Provider(
            "OpenCLI command failed; inspect private acquisition evidence".to_owned(),
        ));
    }
    if value.is_array() {
        return Ok(value);
    }
    if value.get("ok").and_then(Value::as_bool) == Some(true)
        && let Some(data) = value.get("data").filter(|data| data.is_array())
    {
        return Ok(data.clone());
    }
    Err(ApplicationError::Integrity(
        "unsupported OpenCLI JSON envelope".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_and_v2_arrays_are_equivalent() {
        let old = decode_output(true, br#"[{"Id":"1"}]"#, b"").unwrap();
        let new = decode_output(true, br#"{"ok":true,"data":[{"Id":"1"}]}"#, b"").unwrap();
        assert_eq!(old, new);
    }
    #[test]
    fn failures_and_malformed_envelopes_never_become_success() {
        for bytes in [
            br#"{"ok":false,"data":[]}"#.as_slice(),
            br#"{"ok":true}"#,
            br#"{"ok":"true","data":[]}"#,
            br#"{"ok":true,"data":null}"#,
            br#"{"data":[]}"#,
            b"not-json",
        ] {
            assert!(decode_output(true, bytes, b"").is_err());
        }
        assert!(decode_output(false, b"[]", b"").is_err());
        assert!(decode_output(false, br#"{"ok":false}"#, b"").is_err());
        assert!(decode_output(false, b"", br#"{"ok":false}"#).is_err());
    }
}
