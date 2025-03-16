use axum::{
  extract::{Path, State},
  http::StatusCode,
  Extension, Json,
};
use validator::Validate;

use crate::{
  auth::model::EnablePaperTradingRequest,
  error::AppError,
  paper_trading::{
      model::{CreateOrderRequest, OrderResponse, PositionResponse, TradingStatsResponse},
      service::PaperTradingService,
  },
};

// Enable paper trading for a user
pub async fn enable_paper_trading(
  Extension(user_id): Extension<String>,
  State(service): State<PaperTradingService>,
  Json(req): Json<EnablePaperTradingRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
  match service.enable_paper_trading(&user_id, req.initial_balance_usd).await {
      Ok(_) => Ok(StatusCode::OK),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Create a new order
pub async fn create_order(
  Extension(user_id): Extension<String>,
  State(service): State<PaperTradingService>,
  Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderResponse>, (StatusCode, Json<serde_json::Value>)> {
  // Validate request
  if let Err(e) = req.validate() {
      return Err((
          StatusCode::BAD_REQUEST,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      ));
  }

  match service.create_order(&user_id, req).await {
      Ok(response) => Ok(Json(response)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Get all positions for the current user
pub async fn get_positions(
  Extension(user_id): Extension<String>,
  State(service): State<PaperTradingService>,
) -> Result<Json<Vec<PositionResponse>>, (StatusCode, Json<serde_json::Value>)> {
  match service.get_positions(&user_id).await {
      Ok(positions) => Ok(Json(positions)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Get orders
pub async fn get_orders(
  Extension(user_id): Extension<String>,
  State(service): State<PaperTradingService>,
) -> Result<Json<Vec<OrderResponse>>, (StatusCode, Json<serde_json::Value>)> {
  match service.get_orders(&user_id).await {
      Ok(orders) => Ok(Json(orders)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Get account balance details
pub async fn get_balance(
  Extension(user_id): Extension<String>,
  State(service): State<PaperTradingService>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
  match service.get_user_balance_details(&user_id).await {
      Ok(balance) => Ok(Json(balance)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// Get trading stats
pub async fn get_trading_stats(
  Extension(user_id): Extension<String>,
  State(service): State<PaperTradingService>,
) -> Result<Json<TradingStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
  match service.get_trading_stats(&user_id).await {
      Ok(stats) => Ok(Json(stats)),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}