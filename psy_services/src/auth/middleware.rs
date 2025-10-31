use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use super::{AuthError, Claims, JwtManager};

#[derive(Clone)]
pub struct AuthExtension {
    pub claims: Claims,
}

pub async fn auth_middleware(State(jwt_manager): State<Arc<JwtManager>>, mut request: Request, next: Next) -> Result<Response, AuthError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    let token = JwtManager::extract_token_from_header(auth_header).ok_or(AuthError::InvalidToken)?;

    // Validate the token (checks signature and expiration)
    let token_data = jwt_manager.validate_token(&token)?;

    // Add claims to request extensions for use in handlers
    request.extensions_mut().insert(AuthExtension {
        claims: token_data.claims.clone(),
    });

    // Log the authenticated request
    tracing::info!("Authenticated request from '{}' to '{}'", token_data.claims.sub, request.uri().path());

    // Continue to the next middleware/handler
    Ok(next.run(request).await)
}
