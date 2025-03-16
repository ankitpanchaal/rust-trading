use axum::{
  extract::{Path, State},
  http::StatusCode,
  Json,
};

use crate::market::{
  model::{MarketPriceRequest, MarketPriceResponse, ErrorResponse}, 
  service::MarketService
};

pub async fn get_price(
  State(service): State<MarketService>,
  Path(symbol): Path<String>,
) -> Result<Json<MarketPriceResponse>, (StatusCode, Json<ErrorResponse>)> {
  match service.get_ticker_price(&symbol).await {
      Ok((price, timestamp)) => {
          let response = MarketPriceResponse {
              symbol,
              price,
              timestamp,
          };
          Ok(Json(response))
      }
      Err(e) => {
          let error_response = ErrorResponse {
              error: format!("Failed to fetch market data: {}", e),
          };
          Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
      }
  }
}

pub async fn get_price_post(
  State(service): State<MarketService>,
  Json(request): Json<MarketPriceRequest>,
) -> Result<Json<MarketPriceResponse>, (StatusCode, Json<ErrorResponse>)> {
  match service.get_ticker_price(&request.symbol).await {
      Ok((price, timestamp)) => {
          let response = MarketPriceResponse {
              symbol: request.symbol,
              price,
              timestamp,
          };
          Ok(Json(response))
      }
      Err(e) => {
          let error_response = ErrorResponse {
              error: format!("Failed to fetch market data: {}", e),
          };
          Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
      }
  }
}