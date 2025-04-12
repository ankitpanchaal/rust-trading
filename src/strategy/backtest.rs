use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::Path;
use chrono::{DateTime, Utc};

use crate::binance::market_service::BinanceMarketService;
// Removed the unused import: KlineSummary
use crate::error::AppError;
use super::vwap::{Signal, VwapStrategy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub quantity: f64,
    pub side: String,
    pub profit_loss: Option<f64>,
    pub profit_loss_percent: Option<f64>,
    pub exit_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BacktestResult {
    pub symbol: String,
    pub interval: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub initial_balance: f64,
    pub final_balance: f64,
    pub total_profit_loss: f64,
    pub profit_loss_percent: f64,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub total_trades: usize,
    pub win_rate: f64,
    pub trades: Vec<Trade>,
}

pub struct Backtest {
    market_service: BinanceMarketService,
    strategy: VwapStrategy,
    initial_balance: f64,
}

impl Backtest {
    pub fn new(market_service: BinanceMarketService, strategy: VwapStrategy, initial_balance: f64) -> Self {
        Self {
            market_service,
            strategy,
            initial_balance,
        }
    }
    
    pub async fn run(&self, symbol: &str, interval: &str, limit: usize) -> Result<BacktestResult, AppError> {
        // Fetch historical klines
        let klines = self.market_service.get_historical_klines(symbol, interval, limit).await?;
        
        // Apply strategy to get signals
        let signals = self.strategy.apply_strategy(&klines);
        
        // Execute backtest
        let mut balance = self.initial_balance;
        let mut position = None;
        let mut trades = Vec::new();
        
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        
        for (i, kline) in klines.iter().enumerate() {
            if i == 0 { continue; } // Skip first kline as we don't have a signal for it
            
            let current_close = kline.close.parse::<f64>().unwrap_or(0.0);
            let open_time = DateTime::<Utc>::from_timestamp(kline.open_time as i64 / 1000, 0)
                .unwrap_or_else(|| Utc::now());
            let close_time = DateTime::<Utc>::from_timestamp(kline.close_time as i64 / 1000, 0)
                .unwrap_or_else(|| Utc::now());
            
            match &signals[i] {
                Signal::Buy if position.is_none() => {
                    // Calculate quantity based on 95% of available balance
                    let quantity = (balance * 0.95) / current_close;
                    
                    // Create a trade
                    let trade = Trade {
                        entry_time: open_time,
                        exit_time: None,
                        entry_price: current_close,
                        exit_price: None,
                        quantity,
                        side: "BUY".to_string(),
                        profit_loss: None,
                        profit_loss_percent: None,
                        exit_reason: None,
                    };
                    
                    // Update balance and set position
                    balance -= quantity * current_close;
                    position = Some((trades.len(), quantity));
                    trades.push(trade);
                },
                // Fixed: Removed if-let guard and used a regular match + if let pattern
                Signal::Sell => {
                    if let Some((trade_idx, qty)) = position {
                        // Close the position
                        let entry_trade = &trades[trade_idx];
                        let entry_price = entry_trade.entry_price;
                        
                        // Calculate profit/loss
                        let trade_value = qty * current_close;
                        let profit_loss = trade_value - (qty * entry_price);
                        let profit_loss_percent = (profit_loss / (qty * entry_price)) * 100.0;
                        
                        // Update trade
                        let mut trade = trades[trade_idx].clone();
                        trade.exit_time = Some(close_time);
                        trade.exit_price = Some(current_close);
                        trade.profit_loss = Some(profit_loss);
                        trade.profit_loss_percent = Some(profit_loss_percent);
                        trade.exit_reason = Some("SIGNAL".to_string());
                        
                        // Update counters
                        if profit_loss > 0.0 {
                            winning_trades += 1;
                        } else {
                            losing_trades += 1;
                        }
                        
                        // Update balance and reset position
                        balance += trade_value;
                        position = None;
                        trades[trade_idx] = trade;
                    }
                },
                _ => {}
            }
            
            // Check for stop-loss or take-profit if in a position
            if let Some((trade_idx, qty)) = position {
                let entry_trade = &trades[trade_idx];
                let entry_price = entry_trade.entry_price;
                
                // Calculate current profit/loss percentage
                let profit_loss_percent = (current_close - entry_price) / entry_price * 100.0;
                
                // Check stop-loss
                if profit_loss_percent <= -self.strategy.config().sl_percent {
                    // Close position at stop-loss
                    let trade_value = qty * current_close;
                    let profit_loss = trade_value - (qty * entry_price);
                    
                    // Update trade
                    let mut trade = trades[trade_idx].clone();
                    trade.exit_time = Some(close_time);
                    trade.exit_price = Some(current_close);
                    trade.profit_loss = Some(profit_loss);
                    trade.profit_loss_percent = Some(profit_loss_percent);
                    trade.exit_reason = Some("STOP_LOSS".to_string());
                    
                    // Update counters
                    losing_trades += 1;
                    
                    // Update balance and reset position
                    balance += trade_value;
                    position = None;
                    trades[trade_idx] = trade;
                }
                // Check take-profit
                else if profit_loss_percent >= self.strategy.config().tp_percent {
                    // Close position at take-profit
                    let trade_value = qty * current_close;
                    let profit_loss = trade_value - (qty * entry_price);
                    
                    // Update trade
                    let mut trade = trades[trade_idx].clone();
                    trade.exit_time = Some(close_time);
                    trade.exit_price = Some(current_close);
                    trade.profit_loss = Some(profit_loss);
                    trade.profit_loss_percent = Some(profit_loss_percent);
                    trade.exit_reason = Some("TAKE_PROFIT".to_string());
                    
                    // Update counters
                    winning_trades += 1;
                    
                    // Update balance and reset position
                    balance += trade_value;
                    position = None;
                    trades[trade_idx] = trade;
                }
            }
        }
        
        // Rest of the code remains unchanged...
        // Close any open position with the last kline price
        if let Some((trade_idx, qty)) = position {
            if let Some(last_kline) = klines.last() {
                let entry_trade = &trades[trade_idx];
                let entry_price = entry_trade.entry_price;
                let last_close = last_kline.close.parse::<f64>().unwrap_or(0.0);
                let last_close_time = DateTime::<Utc>::from_timestamp(last_kline.close_time as i64 / 1000, 0)
                    .unwrap_or_else(|| Utc::now());
                
                // Calculate profit/loss
                let trade_value = qty * last_close;
                let profit_loss = trade_value - (qty * entry_price);
                let profit_loss_percent = (profit_loss / (qty * entry_price)) * 100.0;
                
                // Update trade
                let mut trade = trades[trade_idx].clone();
                trade.exit_time = Some(last_close_time);
                trade.exit_price = Some(last_close);
                trade.profit_loss = Some(profit_loss);
                trade.profit_loss_percent = Some(profit_loss_percent);
                trade.exit_reason = Some("END_OF_BACKTEST".to_string());
                
                // Update counters
                if profit_loss > 0.0 {
                    winning_trades += 1;
                } else {
                    losing_trades += 1;
                }
                
                // Update balance
                balance += trade_value;
                trades[trade_idx] = trade;
            }
        }
        
        // Calculate final statistics
        let total_trades = winning_trades + losing_trades;
        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };
        
        let total_profit_loss = balance - self.initial_balance;
        let profit_loss_percent = (total_profit_loss / self.initial_balance) * 100.0;
        
        let start_time = if let Some(first_kline) = klines.first() {
            DateTime::<Utc>::from_timestamp(first_kline.open_time as i64 / 1000, 0)
                .unwrap_or_else(|| Utc::now())
        } else {
            Utc::now()
        };
        
        let end_time = if let Some(last_kline) = klines.last() {
            DateTime::<Utc>::from_timestamp(last_kline.close_time as i64 / 1000, 0)
                .unwrap_or_else(|| Utc::now())
        } else {
            Utc::now()
        };
        
        // Create result
        let result = BacktestResult {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            start_time,
            end_time,
            initial_balance: self.initial_balance,
            final_balance: balance,
            total_profit_loss,
            profit_loss_percent,
            winning_trades,
            losing_trades,
            total_trades,
            win_rate,
            trades,
        };
        
        Ok(result)
    }
    
    pub fn save_result(&self, result: &BacktestResult, file_path: &str) -> Result<(), AppError> {
        // Create directory if it doesn't exist
        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .map_err(|e| AppError::InternalError(format!("Failed to create directory: {}", e)))?;
        }
        
        // Convert result to JSON
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize result: {}", e)))?;
        
        // Write to file
        let mut file = File::create(file_path)
            .map_err(|e| AppError::InternalError(format!("Failed to create file: {}", e)))?;
        
        file.write_all(json.as_bytes())
            .map_err(|e| AppError::InternalError(format!("Failed to write to file: {}", e)))?;
        
        Ok(())
    }
}