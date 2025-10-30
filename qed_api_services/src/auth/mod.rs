pub mod jwt;
pub mod middleware;

pub use jwt::{AuthError, Claims, JwtManager};
pub use middleware::{auth_middleware, AuthExtension};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token has expired"),
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid token signature"),
            AuthError::TokenCreationError => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token"),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}