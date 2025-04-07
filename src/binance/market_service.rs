use binance::{api::*, market::*};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tokio::task;

use crate::error::AppError;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PriceUpdate {
    pub symbol: String,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct BinanceMarketService {
    // Channel for broadcasting price updates to all subscribers
    price_tx: broadcast::Sender<PriceUpdate>,
    // Track active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, bool>>>,
}

impl BinanceMarketService {
    pub fn new() -> Self {
        // Create a broadcast channel for price updates with buffer size 100
        let (price_tx, _) = broadcast::channel::<PriceUpdate>(100);

        let service = Self {
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
        let symbol_clone = symbol.to_string();

        // Use spawn_blocking to perform the Binance API call in a blocking context
        let result = task::spawn_blocking(move || {
            let market: Market = Binance::new(None, None);
            market.get_price(&symbol_clone)
        })
        .await;

        match result {
            Ok(Ok(ticker)) => {
                // Current timestamp in milliseconds
                let timestamp = chrono::Utc::now().timestamp_millis() as u64;
                Ok((ticker.price.to_string(), timestamp))
            }
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get book ticker (best bid/ask)
    pub async fn get_book_ticker(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        let symbol_clone = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let market: Market = Binance::new(None, None);
            market.get_book_ticker(&symbol_clone)
        })
        .await;

        match result {
            Ok(Ok(ticker)) => Ok(serde_json::json!({
                "symbol": symbol,
                "bid_price": ticker.bid_price,
                "bid_qty": ticker.bid_qty,
                "ask_price": ticker.ask_price,
                "ask_qty": ticker.ask_qty
            })),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get 24hr price statistics
    pub async fn get_24h_stats(&self, symbol: &str) -> Result<serde_json::Value, AppError> {
        let symbol_clone = symbol.to_string();

        let result = task::spawn_blocking(move || {
            let market: Market = Binance::new(None, None);
            market.get_24h_price_stats(&symbol_clone)
        })
        .await;

        match result {
            Ok(Ok(stats)) => {
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
            }
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get order book
    pub async fn get_depth(
        &self,
        symbol: &str,
        limit: Option<u64>,
    ) -> Result<serde_json::Value, AppError> {
        let symbol_clone = symbol.to_string();
        let limit_clone = limit.clone();

        let result = task::spawn_blocking(move || {
            let market: Market = Binance::new(None, None);
            match limit_clone {
                Some(limit_val) => market.get_custom_depth(&symbol_clone, limit_val),
                None => market.get_depth(&symbol_clone),
            }
        })
        .await;

        match result {
            Ok(Ok(depth)) => Ok(serde_json::json!({
                "symbol": symbol,
                "bids": depth.bids,
                "asks": depth.asks,
            })),
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Get historical klines/candles
    pub async fn get_historical_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: usize,
    ) -> Result<Vec<f64>, AppError> {
        let symbol_clone = symbol.to_string();
        let interval_clone = interval.to_string();
        let limit_clone = limit;

        let result = task::spawn_blocking(move || {
            let market: Market = Binance::new(None, None);
            market.get_klines(
                &symbol_clone,
                &interval_clone,
                limit_clone as u16,
                None,
                None,
            )
        })
        .await;

        match result {
            Ok(Ok(klines)) => {
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
            }
            Ok(Err(e)) => Err(AppError::InternalError(format!(
                "Binance API error: {:?}",
                e
            ))),
            Err(e) => Err(AppError::InternalError(format!("Task join error: {:?}", e))),
        }
    }

    // Start the WebSocket connection to receive market data
    async fn start_market_data_stream(&self) {
        use binance::websockets::*;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        // Keep a reference to self.price_tx rather than cloning it outside the loop
        let subscriptions = self.subscriptions.clone();

        // Run this task in a loop to restart in case of disconnection
        loop {
            // Atomic bool for controlling the WebSocket loop wrapped in Arc
            let keep_running = Arc::new(AtomicBool::new(true));

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
            let endpoints: Vec<String> = current_subs
                .iter()
                .map(|symbol| format!("{}@ticker", symbol.to_lowercase()))
                .collect();

            if endpoints.is_empty() {
                continue;
            }

            // Clone price_tx inside the loop, right before creating the WebSocket
            let price_tx_clone = self.price_tx.clone();

            // Create a clone of keep_running for the closure
            let keep_running_clone = Arc::clone(&keep_running);

            // Use spawn_blocking for WebSocket connection as well
            let endpoints_clone = endpoints.clone();
            let websocket_result = task::spawn_blocking(move || {
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
                        }
                        _ => (),
                    };

                    Ok(())
                });

                // Connect to WebSocket streams
                match web_socket.connect_multiple_streams(&endpoints_clone) {
                    Ok(_) => {
                        // Start the event loop with the Arc<AtomicBool>
                        web_socket.event_loop(&keep_running_clone)
                    }
                    Err(e) => {
                        eprintln!("Failed to connect WebSocket: {:?}", e);
                        Err(e)
                    }
                }
            })
            .await;

            if let Err(e) = websocket_result {
                eprintln!("WebSocket task error: {:?}", e);
            }

            // Set keep_running to false to exit the loop
            keep_running.store(false, Ordering::Relaxed);
        }
    }
}
