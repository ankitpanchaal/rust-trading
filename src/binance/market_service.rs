use std::{sync::Arc, collections::HashMap};
use tokio::sync::{broadcast, RwLock};
use binance::{api::*, market::*};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PriceUpdate {
    pub symbol: String,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct BinanceMarketService {
    market: Market,
    // Channel for broadcasting price updates to all subscribers
    price_tx: broadcast::Sender<PriceUpdate>,
    // Track active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, bool>>>,
}

impl BinanceMarketService {
    pub fn new() -> Self {
        // Configure Binance - use default mainnet endpoints
        let market: Market = Binance::new(None, None);
        
        // Create a broadcast channel for price updates with buffer size 100
        let (price_tx, _) = broadcast::channel::<PriceUpdate>(100);
        
        let service = Self {
            market,
            price_tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start the WebSocket connections in the background
        let service_clone = service.clone();
        tokio::spawn(async move {
            service_clone.start_market_data_stream().await;
        });
        
        service
    }
    
    // Get a receiver for price updates
    pub fn subscribe_to_price_updates(&self) -> broadcast::Receiver<PriceUpdate> {
        self.price_tx.subscribe()
    }
    
    // Add a symbol to track
    pub async fn subscribe_to_symbol(&self, symbol: &str) -> Result<(), AppError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(symbol.to_string(), true);
        Ok(())
    }
    
    // Remove a symbol from tracking
    pub async fn unsubscribe_from_symbol(&self, symbol: &str) -> Result<(), AppError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(symbol);
        Ok(())
    }
    
    // Get current ticker price for a symbol
    pub async fn get_ticker_price(&self, symbol: &str) -> Result<(String, u64), AppError> {
        match self.market.get_price(symbol) {
            Ok(ticker) => {
                // Current timestamp in milliseconds
                let timestamp = chrono::Utc::now().timestamp_millis() as u64;
                Ok((ticker.price.to_string(), timestamp))
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get book ticker (best bid/ask)
    pub async fn get_book_ticker(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        match self.market.get_book_ticker(symbol) {
            Ok(ticker) => {
                Ok(serde_json::json!({
                    "symbol": symbol,
                    "bid_price": ticker.bid_price,
                    "bid_qty": ticker.bid_qty,
                    "ask_price": ticker.ask_price,
                    "ask_qty": ticker.ask_qty
                }))
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get 24hr price statistics
    pub async fn get_24h_stats(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        match self.market.get_24h_price_stats(symbol) {
            Ok(stats) => {
                Ok(serde_json::json!({
                    "symbol": symbol,
                    "price_change": stats.price_change,
                    "price_change_percent": stats.price_change_percent,
                    "weighted_avg_price": stats.weighted_avg_price,
                    "prev_close_price": stats.prev_close_price,
                    "last_price": stats.last_price,
                    "last_qty": "NA", //stats.last_qty
                    "bid_price": stats.bid_price,
                    "ask_price": stats.ask_price,
                    "open_price": stats.open_price,
                    "high_price": stats.high_price,
                    "low_price": stats.low_price,
                    "volume": stats.volume,
                    "quote_volume": "NA" //stats.quote_volume
                }))
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get order book
    pub async fn get_depth(&self, symbol: &str, limit: Option<u64>) -> Result<serde_json::Value, AppError> {
        let result = match limit {
            Some(limit) => self.market.get_custom_depth(symbol, limit),
            None => self.market.get_depth(symbol),
        };
        
        match result {
            Ok(depth) => {
                Ok(serde_json::json!({
                    "symbol": symbol,
                    "bids": depth.bids,
                    "asks": depth.asks,
                }))
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Get historical klines/candles
    pub async fn get_historical_klines(
        &self, 
        symbol: &str, 
        interval: &str, 
        limit: usize
    ) -> Result<Vec<f64>, AppError> {
        match self.market.get_klines(symbol, interval, limit as u16, None, None) {
            Ok(klines) => {
                let mut prices = Vec::with_capacity(limit);
                
                match klines {
                    binance::model::KlineSummaries::AllKlineSummaries(all_klines) => {
                        for kline in all_klines {
                            // Extract closing price from each kline and convert to f64
                            if let Ok(close_price) = kline.close.parse::<f64>() {
                                prices.push(close_price);
                            }
                        }
                    }
                }
                
                Ok(prices)
            },
            Err(e) => Err(AppError::InternalError(format!("Binance API error: {:?}", e))),
        }
    }
    
    // Start the WebSocket connection to receive market data
    async fn start_market_data_stream(&self) {
        use binance::websockets::*;
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // Keep a reference to self.price_tx rather than cloning it outside the loop
        let subscriptions = self.subscriptions.clone();
        
        // Run this task in a loop to restart in case of disconnection
        loop {
            // Atomic bool for controlling the WebSocket loop
            let keep_running = AtomicBool::new(true);
            
            // Sleep to prevent rapid reconnection attempts
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            
            // Get current subscriptions
            let current_subs = {
                let subs = subscriptions.read().await;
                subs.keys().cloned().collect::<Vec<String>>()
            };
            
            if current_subs.is_empty() {
                continue;
            }
            
            // Create endpoints for all subscribed symbols
            let endpoints: Vec<String> = current_subs.iter()
                .map(|symbol| format!("{}@ticker", symbol.to_lowercase()))
                .collect();
            
            if endpoints.is_empty() {
                continue;
            }
            
            // Clone price_tx inside the loop, right before creating the WebSocket
            // This ensures a fresh clone for each iteration
            let price_tx_clone = self.price_tx.clone();
            
            // Create WebSocket handler
            let mut web_socket = WebSockets::new(move |event: WebsocketEvent| {
                match event {
                    WebsocketEvent::DayTicker(ticker_event) => {
                        // Extract price and convert to f64
                        if let Ok(price) = ticker_event.current_close.parse::<f64>() {
                            // Convert timestamp from string to u64
                            let timestamp = chrono::Utc::now().timestamp_millis() as u64;
                            
                            let price_update = PriceUpdate {
                                symbol: ticker_event.symbol.clone(),
                                price,
                                timestamp,
                            };
                            
                            // Broadcast the price update
                            let _ = price_tx_clone.send(price_update);
                        }
                    },
                    _ => (),
                };
                
                Ok(())
            });
            
            // Connect to WebSocket streams
            match web_socket.connect_multiple_streams(&endpoints) {
                Ok(_) => {
                    // Start the event loop
                    if let Err(e) = web_socket.event_loop(&keep_running) {
                        eprintln!("WebSocket error: {:?}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to connect WebSocket: {:?}", e);
                }
            }
            
            // Set keep_running to false to exit the loop
            keep_running.store(false, Ordering::Relaxed);
        }
    }
}