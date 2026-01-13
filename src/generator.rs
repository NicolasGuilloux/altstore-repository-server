use crate::discovery::IpaIndex;
use crate::models::{AppVersion, Config, Repository};
use anyhow::{Context, Result};

/// Generates a repository from config and discovered IPAs
pub fn generate_repository(
    config: Config,
    ipa_index: &IpaIndex,
    base_url: &str,
) -> Result<Repository> {
    let mut repo = config;

    // For each app in the config, populate versions from discovered IPAs
    for app in &mut repo.apps {
        // Find matching IPAs in the index
        // Match by the app's directory name (derived from app name)
        let app_dir_name = get_app_directory_name(&app.name);

        // Keep existing manual versions from config.json
        let manual_versions = std::mem::take(&mut app.versions);

        // Generate versions from discovered IPAs
        let mut discovered_versions = Vec::new();

        if let Some(ipas) = ipa_index.get(&app_dir_name) {
            tracing::debug!("Found {} IPAs for app {}", ipas.len(), app.name);

            for ipa in ipas {
                // Try to get version from Info.plist first, fall back to filename parsing
                let version_info = if let Some(ref bundle_version) = ipa.bundle_version {
                    // Prefer CFBundleShortVersionString (user-facing) over CFBundleVersion (build number)
                    let version = ipa
                        .bundle_short_version
                        .as_ref()
                        .unwrap_or(bundle_version)
                        .clone();

                    let description = if let Some(ref short_ver) = ipa.bundle_short_version {
                        if short_ver != bundle_version {
                            format!("Version {} (build {})", short_ver, bundle_version)
                        } else {
                            format!("Version {}", version)
                        }
                    } else {
                        format!("Version {}", version)
                    };

                    Ok(VersionInfo {
                        version,
                        date: ipa.modified_date.clone(),
                        description,
                    })
                } else {
                    // Fallback to filename parsing if Info.plist extraction failed
                    tracing::debug!(
                        "No version info from Info.plist for {}, trying filename parsing",
                        ipa.filename
                    );
                    parse_version_from_filename(&ipa.filename, &ipa.modified_date)
                };

                match version_info {
                    Ok(version_info) => {
                        let download_url = format!(
                            "{}/apps/{}/{}",
                            base_url.trim_end_matches('/'),
                            app_dir_name,
                            ipa.filename
                        );

                        discovered_versions.push(AppVersion {
                            version: version_info.version,
                            date: version_info.date,
                            localized_description: version_info.description,
                            download_url,
                            size: ipa.size,
                        });
                    }
                    Err(err) => {
                        tracing::warn!("Failed to get version info for {}: {}", ipa.filename, err);
                    }
                }
            }
        } else {
            tracing::warn!(
                "No IPAs found for app {} (directory: {})",
                app.name,
                app_dir_name
            );
        }

        // Merge versions: manual versions take precedence over discovered ones
        app.versions = merge_versions(manual_versions, discovered_versions);
    }

    Ok(repo)
}

/// Parsed version information from filename
#[derive(Debug)]
struct VersionInfo {
    version: String,
    date: String,
    description: String,
}

/// Parse version information from IPA filename
/// Expected format: AppName_x.y.z_a.b.c.ipa or similar patterns
/// Examples:
/// - YouTubePlus_5.2b1_20.26.7.ipa -> version "20.26.7"
/// - MyApp_1.2.3.ipa -> version "1.2.3"
fn parse_version_from_filename(filename: &str, file_date: &str) -> Result<VersionInfo> {
    // Remove .ipa extension
    let name = filename
        .strip_suffix(".ipa")
        .context("Filename does not end with .ipa")?;

    // Split by underscores
    let parts: Vec<&str> = name.split('_').collect();

    // Try different parsing strategies based on the number of parts
    let (version, tweak_version) = match parts.len() {
        // Format: AppName_tweakVersion_appVersion.ipa (e.g., YouTubePlus_5.2b1_20.26.7.ipa)
        3 => {
            let tweak = parts[1].to_string();
            let app_ver = parts[2].to_string();
            (app_ver, Some(tweak))
        }
        // Format: AppName_version.ipa (e.g., MyApp_1.2.3.ipa)
        2 => {
            let ver = parts[1].to_string();
            (ver, None)
        }
        // Fallback: use the entire filename without extension
        _ => {
            let ver = name.to_string();
            (ver, None)
        }
    };

    // Generate description based on available info
    let description = match tweak_version {
        Some(tweak) => format!("Version {} (tweak version: {})", version, tweak),
        None => format!("Version {}", version),
    };

    Ok(VersionInfo {
        version,
        date: file_date.to_string(),
        description,
    })
}

/// Get the directory name for an app based on its name
/// This is a simple mapping that can be customized
fn get_app_directory_name(app_name: &str) -> String {
    // For now, just use the app name as-is
    // In the future, this could use a mapping from config
    app_name.to_string()
}

