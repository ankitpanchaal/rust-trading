use crate::error::AppError;
use binance::{account::*, api::*};
use tokio::task;

#[derive(Clone)]
pub struct BinanceUserService {
    api_key: String,
    secret_key: String,
}

impl BinanceUserService {
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self {
            api_key,
            secret_key,
        }
    }

    // Get account information
    pub async fn get_account(&self) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.get_account()
        })
        .await;

        match result {
            Ok(Ok(account_info)) => Ok(serde_json::json!({
                "maker_commission": account_info.maker_commission,
                "taker_commission": account_info.taker_commission,
                "buyer_commission": account_info.buyer_commission,
                "seller_commission": account_info.seller_commission,
                "can_trade": account_info.can_trade,
                "can_withdraw": account_info.can_withdraw,
                "can_deposit": account_info.can_deposit,
                "balances": account_info.balances
            })),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get account balance for a specific asset
    pub async fn get_balance(&self, asset: &str) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let asset_for_api = asset.to_string(); // Clone for the API call
        let asset_for_response = asset.to_string(); // Clone for the response

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.get_balance(&asset_for_api)
        })
        .await;

        match result {
            Ok(Ok(balance)) => Ok(serde_json::json!({
                "asset": asset_for_response,
                "free": balance.free,
                "locked": balance.locked
            })),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get open orders for a symbol
    pub async fn get_open_orders(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.get_open_orders(&symbol)
        })
        .await;

        match result {
            Ok(Ok(orders)) => Ok(serde_json::json!(orders)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get order status
    pub async fn get_order_status(
        &self,
        symbol: &str,
        order_id: u64,
    ) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.order_status(&symbol, order_id)
        })
        .await;

        match result {
            Ok(Ok(status)) => Ok(serde_json::json!(status)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Place a limit buy order
    pub async fn limit_buy(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.limit_buy(&symbol, quantity, price)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(serde_json::json!(response)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Place a limit sell order
    pub async fn limit_sell(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.limit_sell(&symbol, quantity, price)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(serde_json::json!(response)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Place a market buy order
    pub async fn market_buy(
        &self,
        symbol: &str,
        quantity: f64,
    ) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.market_buy(&symbol, quantity)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(serde_json::json!(response)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Place a market sell order
    pub async fn market_sell(
        &self,
        symbol: &str,
        quantity: f64,
    ) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.market_sell(&symbol, quantity)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(serde_json::json!(response)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Cancel an order
    pub async fn cancel_order(
        &self,
        symbol: &str,
        order_id: u64,
    ) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.cancel_order(&symbol, order_id)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(serde_json::json!(response)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Cancel all open orders for a symbol
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.cancel_all_open_orders(&symbol)
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(serde_json::json!(response)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get trade history for a symbol
    pub async fn get_trade_history(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        let api_key = self.api_key.clone();
        let secret_key = self.secret_key.clone();
        let symbol = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let account: Account = Binance::new(Some(api_key), Some(secret_key));
            account.trade_history(&symbol)
        })
        .await;

        match result {
            Ok(Ok(trades)) => Ok(serde_json::json!(trades)),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }
}
