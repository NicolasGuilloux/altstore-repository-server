mod auth;
mod discovery;
mod generator;
mod ipa_info;
mod models;
mod routes;
mod state;
mod token;

use anyhow::{Context, Result};
use axum::{
    http::{header, Method},
    middleware,
    routing::get,
    Router,
};
use clap::Parser;
use discovery::discover_ipas;
use state::AppState;
use std::{path::PathBuf, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// AltStore Repository Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server listen address
    #[arg(long, env = "LISTEN_URL", default_value = "0.0.0.0")]
    listen_url: String,

    /// Server listen port
    #[arg(long, env = "LISTEN_PORT", default_value = "8080")]
    listen_port: u16,

    /// External base URL for download links
    #[arg(long, env = "EXTERNAL_BASE_URL")]
    external_base_url: Option<String>,

    /// Directory containing app IPA files
    #[arg(long, env = "APPS_DIR", default_value = "apps")]
    apps_dir: PathBuf,

    /// Optional authentication token required as query parameter
    #[arg(long, env = "AUTH_TOKEN")]
    auth_token: Option<String>,

    /// Optional secret key for generating obfuscated download URLs
    #[arg(long, env = "DOWNLOAD_SECRET")]
    download_secret: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (ignore errors if file doesn't exist)
    let _ = dotenvy::dotenv();

    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "altstore_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting AltStore Repository Server");

    // Parse CLI arguments (with environment variable fallbacks)
    let args = Args::parse();

    tracing::info!("Configuration:");
    tracing::info!("  Listen URL: {}", args.listen_url);
    tracing::info!("  Listen Port: {}", args.listen_port);
    tracing::info!("  Apps Directory: {}", args.apps_dir.display());
    if let Some(ref base_url) = args.external_base_url {
        tracing::info!("  External Base URL: {}", base_url);
    }
    if args.auth_token.is_some() {
        tracing::info!("  Authentication: Enabled (token required as query parameter)");
    } else {
        tracing::info!("  Authentication: Disabled");
    }
    if args.download_secret.is_some() {
        tracing::info!("  Download URLs: Obfuscated (using secret key)");
    } else {
        tracing::info!("  Download URLs: Standard (non-obfuscated)");
    }

    // Determine base path (current directory)
    let base_path = std::env::current_dir().context("Failed to get current directory")?;
    tracing::info!("Base path: {}", base_path.display());

    // Resolve apps directory (can be absolute or relative to base_path)
    let apps_dir = if args.apps_dir.is_absolute() {
        args.apps_dir.clone()
    } else {
        base_path.join(&args.apps_dir)
    };
    tracing::info!("Apps directory: {}", apps_dir.display());

    // Path to config.json
    let config_json_path = base_path.join("config.json");
    if !config_json_path.exists() {
        anyhow::bail!("config.json not found at: {}", config_json_path.display());
    }
    tracing::info!("config.json path: {}", config_json_path.display());

    // Read and parse config.json
    let config_content =
        std::fs::read_to_string(&config_json_path).context("Failed to read config.json")?;
    let config: models::Config =
        serde_json::from_str(&config_content).context("Failed to parse config.json")?;
    tracing::info!("Loaded configuration for: {}", config.name);

    // Discover IPAs
    let ipa_index = discover_ipas(&apps_dir).context("Failed to discover IPAs")?;

    if ipa_index.is_empty() {
        tracing::warn!("No IPAs discovered. Server will still run but no apps are available.");
    }

    // Determine external base URL
    let external_base_url = args
        .external_base_url
        .unwrap_or_else(|| format!("http://{}:{}", args.listen_url, args.listen_port));

    tracing::info!("Repository URL: {}/repository.json", external_base_url);

    // Create shared application state
    let state = AppState {
        config: Arc::new(config),
        base_path: base_path.clone(),
        apps_dir,
        external_base_url,
        auth_token: args.auth_token,
        download_secret: args.download_secret.map(Arc::new),
    };

    // Configure CORS (allow all origins for AltStore compatibility)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::HEAD])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    // Build the router
    let app = Router::new()
        .route("/", get(routes::serve_repository_json))
        .route("/repository.json", get(routes::serve_repository_json))
        .route("/apps/:app_name/:filename", get(routes::serve_ipa))
        .route("/download/:token", get(routes::serve_ipa_obfuscated))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::validate_token,
        ))
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    // Bind to address
    let addr = format!("{}:{}", args.listen_url, args.listen_port);
    tracing::info!("Listening on {}", addr);

    // Create listener
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    tracing::info!("Server started successfully");

    // Run the server with graceful shutdown
    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}
