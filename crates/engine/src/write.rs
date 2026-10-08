use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Replace `path` with `data` through a sibling temporary file and a rename,
/// so a crash never leaves a truncated source behind. The original's
/// permissions are kept; a symlink is written through to its target.
pub fn atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let permissions = fs::metadata(&target).map(|m| m.permissions()).ok();
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut tmp = tempfile::Builder::new().prefix(&format!(".{name}.")).suffix(".tmp").tempfile_in(dir)?;

    tmp.write_all(data)?;

    if let Some(permissions) = permissions {
        tmp.as_file().set_permissions(permissions)?;
    }

    tmp.persist(&target).map_err(|e| e.error)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::atomic;
    use std::fs;

    #[test]
    fn replaces_content_and_keeps_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ts");

        fs::write(&path, "old").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
        }

        atomic(&path, b"new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o750);
        }
    }
}
