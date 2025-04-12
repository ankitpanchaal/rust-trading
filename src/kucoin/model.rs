use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}