use crate::discovery::discover_ipas;
use crate::generator::generate_repository;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RepositoryQuery {
    #[serde(default)]
    token: Option<String>,
}

/// Dynamically generates and serves repository.json based on config.json and discovered IPAs
pub async fn serve_repository_json(
    State(state): State<AppState>,
    Query(query): Query<RepositoryQuery>,
) -> Result<Response, (StatusCode, String)> {
    tracing::debug!("Generating repository.json dynamically");

    // Re-discover IPAs to reflect current filesystem state
    let ipa_index = discover_ipas(&state.apps_dir).map_err(|err| {
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

    // Generate the repository with populated versions from discovered IPAs
    let repository = generate_repository(
        config,
        &ipa_index,
        &state.external_base_url,
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
