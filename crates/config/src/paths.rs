//! FOX-DIE Path utilities

use std::path::PathBuf;

/// FOX-DIE secure data vault (~/.opensam)
pub fn data_dir() -> PathBuf {
    dirs::home_dir()
        .expect("◆ FAILED TO LOCATE HOME BASE")
        .join(".opensam")
}

/// Mission parameters location
pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

/// Operations theater location
pub fn workspace_path() -> PathBuf {
    data_dir().join("ops")
}

/// Mission logs storage
pub fn sessions_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Timeline operations storage
pub fn cron_dir() -> PathBuf {
    data_dir().join("timeline")
}

/// Media intelligence storage
pub fn media_dir() -> PathBuf {
    data_dir().join("intel")
}

/// Ensure directory exists
pub async fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
    tokio::fs::create_dir_all(path).await
}

/// Sanitize filename for secure storage
pub fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect()
}
