use axum::{
  extract::State,
  http::StatusCode,
  Json,
};

use crate::{
  auth::{
      model::{AuthResponse, LoginUserRequest, RefreshTokenRequest, RegisterUserRequest, UserResponse},
      service::AuthService,
  },
  error::AppError,
};

pub async fn register(
  State(service): State<AuthService>,
  Json(req): Json<RegisterUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
  let user = service.register(req).await?;
  Ok((StatusCode::CREATED, Json(user)))
}

pub async fn login(
  State(service): State<AuthService>,
  Json(req): Json<LoginUserRequest>,
) -> Result<Json<AuthResponse>, AppError> {
  let response = service.login(req).await?;
  Ok(Json(response))
}

pub async fn refresh_token(
  State(service): State<AuthService>,
  Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<AuthResponse>, AppError> {
  let response = service.refresh_token(&req.refresh_token).await?;
  Ok(Json(response))
}

pub async fn me(
  State(service): State<AuthService>,
  user_id: String,
) -> Result<Json<UserResponse>, AppError> {
  let user = service.get_user_by_id(&user_id).await?;
  Ok(Json(user))
}