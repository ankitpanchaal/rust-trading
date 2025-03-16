use axum::{
  extract::{Path, State},
  http::StatusCode,
  Extension, Json,
};
use validator::Validate;

use crate::{
  error::AppError,
  strategies::{
      model::{CreateStrategyRequest, StrategyResponse, UpdateStrategyRequest},
      service::StrategyService,
  },
};

// Create a new strategy
pub async fn create_strategy(
  Extension(user_id): Extension<String>,
  State(service): State<StrategyService>,
  Json(req): Json<CreateStrategyRequest>,
) -> Result<Json<StrategyResponse>, (StatusCode, Json<serde_json::Value>)> {
  // Validate request
  if let Err(e) = req.validate() {
      return Err((
          StatusCode::BAD_REQUEST,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      ));
  }

  match service.create_strategy(&user_id, req).await {
      Ok(response) => Ok(Json(response)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Update an existing strategy
pub async fn update_strategy(
  Extension(user_id): Extension<String>,
  State(service): State<StrategyService>,
  Path(strategy_id): Path<String>,
  Json(req): Json<UpdateStrategyRequest>,
) -> Result<Json<StrategyResponse>, (StatusCode, Json<serde_json::Value>)> {
  // Validate request
  if let Err(e) = req.validate() {
      return Err((
          StatusCode::BAD_REQUEST,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      ));
  }

  match service.update_strategy(&user_id, &strategy_id, req).await {
      Ok(response) => Ok(Json(response)),
      Err(e) => match e {
          AppError::NotFoundError(_) => Err((
              StatusCode::NOT_FOUND,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
          AppError::AuthorizationError(_) => Err((
              StatusCode::FORBIDDEN,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
          _ => Err((
              StatusCode::INTERNAL_SERVER_ERROR,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
      },
  }
}

// Get a specific strategy
pub async fn get_strategy(
  Extension(user_id): Extension<String>,
  State(service): State<StrategyService>,
  Path(strategy_id): Path<String>,
) -> Result<Json<StrategyResponse>, (StatusCode, Json<serde_json::Value>)> {
  match service.get_strategy(&user_id, &strategy_id).await {
      Ok(response) => Ok(Json(response)),
      Err(e) => match e {
          AppError::NotFoundError(_) => Err((
              StatusCode::NOT_FOUND,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
          AppError::AuthorizationError(_) => Err((
              StatusCode::FORBIDDEN,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
          _ => Err((
              StatusCode::INTERNAL_SERVER_ERROR,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
      },
  }
}

// Get all strategies for a user
pub async fn get_strategies(
  Extension(user_id): Extension<String>,
  State(service): State<StrategyService>,
) -> Result<Json<Vec<StrategyResponse>>, (StatusCode, Json<serde_json::Value>)> {
  match service.get_user_strategies(&user_id).await {
      Ok(responses) => Ok(Json(responses)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Delete a strategy
pub async fn delete_strategy(
  Extension(user_id): Extension<String>,
  State(service): State<StrategyService>,
  Path(strategy_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
  match service.delete_strategy(&user_id, &strategy_id).await {
      Ok(_) => Ok(StatusCode::NO_CONTENT),
      Err(e) => match e {
          AppError::NotFoundError(_) => Err((
              StatusCode::NOT_FOUND,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
          AppError::AuthorizationError(_) => Err((
              StatusCode::FORBIDDEN,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
          _ => Err((
              StatusCode::INTERNAL_SERVER_ERROR,
              Json(serde_json::json!({ "error": format!("{}", e) })),
          )),
      },
  }
}