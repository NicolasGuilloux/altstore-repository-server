use crate::discovery::discover_ipas;
use crate::generator::generate_repository;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RepositoryQuery {
    #[serde(default)]
    token: Option<String>,
}

/// Derives the base URL from the incoming request headers.
/// Checks X-Forwarded-Proto/X-Forwarded-Host first (reverse proxy),
/// then falls back to the Host header.
fn base_url_from_headers(headers: &HeaderMap) -> String {
    let host = headers
        .get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("host").and_then(|v| v.to_str().ok()))
        .unwrap_or("localhost");

    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    format!("{}://{}", proto, host)
}

/// Dynamically generates and serves repository.json based on config.json and discovered IPAs
pub async fn serve_repository_json(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RepositoryQuery>,
) -> Result<Response, (StatusCode, String)> {
    tracing::debug!("Generating repository.json dynamically");

    // Re-discover IPAs to reflect current filesystem state
    let ipa_index = discover_ipas(&state.apps_dir, Some(&state.ipa_cache))
        .await
        .map_err(|err| {
            tracing::error!("Failed to discover IPAs: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to discover IPA files: {}", err),
            )
        })?;

    // Clone the config to avoid holding the Arc lock
    let config = (*state.config).clone();

    // Get download secret if configured
    let download_secret = state.download_secret.as_ref().map(|s| s.as_str());

    // Derive base URL from request headers
    let base_url = base_url_from_headers(&headers);

    // Generate the repository with populated versions from discovered IPAs
    let repository = generate_repository(
        config,
        &ipa_index,
        &base_url,
        download_secret,
        query.token.as_deref(),
    )
    .map_err(|err| {
        tracing::error!("Failed to generate repository: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate repository manifest: {}", err),
        )
    })?;

    // Serialize to JSON
    let content = serde_json::to_string_pretty(&repository).map_err(|err| {
        tracing::error!("Failed to serialize repository: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize repository manifest: {}", err),
        )
    })?;

    tracing::debug!(
        "Successfully generated repository.json ({} bytes)",
        content.len()
    );

    // Return the JSON with proper content type
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        content,
    )
        .into_response())
}
