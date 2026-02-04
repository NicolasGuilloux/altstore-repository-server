use crate::cache::IpaCache;
use crate::models::Config;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    #[allow(dead_code)]
    pub base_path: PathBuf,
    pub apps_dir: PathBuf,
    pub auth_token: Option<String>,
    /// Optional secret key for generating obfuscated download tokens
    pub download_secret: Option<Arc<String>>,
    /// Cache for IPA metadata to avoid repeated extraction
    pub ipa_cache: Arc<IpaCache>,
}
