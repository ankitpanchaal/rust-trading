use crate::error::AppError;
use kucoin_rs::{kucoin::client::Kucoin, kucoin::client::KucoinEnv};
use std::{sync::Arc, collections::HashMap};
use tokio::sync::{broadcast, mpsc, RwLock};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PriceUpdate {
    pub symbol: String,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct MarketService {
    client: Arc<Kucoin>,
    // Channel for broadcasting price updates to all subscribers
    price_tx: broadcast::Sender<PriceUpdate>,
    // Track active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, bool>>>, 
}

impl MarketService {
    pub fn new() -> Self {
        let client_result = Kucoin::new(KucoinEnv::Live, None);
        let client = Arc::new(match client_result {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to initialize KuCoin client: {}", e);
                panic!("Failed to initialize KuCoin client");
            }
        });
        
        // Create a broadcast channel for price updates with buffer size 100
        let (price_tx, _) = broadcast::channel::<PriceUpdate>(100);
        
        let service = Self { 
            client,
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
        
        // In a real implementation, you might need to modify the WebSocket subscription here
        Ok(())
    }
    
    // Remove a symbol from tracking
    pub async fn unsubscribe_from_symbol(&self, symbol: &str) -> Result<(), AppError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(symbol);
        Ok(())
    }
    
    // Get current ticker price (kept for compatibility)
    pub async fn get_ticker_price(&self, symbol: &str) -> Result<(String, u64), AppError> {
        let ticker_response = self.client.get_ticker(symbol).await
            .map_err(|e| AppError::InternalError(format!("KuCoin API error: {}", e)))?;
            
        let ticker_data = ticker_response.data
            .ok_or_else(|| AppError::InternalError("No ticker data returned".to_string()))?;
            
        let price = ticker_data.price.clone();
        let timestamp = ticker_data.time as u64;
        
        Ok((price, timestamp))
    }
    
    // Get historical klines/candles
    pub async fn get_historical_klines(
        &self, 
        symbol: &str, 
        interval: &str, 
        limit: usize
    ) -> Result<Vec<f64>, AppError> {
        // In a real implementation, use the KuCoin API to get historical candles
        // For now, we'll fall back to simulated data
        let (price_str, _) = self.get_ticker_price(symbol).await?;
        let current_price = price_str.parse::<f64>().map_err(|_| {
            AppError::InternalError(format!("Failed to parse price: {}", price_str))
        })?;
        
        // Simulate historical data based on current price
        let mut prices = Vec::with_capacity(limit);
        let mut price = current_price;
        
        for _ in 0..limit {
            prices.push(price);
            // Add some random movement
            let change = price * 0.01 * (rand::random::<f64>() - 0.5);
            price += change;
        }
        
        // Reverse to get chronological order (oldest first)
        prices.reverse();
        Ok(prices)
    }
    
    // Start the WebSocket connection to receive market data
    async fn start_market_data_stream(&self) {
        // In a real implementation, connect to KuCoin WebSocket API
        // For our mock implementation, we'll simulate price updates
        
        let price_tx = self.price_tx.clone();
        let subscriptions = self.subscriptions.clone();
        
        // Simulate price updates every second
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // Get active subscriptions
            let subscriptions_guard = subscriptions.read().await;
            
            // Update price for each subscribed symbol
            for symbol in subscriptions_guard.keys() {
                // In a real implementation, you would get this from the WebSocket
                // For now, simulate a price change
                match self.get_ticker_price(symbol).await {
                    Ok((price_str, timestamp)) => {
                        if let Ok(price) = price_str.parse::<f64>() {
                            // Add a small random change to simulate market movement
                            let change = price * 0.001 * (rand::random::<f64>() - 0.5);
                            let new_price = price + change;
                            
                            let price_update = PriceUpdate {
                                symbol: symbol.clone(),
                                price: new_price,
                                timestamp,
                            };
                            
                            // Broadcast the price update to all subscribers
                            let _ = price_tx.send(price_update);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error getting price for {}: {}", symbol, e);
                    }
                }
            }
        }
    }
}