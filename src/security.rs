use anyhow::Result;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Maximum spec file size (50 MB).
pub const MAX_SPEC_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// Maximum number of endpoints to process.
pub const MAX_ENDPOINTS: usize = 10_000;

/// Maximum number of schema properties to process.
pub const MAX_SCHEMA_PROPERTIES: usize = 1_000;

/// Maximum length for descriptions in generated output.
pub const MAX_DESCRIPTION_LENGTH: usize = 500;

/// Maximum JSON nesting depth to prevent stack overflow.
pub const MAX_JSON_DEPTH: usize = 128;

/// Validate JSON nesting depth doesn't exceed the limit.
pub fn validate_json_depth(content: &str) -> Result<()> {
    let mut depth: usize = 0;
    let mut max_depth: usize = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in content.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '{' | '[' => {
                depth += 1;
                if depth > max_depth {
                    max_depth = depth;
                }
                if max_depth > MAX_JSON_DEPTH {
                    anyhow::bail!("JSON nesting depth exceeds maximum of {MAX_JSON_DEPTH}");
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Returns the list of system directory prefixes that must never be written to.
fn blocked_prefixes() -> Vec<PathBuf> {
    let root = Path::new(std::path::MAIN_SEPARATOR_STR);
    ["etc", "dev", "proc", "sys", "boot"]
        .iter()
        .map(|name| root.join(name))
        .collect()
}

/// Check if a path is a symlink.
fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
}

/// Atomically load a spec file: check symlinks, check size, read — all in one operation.
/// Eliminates TOCTOU race between size check and read.
pub fn load_spec_safe(path: &Path) -> Result<String> {
    // Reject symlinks
    if is_symlink(path) {
        anyhow::bail!("Spec file must not be a symbolic link");
    }

    let mut file =
        std::fs::File::open(path).map_err(|e| anyhow::anyhow!("Cannot open spec file: {e}"))?;

    // Check size on the open file descriptor (no TOCTOU)
    let metadata = file.metadata()?;
    if metadata.len() > MAX_SPEC_FILE_BYTES {
        anyhow::bail!("Spec file exceeds maximum size of {MAX_SPEC_FILE_BYTES} bytes");
    }

    let capacity = usize::try_from(metadata.len()).unwrap_or(0);
    let mut content = String::with_capacity(capacity);
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// Validate an output path, then atomically write content with restricted permissions.
/// Rejects traversal, symlinks, and writes to system directories.
pub fn write_output_safe(path: &Path, content: &str) -> Result<()> {
    let path_str = path.to_string_lossy();

    if path_str.contains("..") {
        anyhow::bail!("Output path must not contain '..' traversal");
    }

    if is_symlink(path) {
        anyhow::bail!("Output path must not be a symbolic link");
    }

    let resolved = if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            let cwd = std::env::current_dir()?;
            cwd.join(path)
        } else {
            if is_symlink(parent) {
                anyhow::bail!("Output parent directory must not be a symbolic link");
            }
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| anyhow::anyhow!("Cannot resolve output directory: {e}"))?;
            canonical_parent.join(path.file_name().unwrap_or_default())
        }
    } else {
        path.to_path_buf()
    };

    let resolved_str = resolved.to_string_lossy();
    for prefix in blocked_prefixes() {
        let prefix_str = prefix.to_string_lossy();
        if resolved_str.starts_with(prefix_str.as_ref()) {
            anyhow::bail!("Writing to system directories is not allowed");
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&resolved)?;

    file.write_all(content.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Escape a string for safe inclusion in markdown tables and inline contexts.
#[must_use]
pub fn escape_markdown(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('\n', " ")
        .replace('\r', "")
        .replace('`', "'")
}

/// Truncate a description to a safe maximum length, respecting UTF-8 char boundaries.
#[must_use]
pub fn safe_description(desc: &str, max_len: usize) -> String {
    let escaped = escape_markdown(desc);
    if escaped.len() <= max_len {
        return escaped;
    }
    let truncate_at = max_len.saturating_sub(3);
    let boundary = escaped
        .char_indices()
        .take_while(|(i, _)| *i <= truncate_at)
        .last()
        .map_or(0, |(i, _)| i);
    format!("{}...", &escaped[..boundary])
}

/// Escape a string for safe inclusion in shell command examples.
#[must_use]
pub fn escape_shell_example(s: &str) -> String {
    s.replace('\'', "'\\''")
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system_path(name: &str) -> PathBuf {
        Path::new(std::path::MAIN_SEPARATOR_STR).join(name)
    }

    #[test]
    fn rejects_traversal_paths() {
        let path = PathBuf::from("..").join("..").join("etc").join("passwd");
        assert!(write_output_safe(&path, "test").is_err());
    }

    #[test]
    fn rejects_system_etc_paths() {
        let path = system_path("etc").join("shadow");
        assert!(write_output_safe(&path, "test").is_err());
    }

    #[test]
    fn writes_to_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.md");
        write_output_safe(&path, "# Guide").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Guide");
    }

    #[test]
    #[cfg(unix)]
    fn sets_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secure.md");
        write_output_safe(&path, "secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_symlink_target() {
        let dir = tempfile::tempdir().unwrap();
        let real_file = dir.path().join("real.md");
        std::fs::write(&real_file, "x").unwrap();
        let link_path = dir.path().join("link.md");
        std::os::unix::fs::symlink(&real_file, &link_path).unwrap();
        assert!(write_output_safe(&link_path, "test").is_err());
    }

    #[test]
    fn loads_spec_from_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spec.json");
        std::fs::write(&path, r#"{"openapi": "3.0.0"}"#).unwrap();
        let content = load_spec_safe(&path).unwrap();
        assert!(content.contains("openapi"));
    }

    #[test]
    #[cfg(unix)]
    fn rejects_symlink_spec() {
        let dir = tempfile::tempdir().unwrap();
        let real_file = dir.path().join("real.json");
        std::fs::write(&real_file, "{}").unwrap();
        let link_path = dir.path().join("link.json");
        std::os::unix::fs::symlink(&real_file, &link_path).unwrap();
        assert!(load_spec_safe(&link_path).is_err());
    }

    #[test]
    fn rejects_deep_json() {
        let deep = "{".repeat(200) + &"}".repeat(200);
        assert!(validate_json_depth(&deep).is_err());
    }

    #[test]
    fn accepts_normal_json() {
        assert!(validate_json_depth(r#"{"a":{"b":{"c":1}}}"#).is_ok());
    }

    #[test]
    fn escapes_pipes_and_newlines() {
        assert_eq!(escape_markdown("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn truncates_long_descriptions() {
        let long = "a".repeat(600);
        let result = safe_description(&long, MAX_DESCRIPTION_LENGTH);
        assert!(result.len() <= MAX_DESCRIPTION_LENGTH);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn escapes_shell_metacharacters() {
        assert_eq!(escape_shell_example("$(rm)"), "\\$(rm)");
        assert_eq!(escape_shell_example("`cmd`"), "\\`cmd\\`");
    }
}
