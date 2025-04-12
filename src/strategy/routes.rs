use axum::{
  middleware,
  routing::{get, post},
  Router,
};

use crate::{
  middleware::auth::auth_middleware,
  config::Config,
};

use super::handler;
use super::live_runner::LiveStrategyRunner;

pub fn strategy_routes(runner: LiveStrategyRunner, config: Config) -> Router {
  Router::new()
      .route("/start", post(handler::start_strategy))
      .route("/stop", post(handler::stop_strategy))
      .route("/active", get(handler::get_active_strategies))
      .layer(middleware::from_fn_with_state(config, auth_middleware))
      .with_state(runner)
}