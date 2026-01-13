use crate::discovery::{discover_ipas, is_valid_path_component};
use crate::state::AppState;
use crate::token::generate_download_token;
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

    // If DOWNLOAD_SECRET is configured, direct downloads are disabled
    // Users must use obfuscated /download/:token URLs instead
    if state.download_secret.is_some() {
        tracing::warn!(
            "Direct download attempt rejected (DOWNLOAD_SECRET configured): {}/{}",
            app_name,
            filename
        );
        return Err((
            StatusCode::FORBIDDEN,
            "Direct downloads are disabled. Use the repository URL to access downloads."
                .to_string(),
        ));
    }

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

/// Serves IPA files using obfuscated download tokens
/// This handler searches for the IPA that matches the provided token
pub async fn serve_ipa_obfuscated(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, String)> {
    tracing::debug!("Request for IPA with token: {}", token);

    // Re-discover IPAs to get current filesystem state
    let ipa_index = discover_ipas(&state.apps_dir).map_err(|err| {
        tracing::error!("Failed to discover IPAs: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to discover IPA files: {}", err),
        )
    })?;

    // Get the secret if configured
    let secret = state.download_secret.as_ref().map(|s| s.as_str());

    // Search through all apps and IPAs to find the one matching this token
    for (app_name, ipas) in ipa_index.iter() {
        for ipa in ipas {
            let ipa_token = generate_download_token(app_name, &ipa.filename, secret);

            if ipa_token == token {
                // Found the matching IPA!
                tracing::info!(
                    "Serving IPA via obfuscated URL: {}/{} ({} bytes)",
                    app_name,
                    ipa.filename,
                    ipa.size
                );

                // Open the file for streaming
                let file = File::open(&ipa.path).await.map_err(|err| {
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
                    .header(header::CONTENT_LENGTH, ipa.size.to_string())
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", ipa.filename),
                    )
                    .body(body)
                    .map_err(|err| {
                        tracing::error!("Failed to build response: {}", err);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to build response: {}", err),
                        )
                    })?;

                return Ok(response);
            }
        }
    }

    // No matching token found
    tracing::debug!("No IPA found for token: {}", token);
    Err((StatusCode::NOT_FOUND, "Download not found".to_string()))
}
