use crate::discovery::{discover_ipas, is_valid_path_component};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

/// Serves IPA files from the discovered index
pub async fn serve_ipa(
    Path((app_name, filename)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, String)> {
    tracing::debug!("Request for IPA: {}/{}", app_name, filename);

    // Validate path components to prevent directory traversal
    if !is_valid_path_component(&app_name) {
        tracing::warn!("Invalid app_name: {}", app_name);
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid app name: {}", app_name),
        ));
    }

    if !is_valid_path_component(&filename) {
        tracing::warn!("Invalid filename: {}", filename);
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid filename: {}", filename),
        ));
    }

    // Re-discover IPAs to get current filesystem state
    let ipa_index = discover_ipas(&state.apps_dir).map_err(|err| {
        tracing::error!("Failed to discover IPAs: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to discover IPA files: {}", err),
        )
    })?;

    // Look up the app in the index
    let app_ipas = ipa_index.get(&app_name).ok_or_else(|| {
        tracing::debug!("App not found: {}", app_name);
        (
            StatusCode::NOT_FOUND,
            format!("App not found: {}", app_name),
        )
    })?;

    // Find the specific IPA file
    let ipa_entry = app_ipas
        .iter()
        .find(|ipa| ipa.filename == filename)
        .ok_or_else(|| {
            tracing::debug!("IPA file not found: {}/{}", app_name, filename);
            (
                StatusCode::NOT_FOUND,
                format!("IPA file not found: {}", filename),
            )
        })?;

    tracing::info!(
        "Serving IPA: {}/{} ({} bytes)",
        app_name,
        filename,
        ipa_entry.size
    );

    // Open the file for streaming
    let file = File::open(&ipa_entry.path).await.map_err(|err| {
        tracing::error!("Failed to open IPA file: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to open file: {}", err),
        )
    })?;

    // Create a stream from the file
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // Build response with proper headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, ipa_entry.size.to_string())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(body)
        .map_err(|err| {
            tracing::error!("Failed to build response: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {}", err),
            )
        })?;

    Ok(response)
}