/// Merge manual versions (from config.json) with discovered versions (from IPA files)
/// Manual versions take precedence - if a version string exists in both, use the manual entry
/// but update downloadURL and size from the discovered version if the IPA file exists
fn merge_versions(
    manual_versions: Vec<AppVersion>,
    discovered_versions: Vec<AppVersion>,
) -> Vec<AppVersion> {
    use std::collections::HashMap;

    // Index manual versions by version string
    let mut manual_map: HashMap<String, AppVersion> = manual_versions
        .into_iter()
        .map(|v| (v.version.clone(), v))
        .collect();

    // Process discovered versions
    for discovered in discovered_versions {
        if let Some(manual) = manual_map.get_mut(&discovered.version) {
            // Version exists in both manual and discovered
            // Keep manual entry but update download URL and size from IPA file
            manual.download_url = discovered.download_url;
            manual.size = discovered.size;
            tracing::debug!(
                "Merged version {}: kept manual metadata, updated URL and size from IPA",
                discovered.version
            );
        } else {
            // This is a new discovered version not in manual config
            manual_map.insert(discovered.version.clone(), discovered);
            tracing::debug!("Added discovered version {}", manual_map.len());
        }
    }

    // Convert back to vector and sort by date (newest first)
    let mut merged: Vec<AppVersion> = manual_map.into_values().collect();
    merged.sort_by(|a, b| b.date.cmp(&a.date));

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_from_filename_three_parts() {
        let result =
            parse_version_from_filename("YouTubePlus_5.2b1_20.26.7.ipa", "2025-01-13").unwrap();
        assert_eq!(result.version, "20.26.7");
        assert!(result.description.contains("5.2b1"));
        assert_eq!(result.date, "2025-01-13");
    }

    #[test]
    fn test_parse_version_from_filename_two_parts() {
        let result = parse_version_from_filename("MyApp_1.2.3.ipa", "2025-01-13").unwrap();
        assert_eq!(result.version, "1.2.3");
        assert_eq!(result.date, "2025-01-13");
    }

    #[test]
    fn test_parse_version_from_filename_invalid() {
        let result = parse_version_from_filename("invalid.txt", "2025-01-13");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_versions_keeps_manual_metadata() {
        let manual = vec![AppVersion {
            version: "1.0.0".to_string(),
            date: "2025-01-01".to_string(),
            localized_description: "Custom description".to_string(),
            download_url: "https://old-url.com/file.ipa".to_string(),
            size: 1000,
        }];

        let discovered = vec![AppVersion {
            version: "1.0.0".to_string(),
            date: "2025-01-13".to_string(),
            localized_description: "Auto-generated description".to_string(),
            download_url: "https://new-url.com/file.ipa".to_string(),
            size: 2000,
        }];

        let merged = merge_versions(manual, discovered);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].version, "1.0.0");
        // Manual metadata preserved
        assert_eq!(merged[0].date, "2025-01-01");
        assert_eq!(merged[0].localized_description, "Custom description");
        // Discovered URL and size updated
        assert_eq!(merged[0].download_url, "https://new-url.com/file.ipa");
        assert_eq!(merged[0].size, 2000);
    }

    #[test]
    fn test_merge_versions_adds_new_discovered() {
        let manual = vec![AppVersion {
            version: "1.0.0".to_string(),
            date: "2025-01-01".to_string(),
            localized_description: "Version 1".to_string(),
            download_url: "https://example.com/v1.ipa".to_string(),
            size: 1000,
        }];

        let discovered = vec![
            AppVersion {
                version: "1.0.0".to_string(),
                date: "2025-01-13".to_string(),
                localized_description: "Auto v1".to_string(),
                download_url: "https://example.com/v1-new.ipa".to_string(),
                size: 1500,
            },
            AppVersion {
                version: "2.0.0".to_string(),
                date: "2025-01-13".to_string(),
                localized_description: "Auto v2".to_string(),
                download_url: "https://example.com/v2.ipa".to_string(),
                size: 2000,
            },
        ];

        let merged = merge_versions(manual, discovered);

        assert_eq!(merged.len(), 2);
        // Should be sorted by date (newest first)
        // v2 is newer (2025-01-13) than v1 (2025-01-01)
        assert_eq!(merged[0].version, "2.0.0");
        assert_eq!(merged[1].version, "1.0.0");
    }

    #[test]
    fn test_merge_versions_empty_manual() {
        let manual = vec![];
        let discovered = vec![AppVersion {
            version: "1.0.0".to_string(),
            date: "2025-01-13".to_string(),
            localized_description: "Auto-generated".to_string(),
            download_url: "https://example.com/file.ipa".to_string(),
            size: 1000,
        }];

        let merged = merge_versions(manual, discovered);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].version, "1.0.0");
        assert_eq!(merged[0].localized_description, "Auto-generated");
    }
}
