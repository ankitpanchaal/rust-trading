use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StrategyType {
    MovingAverageCrossover,
    RSIStrategy,
    MACDStrategy,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StrategyStatus {
    Active,
    Paused,
    Stopped,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Strategy {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub name: String,
    pub description: String,
    pub strategy_type: StrategyType,
    pub status: StrategyStatus,
    pub symbols: Vec<String>,
    pub parameters: serde_json::Value,
    pub risk_parameters: RiskParameters,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskParameters {
    pub max_position_size: f64,         // Maximum size of a single position
    pub max_total_positions: u32,       // Maximum number of open positions
    pub stop_loss_percentage: f64,      // Stop loss as a percentage
    pub take_profit_percentage: f64,    // Take profit as a percentage
    pub max_daily_loss: f64,            // Maximum daily loss amount
    pub trailing_stop_enabled: bool,    // Whether trailing stop is enabled
    pub trailing_stop_percentage: f64,  // Trailing stop as a percentage
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateStrategyRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 0, max = 500))]
    pub description: String,
    pub strategy_type: StrategyType,
    #[validate(length(min = 1))]
    pub symbols: Vec<String>,
    pub parameters: serde_json::Value,
    pub risk_parameters: RiskParameters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub strategy_type: StrategyType,
    pub status: StrategyStatus,
    pub symbols: Vec<String>,
    pub parameters: serde_json::Value,
    pub risk_parameters: RiskParameters,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateStrategyRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[validate(length(min = 0, max = 500))]
    pub description: Option<String>,
    pub status: Option<StrategyStatus>,
    pub symbols: Option<Vec<String>>,
    pub parameters: Option<serde_json::Value>,
    pub risk_parameters: Option<RiskParameters>,
}

impl From<Strategy> for StrategyResponse {
    fn from(strategy: Strategy) -> Self {
        Self {
            id: strategy.id.map_or_else(|| "unknown".to_string(), |id| id.to_string()),
            name: strategy.name,
            description: strategy.description,
            strategy_type: strategy.strategy_type,
            status: strategy.status,
            symbols: strategy.symbols,
            parameters: strategy.parameters,
            risk_parameters: strategy.risk_parameters,
            created_at: strategy.created_at,
            updated_at: strategy.updated_at,
            last_executed_at: strategy.last_executed_at,
        }
    }
}