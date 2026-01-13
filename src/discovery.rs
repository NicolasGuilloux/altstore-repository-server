use crate::ipa_info;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Represents a discovered IPA file with extracted metadata
#[derive(Debug, Clone)]
pub struct IpaEntry {
    #[allow(dead_code)]
    pub app_name: String,
    pub filename: String,
    pub path: PathBuf,
    pub size: u64,
    /// File modification date (used as version date)
    pub modified_date: String,
    /// Bundle identifier (e.g., "com.example.app")
    #[allow(dead_code)]
    pub bundle_identifier: Option<String>,
    /// Bundle version (CFBundleVersion)
    pub bundle_version: Option<String>,
    /// Short version string (CFBundleShortVersionString)
    pub bundle_short_version: Option<String>,
    /// Bundle display name
    #[allow(dead_code)]
    pub bundle_name: Option<String>,
}

/// Index of all discovered IPAs, keyed by app name
pub type IpaIndex = HashMap<String, Vec<IpaEntry>>;

/// Directories to skip during discovery
const SKIP_DIRS: &[&str] = &[
    ".git", ".devenv", ".direnv", ".claude", "target", "src", ".github",
];

/// Validates that a path component doesn't contain directory traversal characters
pub fn is_valid_path_component(component: &str) -> bool {
    !component.is_empty()
        && !component.starts_with('.')
        && !component.contains("..")
        && !component.contains('/')
        && !component.contains('\\')
}

/// Discovers all IPA files in app directories under the apps directory
pub fn discover_ipas(apps_path: &Path) -> Result<IpaIndex> {
    let mut index: IpaIndex = HashMap::new();

    tracing::info!("Scanning for IPAs in: {}", apps_path.display());

    // Check if apps directory exists
    if !apps_path.exists() {
        anyhow::bail!("Apps directory not found: {}", apps_path.display());
    }
    if !apps_path.is_dir() {
        anyhow::bail!("Apps path is not a directory: {}", apps_path.display());
    }

    let scan_path = apps_path;

    // Read all entries in the apps directory
    let entries = fs::read_dir(scan_path).context("Failed to read apps directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Skip if not a directory
        if !path.is_dir() {
            continue;
        }

        // Get directory name
        let dir_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // Skip special directories
        if SKIP_DIRS.contains(&dir_name.as_str()) {
            tracing::debug!("Skipping directory: {}", dir_name);
            continue;
        }

        tracing::debug!("Scanning app directory: {}", dir_name);

        // Scan for .ipa files in this directory (max depth 1)
        let mut ipa_entries = Vec::new();

        for ipa_entry in WalkDir::new(&path).max_depth(1) {
            let ipa_entry = match ipa_entry {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("Failed to read entry in {}: {}", dir_name, err);
                    continue;
                }
            };

            let ipa_path = ipa_entry.path();

            // Skip if not a file
            if !ipa_path.is_file() {
                continue;
            }

            // Check if it's an IPA file
            if let Some(ext) = ipa_path.extension() {
                if ext.eq_ignore_ascii_case("ipa") {
                    // Get filename
                    let filename = match ipa_path.file_name() {
                        Some(name) => name.to_string_lossy().to_string(),
                        None => continue,
                    };

                    // Get file size and modification date
                    let (size, modified_date) = match fs::metadata(ipa_path) {
                        Ok(metadata) => {
                            let size = metadata.len();

                            // Get modification time and format as YYYY-MM-DD
                            let modified_time = metadata.modified().unwrap_or(SystemTime::now());
                            let datetime: DateTime<Utc> = modified_time.into();
                            let date_str = datetime.format("%Y-%m-%d").to_string();

                            (size, date_str)
                        }
                        Err(err) => {
                            tracing::warn!("Failed to get metadata for {}: {}", filename, err);
                            continue;
                        }
                    };

                    // Extract Info.plist information from IPA
                    let (bundle_identifier, bundle_version, bundle_short_version, bundle_name) =
                        match ipa_info::extract_ipa_info(ipa_path) {
                            Ok(info) => {
                                tracing::info!(
                                    "Extracted info from {}/{}: version={}, bundle_id={}",
                                    dir_name,
                                    filename,
                                    info.bundle_version,
                                    info.bundle_identifier
                                );
                                (
                                    Some(info.bundle_identifier),
                                    Some(info.bundle_version),
                                    info.bundle_short_version,
                                    Some(info.bundle_name),
                                )
                            }
                            Err(err) => {
                                tracing::warn!(
                                    "Failed to extract Info.plist from {}/{}: {}",
                                    dir_name,
                                    filename,
                                    err
                                );
                                (None, None, None, None)
                            }
                        };

                    tracing::info!("Discovered IPA: {}/{} ({} bytes)", dir_name, filename, size);

                    ipa_entries.push(IpaEntry {
                        app_name: dir_name.clone(),
                        filename,
                        path: ipa_path.to_path_buf(),
                        size,
                        modified_date,
                        bundle_identifier,
                        bundle_version,
                        bundle_short_version,
                        bundle_name,
                    });
                }
            }
        }

        // Add to index if we found any IPAs
        if !ipa_entries.is_empty() {
            index.insert(dir_name, ipa_entries);
        }
    }

    let total_ipas: usize = index.values().map(|v| v.len()).sum();
    tracing::info!(
        "Discovery complete: {} apps, {} IPAs",
        index.len(),
        total_ipas
    );

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_path_component() {
        // Valid components
        assert!(is_valid_path_component("YourApp"));
        assert!(is_valid_path_component("App123"));
        assert!(is_valid_path_component("my_app"));

        // Invalid components
        assert!(!is_valid_path_component(""));
        assert!(!is_valid_path_component(".hidden"));
        assert!(!is_valid_path_component(".."));
        assert!(!is_valid_path_component("../etc"));
        assert!(!is_valid_path_component("app/name"));
        assert!(!is_valid_path_component("app\\name"));
    }
}
