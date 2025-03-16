use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

// Order models
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    Market,
    // Other order types can be added later
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderStatus {
    Filled,
    // Other statuses can be added later
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: Option<f64>,
    pub status: OrderStatus,
    pub position_id: Option<ObjectId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub filled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateOrderRequest {
    #[validate(length(min = 1, max = 10))]
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    #[validate(range(min = 0.0001))]
    pub quantity: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: Option<f64>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub filled_at: Option<DateTime<Utc>>,
}

impl From<Order> for OrderResponse {
    fn from(order: Order) -> Self {
        Self {
            id: order.id.unwrap().to_string(),
            symbol: order.symbol,
            order_type: order.order_type,
            side: order.side,
            quantity: order.quantity,
            price: order.price,
            status: order.status,
            created_at: order.created_at,
            filled_at: order.filled_at,
        }
    }
}

// Position models
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub side: OrderSide,
    pub opened_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionResponse {
    pub id: String,
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub side: OrderSide,
    pub opened_at: DateTime<Utc>,
}

impl From<Position> for PositionResponse {
    fn from(position: Position) -> Self {
        Self {
            id: position.id.unwrap().to_string(),
            symbol: position.symbol,
            quantity: position.quantity,
            entry_price: position.entry_price,
            current_price: position.current_price,
            unrealized_pnl: position.unrealized_pnl,
            side: position.side,
            opened_at: position.opened_at,
        }
    }
}

// Trading stats
#[derive(Debug, Serialize, Deserialize)]
pub struct TradingStatsResponse {
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub pnl_percentage: f64,
    pub average_profit: f64,
    pub average_loss: f64,
    pub risk_reward_ratio: f64,
    pub current_balance: f64,
}