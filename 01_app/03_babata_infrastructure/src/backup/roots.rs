use std::path::{Path, PathBuf};

use babata_application::ApplicationError;

/// Resolve existing ancestors before comparing roots (including junction aliases).
pub(super) fn resolved_path(path: &Path) -> Result<PathBuf, ApplicationError> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(ApplicationError::Integrity(
            "backup roots must be absolute paths without parent traversal".to_owned(),
        ));
    }
    let mut ancestor = path;
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        suffix.push(
            ancestor
                .file_name()
                .ok_or_else(|| {
                    ApplicationError::Integrity("root has no existing ancestor".to_owned())
                })?
                .to_os_string(),
        );
        ancestor = ancestor.parent().ok_or_else(|| {
            ApplicationError::Integrity("root has no existing ancestor".to_owned())
        })?;
    }
    let mut resolved = std::fs::canonicalize(ancestor)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    for name in suffix.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

fn path_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace(char::from(92), "/");
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

pub(super) fn overlaps(left: &Path, right: &Path) -> bool {
    let left = path_key(left);
    let right = path_key(right);
    left == right
        || left.starts_with(&format!("{}/", right.trim_end_matches('/')))
        || right.starts_with(&format!("{}/", left.trim_end_matches('/')))
}

pub(super) fn validate_roots(roots: &[PathBuf]) -> Result<(), ApplicationError> {
    let resolved = roots
        .iter()
        .map(|path| resolved_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, path) in resolved.iter().enumerate() {
        if path
            .ancestors()
            .any(|ancestor| ancestor.join(".git").exists())
        {
            return Err(ApplicationError::Integrity(
                "runtime roots must stay outside Git".to_owned(),
            ));
        }
        if resolved[..index].iter().any(|other| overlaps(path, other)) {
            return Err(ApplicationError::Integrity(
                "runtime and backup roots must be disjoint".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(super) fn is_link(path: &Path) -> Result<bool, ApplicationError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(metadata.file_attributes() & 0x400 != 0)
    }
    #[cfg(not(windows))]
    {
        Ok(metadata.file_type().is_symlink())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_nested_equal_git_and_traversal_roots() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("data");
        assert!(validate_roots(&[root.clone(), root.join("child")]).is_err());
        assert!(validate_roots(&[root.clone(), root]).is_err());
        assert!(resolved_path(&temp.path().join("x/../y")).is_err());
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        assert!(validate_roots(&[temp.path().join("data")]).is_err());
    }

    #[test]
    fn distinct_sibling_roots_are_valid() {
        let temp = tempdir().unwrap();
        assert!(validate_roots(&[temp.path().join("data"), temp.path().join("data2")]).is_ok());
    }
}
