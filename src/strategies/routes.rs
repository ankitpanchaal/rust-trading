use axum::{
  middleware,
  routing::{delete, get, post, put},
  Router,
};

use crate::{
  config::Config,
  db::MongoDb,
  market::service::MarketService,
  middleware::auth::auth_middleware,
  paper_trading::service::PaperTradingService,
  strategies::{
      handler, repository::StrategyRepository, service::StrategyService,
  },
};

pub fn strategy_routes(
  db: MongoDb,
  paper_trading_service: PaperTradingService, 
  market_service: MarketService, 
  config: Config
) -> Router {
  let repository = StrategyRepository::new(db);
  let service = StrategyService::new(repository, paper_trading_service, market_service);
  let auth_config = config.clone();

  Router::new()
      // Strategy CRUD operations
      .route("/strategies", post(handler::create_strategy))
      .route("/strategies", get(handler::get_strategies))
      .route("/strategies/:strategy_id", get(handler::get_strategy))
      .route("/strategies/:strategy_id", put(handler::update_strategy))
      .route("/strategies/:strategy_id", delete(handler::delete_strategy))
      .layer(middleware::from_fn_with_state(auth_config, auth_middleware))
      .with_state(service)
}