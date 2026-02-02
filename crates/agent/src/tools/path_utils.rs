//! Path validation utilities for workspace-safe operations

use std::path::{Path, PathBuf};

/// Error type for path validation failures
#[derive(Debug, Clone)]
pub struct PathValidationError {
    pub path: String,
    pub workspace: String,
}

impl std::fmt::Display for PathValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path {} is outside workspace {}",
            self.path, self.workspace
        )
    }
}

impl std::error::Error for PathValidationError {}

/// Validates that a path is within the workspace.
///
/// Steps:
/// 1. Expand `~/` to home directory if present, or join relative paths to workspace
/// 2. Convert to absolute path (using canonicalize if the path exists, or joining with cwd)
/// 3. Ensure the result starts with the canonical workspace root
///
/// Returns the validated absolute path or an error if outside workspace.
pub async fn validate_workspace_path(
    path: &str,
    workspace_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Step 1: Expand ~ to home directory OR join relative paths to workspace
    let expanded = if !path.starts_with("/") && !path.starts_with("~") {
        workspace_root.join(path)
    } else {
        expand_tilde(path)
    };

    // Step 2: Get absolute path
    // If the path exists, use canonicalize to resolve symlinks and get absolute path
    // If it doesn't exist, join with current dir and normalize
    let absolute = if expanded.exists() {
        match tokio::fs::canonicalize(&expanded).await {
            Ok(p) => p,
            Err(_e) => {
                // If canonicalize fails (e.g., broken symlink), try to get absolute path manually
                std::env::current_dir()?.join(&expanded)
            }
        }
    } else {
        // For non-existent paths, we need to resolve the parent directory if possible
        // and then join the file name
        let parent = expanded.parent().filter(|p| !p.as_os_str().is_empty());
        let file_name = expanded.file_name();

        if let Some(parent) = parent {
            let canonical_parent = if parent.exists() {
                tokio::fs::canonicalize(parent)
                    .await
                    .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(parent))
            } else {
                std::env::current_dir()?.join(parent)
            };

            if let Some(file_name) = file_name {
                canonical_parent.join(file_name)
            } else {
                canonical_parent
            }
        } else {
            std::env::current_dir()?.join(&expanded)
        }
    };

    // Step 3: Canonicalize workspace root
    let canonical_workspace = if workspace_root.exists() {
        tokio::fs::canonicalize(workspace_root)
            .await
            .unwrap_or_else(|_| workspace_root.to_path_buf())
    } else {
        workspace_root.to_path_buf()
    };

    // Step 4: Check if the absolute path starts with the workspace
    if !is_path_within_workspace(&absolute, &canonical_workspace) {
        return Err(Box::new(PathValidationError {
            path: path.to_string(),
            workspace: canonical_workspace.display().to_string(),
        }));
    }

    Ok(absolute)
}

/// Check if a path is within the workspace
fn is_path_within_workspace(path: &Path, workspace: &Path) -> bool {
    // Use components comparison for proper path prefix checking
    let path_components: Vec<_> = path.components().collect();
    let workspace_components: Vec<_> = workspace.components().collect();

    if path_components.len() < workspace_components.len() {
        return false;
    }

    for (i, workspace_comp) in workspace_components.iter().enumerate() {
        if path_components.get(i) != Some(workspace_comp) {
            return false;
        }
    }

    true
}

/// Expand tilde (~) to home directory
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Get the default workspace path (~/.opensam/ops)
pub fn default_workspace_path() -> PathBuf {
    opensam_config::workspace_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_path_within_workspace() {
        let workspace = Path::new("/home/user/.opensam/ops");

        // Inside workspace
        assert!(is_path_within_workspace(
            Path::new("/home/user/.opensam/ops/file.txt"),
            workspace
        ));
        assert!(is_path_within_workspace(
            Path::new("/home/user/.opensam/ops/subdir/file.txt"),
            workspace
        ));

        // Outside workspace
        assert!(!is_path_within_workspace(
            Path::new("/home/user/other/file.txt"),
            workspace
        ));
        assert!(!is_path_within_workspace(
            Path::new("/etc/passwd"),
            workspace
        ));
        assert!(!is_path_within_workspace(
            Path::new("/home/user/.opensam"),
            workspace
        ));

        // Edge case: same path
        assert!(is_path_within_workspace(workspace, workspace));
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().expect("Should have home dir");

        assert_eq!(expand_tilde("~/test"), home.join("test"));
        assert_eq!(
            expand_tilde("/absolute/path"),
            PathBuf::from("/absolute/path")
        );
        assert_eq!(
            expand_tilde("relative/path"),
            PathBuf::from("relative/path")
        );
    }

    #[tokio::test]
    async fn test_validate_workspace_path_inside() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let test_file = workspace.join("test.txt");
        fs::write(&test_file, "content").unwrap();

        let result = validate_workspace_path(test_file.to_str().unwrap(), workspace).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn test_validate_workspace_path_outside() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        let outside_file = temp_dir.path().join("outside.txt");
        fs::write(&outside_file, "content").unwrap();

        let result = validate_workspace_path(outside_file.to_str().unwrap(), &workspace).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("is outside workspace"));
    }

    #[tokio::test]
    async fn test_validate_workspace_path_traversal_escape() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        // Create a file outside the workspace
        let outside_file = temp_dir.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        // Try to access it using ../ escape
        let result = validate_workspace_path("../secret.txt", &workspace).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_workspace_path_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Non-existent file inside workspace should be ok (returns path)
        let result =
            validate_workspace_path(workspace.join("new_file.txt").to_str().unwrap(), workspace)
                .await;

        assert!(result.is_ok());
    }
}
