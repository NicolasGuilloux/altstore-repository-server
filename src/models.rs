use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration structure (from config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub identifier: String,
    pub website: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "tintColor")]
    pub tint_color: String,
    #[serde(rename = "iconURL")]
    pub icon_url: String,
    pub apps: Vec<AppConfig>,
    #[serde(rename = "sourceURL")]
    pub source_url: String,
    #[serde(default)]
    pub news: Vec<NewsItem>,
    #[serde(default, rename = "userInfo")]
    pub user_info: HashMap<String, serde_json::Value>,
}

/// App configuration (base metadata without versions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub beta: Option<bool>,
    pub name: String,
    #[serde(rename = "bundleIdentifier")]
    pub bundle_identifier: String,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    pub subtitle: Option<String>,
    #[serde(rename = "localizedDescription")]
    pub localized_description: String,
    #[serde(rename = "iconURL")]
    pub icon_url: String,
    #[serde(rename = "tintColor")]
    pub tint_color: String,
    pub category: String,
    #[serde(rename = "screenshotURLs")]
    pub screenshot_urls: Vec<String>,
    #[serde(rename = "appPermissions")]
    pub app_permissions: AppPermissions,
    /// Versions can be manually configured or will be populated dynamically from filesystem
    #[serde(default)]
    pub versions: Vec<AppVersion>,
}

/// App permissions structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPermissions {
    pub entitlements: Vec<String>,
    pub privacy: HashMap<String, String>,
}

/// App version entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVersion {
    pub version: String,
    pub date: String,
    #[serde(rename = "localizedDescription")]
    pub localized_description: String,
    #[serde(rename = "downloadURL")]
    pub download_url: String,
    pub size: u64,
}

/// News item for updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    #[serde(rename = "appID")]
    pub app_id: String,
    pub caption: String,
    pub date: String,
    pub identifier: String,
    pub notify: bool,
    #[serde(rename = "tintColor")]
    pub tint_color: String,
    pub title: String,
}

/// Repository structure (output for /repository.json)
/// This is essentially the same as Config but with populated versions
pub type Repository = Config;
