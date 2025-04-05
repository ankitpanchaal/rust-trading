use axum::{
    routing::{delete, get, post},
    Router,
};
use std::env;

use crate::{
    binance::{handler, market_service::BinanceMarketService, user_service::BinanceUserService},
    error::AppError,
};

pub fn binance_routes() -> Result<Router, AppError> {
    // Initialize services
    let market_service = BinanceMarketService::new();

    // Get API keys from environment variables
    let api_key = env::var("BINANCE_API_KEY")
        .map_err(|_| AppError::ConfigError("BINANCE_API_KEY is not set".into()))?;

    let secret_key = env::var("BINANCE_SECRET_KEY")
        .map_err(|_| AppError::ConfigError("BINANCE_SECRET_KEY is not set".into()))?;

    // Initialize user service with API keys
    let user_service = BinanceUserService::new(api_key, secret_key);

    // Public market data routes (no auth required)
    let market_routes = Router::new()
        .route("/price/:symbol", get(handler::get_price))
        .route("/price", post(handler::get_price_post))
        .route("/ticker/:symbol", get(handler::get_book_ticker))
        .route("/stats/:symbol", get(handler::get_24h_stats))
        .route("/depth/:symbol", get(handler::get_depth))
        .with_state(market_service);

    // User account routes (auth required)
    let account_routes = Router::new()
        .route("/account", get(handler::get_account))
        .route("/balance/:asset", get(handler::get_balance))
        .route("/orders/:symbol", get(handler::get_open_orders))
        .route("/order", post(handler::create_order))
        .route("/order/:symbol/:order_id", get(handler::get_order_status))
        .route("/order/:symbol/:order_id", delete(handler::cancel_order))
        .route("/trades/:symbol", get(handler::get_trade_history))
        .with_state(user_service);

    // Combine all routes
    let router = Router::new()
        .nest("/market", market_routes)
        .nest("/account", account_routes);

    Ok(router)
}
