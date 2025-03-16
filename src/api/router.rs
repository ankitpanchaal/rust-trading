use axum::{
  routing::get,
  http::StatusCode,
  Json, Router,
};
use serde_json::json;
use tower_http::{
  cors::{Any, CorsLayer},
  trace::TraceLayer,
};

use crate::{
  auth::{repository::AuthRepository, routes::auth_routes, service::AuthService},
  config::Config,
  db::MongoDb,
  error::AppError,
  market::{routes::market_routes, service::MarketService},
  paper_trading::{
    routes::paper_trading_routes,
    repository::PaperTradingRepository,
    service::PaperTradingService
  },
  strategies::routes::strategy_routes,
};

pub async fn create_router(db: MongoDb) -> Result<Router, AppError> {
  // Load configuration
  let config = Config::from_env()?;
  
  // Setup CORS
  let cors = CorsLayer::new()
      .allow_origin(Any)
      .allow_methods(Any)
      .allow_headers(Any);
  
  // Setup repositories
  let auth_repository = AuthRepository::new(db.clone());
  
  // Setup services
  let auth_service = AuthService::new(auth_repository, config.clone());
  let market_service = MarketService::new();
  
  // Create paper trading repository and service
  let paper_trading_repository = PaperTradingRepository::new(db.clone(), market_service.clone());
  let paper_trading_service = PaperTradingService::new(paper_trading_repository, market_service.clone());
  
  // Setup routes
  let api_routes = Router::new()
      .route("/health", get(health_check))
      .nest("/auth", auth_routes(auth_service))
      .nest("/market", market_routes())
      .nest("/trading", paper_trading_routes(db.clone(), market_service.clone(), config.clone()))
      .nest("/st", strategy_routes(db.clone(), paper_trading_service.clone(), market_service.clone(), config.clone()));
  
  // Build the router
  let app = Router::new()
      .with_state(config)
      .nest("/api/v1", api_routes)
      .layer(TraceLayer::new_for_http())
      .layer(cors);
  
  Ok(app)
}

async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
  (
      StatusCode::OK,
      Json(json!({
          "status": "success",
          "message": "Server is running"
      })),
  )
}