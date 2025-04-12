use axum::{http::StatusCode, routing::get, Json, Router};
use serde_json::json;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    auth::{repository::AuthRepository, routes::auth_routes, service::AuthService},
    binance::{market_service::BinanceMarketService, routes::binance_routes},
    config::Config,
    db::MongoDb,
    error::AppError,
    paper_trading::{
        repository::PaperTradingRepository, routes::paper_trading_routes,
        service::PaperTradingService,
    },
    strategy::{routes::strategy_routes, LiveStrategyRunner},
    telegram::{
        message_service::TelegramMessageService, repository::TelegramRepository,
        routes::telegram_routes,
    },
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
    let binance_service = BinanceMarketService::new();
    // Create paper trading repository and service
    let paper_trading_repository = PaperTradingRepository::new(db.clone(), binance_service.clone());

    // Create telegram message service
    let telegram_repository = TelegramRepository::new(db.clone());
    let telegram_message_service = TelegramMessageService::new(telegram_repository)
        .expect("Failed to create TelegramMessageService");

    let paper_trading_service = PaperTradingService::new(
        paper_trading_repository,
        binance_service.clone(),
        telegram_message_service,
    );
    let live_strategy_runner = LiveStrategyRunner::new(
        binance_service.clone(),
        paper_trading_service.clone(),
        100, // kline buffer size
        auth_service.clone(),
    );

    // Setup routes
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth_routes(auth_service.clone()))
        .nest(
            "/trading",
            paper_trading_routes(db.clone(), binance_service.clone(), config.clone()),
        )
        .nest(
            "/telegram",
            telegram_routes(db.clone(), auth_service.clone(), config.clone())?,
        )
        .nest("/binance", binance_routes()?)
        .nest(
            "/strategy",
            strategy_routes(live_strategy_runner, config.clone()),
        );

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
