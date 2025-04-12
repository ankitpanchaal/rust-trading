use axum::{extract::State, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use super::live_runner::LiveStrategyRunner;

#[derive(Debug, Deserialize)]
pub struct StartStrategyRequest {
    pub symbol: String,
    pub interval: String,
}

#[derive(Debug, Serialize)]
pub struct ActiveStrategiesResponse {
    pub strategies: Vec<String>,
}

// Start a strategy for a user
pub async fn start_strategy(
    Extension(user_id): Extension<String>,
    State(runner): State<LiveStrategyRunner>,
    Json(req): Json<StartStrategyRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match runner.start_strategy_for_user(&user_id, &req.symbol, &req.interval).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )),
    }
}

// Stop a strategy for a user
pub async fn stop_strategy(
    Extension(user_id): Extension<String>,
    State(runner): State<LiveStrategyRunner>,
    Json(req): Json<StartStrategyRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match runner.stop_strategy_for_user(&user_id, &req.symbol, &req.interval).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )),
    }
}

// Get active strategies for a user
pub async fn get_active_strategies(
    Extension(user_id): Extension<String>,
    State(runner): State<LiveStrategyRunner>,
) -> Result<Json<ActiveStrategiesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let strategies = runner.get_active_strategies_for_user(&user_id).await;
    Ok(Json(ActiveStrategiesResponse { strategies }))
}