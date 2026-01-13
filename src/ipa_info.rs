use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

/// Information extracted from IPA's Info.plist
#[derive(Debug, Clone)]
pub struct IpaInfo {
    pub bundle_identifier: String,
    pub bundle_version: String,
    pub bundle_short_version: Option<String>,
    pub bundle_name: String,
}

/// Subset of Info.plist keys we care about
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InfoPlist {
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: Option<String>,

    #[serde(rename = "CFBundleVersion")]
    bundle_version: Option<String>,

    #[serde(rename = "CFBundleShortVersionString")]
    bundle_short_version: Option<String>,

    #[serde(rename = "CFBundleName")]
    bundle_name: Option<String>,

    #[serde(rename = "CFBundleDisplayName")]
    bundle_display_name: Option<String>,
}

/// Extract Info.plist from an IPA file
pub fn extract_ipa_info(ipa_path: &Path) -> Result<IpaInfo> {
    let file = File::open(ipa_path)
        .with_context(|| format!("Failed to open IPA file: {}", ipa_path.display()))?;

    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader).context("Failed to read IPA as ZIP archive")?;

    // IPA files have structure: Payload/AppName.app/Info.plist
    // Find the Info.plist file
    let plist_path = find_info_plist(&mut archive)?;

    // Read the Info.plist file
    let mut plist_file = archive
        .by_name(&plist_path)
        .context("Failed to read Info.plist from IPA")?;

    let mut plist_data = Vec::new();
    plist_file
        .read_to_end(&mut plist_data)
        .context("Failed to read Info.plist contents")?;

    // Parse the plist
    let info: InfoPlist = plist::from_bytes(&plist_data).context("Failed to parse Info.plist")?;

    // Extract required fields
    let bundle_identifier = info
        .bundle_identifier
        .context("CFBundleIdentifier not found in Info.plist")?;

    let bundle_version = info
        .bundle_version
        .context("CFBundleVersion not found in Info.plist")?;

    let bundle_name = info
        .bundle_display_name
        .or(info.bundle_name)
        .context("CFBundleName or CFBundleDisplayName not found in Info.plist")?;

    Ok(IpaInfo {
        bundle_identifier,
        bundle_version,
        bundle_short_version: info.bundle_short_version,
        bundle_name,
    })
}

/// Find the Info.plist file within the IPA archive
fn find_info_plist(archive: &mut ZipArchive<BufReader<File>>) -> Result<String> {
    for i in 0..archive.len() {
        let file = archive.by_index(i).context("Failed to access ZIP entry")?;
        let name = file.name();

        // Look for Payload/*/Info.plist
        if name.starts_with("Payload/") && name.ends_with(".app/Info.plist") {
            return Ok(name.to_string());
        }
    }

    anyhow::bail!("Info.plist not found in IPA archive")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_plist_path_detection() {
        // This is a unit test - we'd need a real IPA file to test extraction
        // For now, just ensure the module compiles
    }
}
