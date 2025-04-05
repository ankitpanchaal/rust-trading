use binance::{api::*, account::*};

use crate::error::AppError;

#[derive(Clone)]
pub struct BinanceUserService {
    account: Account,
}

impl BinanceUserService {
    pub fn new(api_key: String, secret_key: String) -> Self {
        let account: Account = Binance::new(Some(api_key), Some(secret_key));
        
        Self {
            account,
        }
    }
    
    // Get account information
    pub async fn get_account(&self) -> Result<serde_json::Value, AppError> {
        match self.account.get_account() {
            Ok(account_info) => {
                Ok(serde_json::json!({
                    "maker_commission": account_info.maker_commission,
                    "taker_commission": account_info.taker_commission,
                    "buyer_commission": account_info.buyer_commission,
                    "seller_commission": account_info.seller_commission,
                    "can_trade": account_info.can_trade,
                    "can_withdraw": account_info.can_withdraw,
                    "can_deposit": account_info.can_deposit,
                    "balances": account_info.balances
                }))
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get account balance for a specific asset
    pub async fn get_balance(&self, asset: &str) -> Result<serde_json::Value, AppError> {
        match self.account.get_balance(asset) {
            Ok(balance) => {
                Ok(serde_json::json!({
                    "asset": asset,
                    "free": balance.free,
                    "locked": balance.locked
                }))
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get open orders for a symbol
    pub async fn get_open_orders(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        match self.account.get_open_orders(symbol) {
            Ok(orders) => Ok(serde_json::json!(orders)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get order status
    pub async fn get_order_status(&self, symbol: &str, order_id: u64) -> Result<serde_json::Value, AppError> {
        match self.account.order_status(symbol, order_id) {
            Ok(status) => Ok(serde_json::json!(status)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Place a limit buy order
    pub async fn limit_buy(&self, symbol: &str, quantity: f64, price: f64) -> Result<serde_json::Value, AppError> {
        match self.account.limit_buy(symbol, quantity, price) {
            Ok(response) => Ok(serde_json::json!(response)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Place a limit sell order
    pub async fn limit_sell(&self, symbol: &str, quantity: f64, price: f64) -> Result<serde_json::Value, AppError> {
        match self.account.limit_sell(symbol, quantity, price) {
            Ok(response) => Ok(serde_json::json!(response)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Place a market buy order
    pub async fn market_buy(&self, symbol: &str, quantity: f64) -> Result<serde_json::Value, AppError> {
        match self.account.market_buy(symbol, quantity) {
            Ok(response) => Ok(serde_json::json!(response)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Place a market sell order
    pub async fn market_sell(&self, symbol: &str, quantity: f64) -> Result<serde_json::Value, AppError> {
        match self.account.market_sell(symbol, quantity) {
            Ok(response) => Ok(serde_json::json!(response)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Cancel an order
    pub async fn cancel_order(&self, symbol: &str, order_id: u64) -> Result<serde_json::Value, AppError> {
        match self.account.cancel_order(symbol, order_id) {
            Ok(response) => Ok(serde_json::json!(response)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Cancel all open orders for a symbol
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        match self.account.cancel_all_open_orders(symbol) {
            Ok(response) => Ok(serde_json::json!(response)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get trade history for a symbol
    pub async fn get_trade_history(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        match self.account.trade_history(symbol) {
            Ok(trades) => Ok(serde_json::json!(trades)),
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
}