use serde::{Deserialize, Serialize};
use validator::Validate;

// Request/Response models for price endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketPriceRequest {
    pub symbol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketPriceResponse {
    pub symbol: String,
    pub price: String,
    pub timestamp: u64,
}

// Order book request/response
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderBookRequest {
    pub symbol: String,
    pub limit: Option<u64>,
}

// WebSocket subscription request/response
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct WebSocketSubscriptionRequest {
    #[validate(length(min = 1, message = "Symbol cannot be empty"))]
    pub symbol: String,
    pub stream_type: StreamType,
    #[serde(default)]
    pub interval: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StreamType {
    #[serde(rename = "ticker")]
    Ticker,
    #[serde(rename = "kline")]
    Kline,
    #[serde(rename = "depth")]
    Depth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketSubscriptionResponse {
    pub connection_id: String,
    pub status: String,
}

// Order request/response
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct OrderRequest {
    #[validate(length(min = 1, message = "Symbol cannot be empty"))]
    pub symbol: String,
    pub side: OrderSide,
    pub type_: OrderType,
    #[validate(range(min = 0.0001, message = "Quantity must be greater than 0.0001"))]
    pub quantity: f64,
    pub price: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderSide {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "LIMIT")]
    Limit,
    #[serde(rename = "MARKET")]
    Market,
}

// Generic error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

// Update the KlineRequest struct to ensure it has appropriate defaults

#[derive(Debug, Serialize, Deserialize)]
pub struct KlineRequest {
    pub interval: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub start_time: Option<u64>,
    #[serde(default)]
    pub end_time: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlineSummary {
    pub open_time: u64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time: u64,
    pub quote_asset_volume: String,
    pub number_of_trades: u64,
    pub taker_buy_base_asset_volume: String,
    pub taker_buy_quote_asset_volume: String,
}