use axum::{
  routing::{get, post},
  Router,
};

use crate::market::{handler, service::MarketService};

pub fn market_routes() -> Router {
  let service = MarketService::new();
  
  Router::new()
      .route("/price/:symbol", get(handler::get_price))
      .route("/price", post(handler::get_price_post))
      .with_state(service)
}