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
    pub external_base_url: String,
    pub auth_token: Option<String>,
    /// Optional secret key for generating obfuscated download tokens
    pub download_secret: Option<Arc<String>>,
}
