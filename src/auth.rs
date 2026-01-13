use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct AuthQuery {
    #[serde(default)]
    token: Option<String>,
}

/// Middleware to validate authentication token from query parameter
pub async fn validate_token(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Skip authentication for obfuscated download routes
    // The obfuscated token itself serves as authentication
    if request.uri().path().starts_with("/download/") {
        return Ok(next.run(request).await);
    }

    // If no auth token is configured, allow all requests
    let Some(expected_token) = &state.auth_token else {
        return Ok(next.run(request).await);
    };

    // If auth token is configured, validate the provided token
    match query.token {
        Some(provided_token) if provided_token == *expected_token => {
            // Token is valid, proceed
            Ok(next.run(request).await)
        }
        Some(_) => {
            // Token provided but incorrect
            tracing::warn!("Invalid authentication token provided");
            Err((StatusCode::UNAUTHORIZED, "Invalid authentication token"))
        }
        None => {
            // No token provided but required
            tracing::warn!("Authentication token required but not provided");
            Err((StatusCode::UNAUTHORIZED, "Authentication token required"))
        }
    }
}
