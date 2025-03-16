use axum::{
  http::StatusCode,
  response::{IntoResponse, Response},
  Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
  #[error("Authentication error: {0}")]
  AuthError(String),
  
  #[error("Authorization error: {0}")]
  AuthzError(String),
  
  #[error("Authorization error: {0}")]
  AuthorizationError(String),
  
  #[error("Validation error: {0}")]
  ValidationError(String),
  
  #[error("Database error: {0}")]
  DatabaseError(String),
  
  #[error("Config error: {0}")]
  ConfigError(String),
  
  #[error("Not found: {0}")]
  NotFoundError(String),
  
  #[error("Internal server error: {0}")]
  InternalError(String),
}

impl From<mongodb::error::Error> for AppError {
  fn from(err: mongodb::error::Error) -> Self {
      Self::DatabaseError(err.to_string())
  }
}

impl From<jsonwebtoken::errors::Error> for AppError {
  fn from(err: jsonwebtoken::errors::Error) -> Self {
      Self::AuthError(err.to_string())
  }
}

impl From<bcrypt::BcryptError> for AppError {
  fn from(err: bcrypt::BcryptError) -> Self {
      Self::InternalError(format!("Password hashing error: {}", err))
  }
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
      let (status, error_message) = match self {
          AppError::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
          AppError::AuthzError(_) => (StatusCode::FORBIDDEN, self.to_string()),
          AppError::AuthorizationError(_) => (StatusCode::FORBIDDEN, self.to_string()),
          AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
          AppError::NotFoundError(_) => (StatusCode::NOT_FOUND, self.to_string()),
          AppError::DatabaseError(err) => (
              StatusCode::INTERNAL_SERVER_ERROR,
              format!("Database error: {}", err),
          ),
          AppError::ConfigError(_) => (
              StatusCode::INTERNAL_SERVER_ERROR,
              "A configuration error occurred".to_string(),
          ),
          AppError::InternalError(err) => (
              StatusCode::INTERNAL_SERVER_ERROR,
              format!("An internal server error occurred: {}", err),
          ),
      };

      let body = Json(json!({
          "status": "error",
          "message": error_message,
      }));

      (status, body).into_response()
  }
}