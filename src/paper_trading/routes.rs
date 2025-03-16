use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};

use crate::{
    db::MongoDb,
    market::service::MarketService,
    middleware::auth::auth_middleware,
    paper_trading::{handler, repository::PaperTradingRepository, service::PaperTradingService},
};

pub fn paper_trading_routes(db: MongoDb, market_service: MarketService) -> Router {
    let repository = PaperTradingRepository::new(db, market_service.clone());
    let service = PaperTradingService::new(repository, market_service);

    Router::new()
        // Paper trading setup
        .route("/enable", post(handler::enable_paper_trading))
        
        // Orders
        .route("/orders", post(handler::create_order))
        .route("/orders", get(handler::get_orders))
        
        // Positions
        .route("/positions", get(handler::get_positions))
        .route("/positions/:position_id", get(handler::get_position))
        // We can keep this endpoint for backward compatibility but it doesn't do much
        .route("/positions/update", put(handler::update_positions))
        
        // Account info
        .route("/balance", get(handler::get_balance))
        .route("/stats", get(handler::get_trading_stats))
        .with_state(service)
}