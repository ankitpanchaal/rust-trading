use axum::{
  body::Body,
  extract::State,
  http::{Request, StatusCode},
  middleware::Next,
  response::Response,
};

use crate::{config::Config, error::AppError, utils::jwt};

pub async fn auth_middleware(
  State(config): State<Config>,
  mut req: Request<Body>,
  next: Next,
) -> Result<Response, AppError> {
  // Extract the token from Authorization header
  let token = extract_token_from_request(&req)?;
  
  // Verify the token
  let claims = jwt::verify_jwt(&token, &config.jwt_secret)?;
  
  // Add user ID to request extensions for handlers to use
  req.extensions_mut().insert(claims.sub.clone());
  
  // Continue with the request
  Ok(next.run(req).await)
}

fn extract_token_from_request(req: &Request<Body>) -> Result<String, AppError> {
  // Get Authorization header
  let auth_header = req
      .headers()
      .get("Authorization")
      .ok_or_else(|| AppError::AuthError("Missing Authorization header".into()))?
      .to_str()
      .map_err(|_| AppError::AuthError("Invalid Authorization header".into()))?;
  
  // Check if it's a Bearer token
  if !auth_header.starts_with("Bearer ") {
      return Err(AppError::AuthError("Invalid Authorization header format".into()));
  }
  
  // Extract the token
  let token = auth_header[7..].to_string();
  if token.is_empty() {
      return Err(AppError::AuthError("Empty token".into()));
  }
  
  Ok(token)
}