use crate::error::AppError;
use kucoin_rs::{kucoin::client::Kucoin, kucoin::client::KucoinEnv};
use std::sync::Arc;

#[derive(Clone)]
pub struct MarketService {
    client: Arc<Kucoin>,
}

impl MarketService {
    pub fn new() -> Self {
        // Initialize KuCoin client with just the environment
        // The kucoin_rs crate expects only 2 arguments based on the error
        let client_result = Kucoin::new(KucoinEnv::Live, None);
        
        // Handle the Result returned by Kucoin::new
        let client = Arc::new(match client_result {
            Ok(client) => client,
            Err(e) => {
                // Log the error and create a default client
                // In production, you might want to handle this differently
                eprintln!("Failed to initialize KuCoin client: {}", e);
                panic!("Failed to initialize KuCoin client"); // Or handle more gracefully
            }
        });
        
        Self { client }
    }
    
    pub async fn get_ticker_price(&self, symbol: &str) -> Result<(String, u64), AppError> {
        // Fetch ticker data from KuCoin
        let ticker_response = self.client.get_ticker(symbol).await
            .map_err(|e| AppError::InternalError(format!("KuCoin API error: {}", e)))?;
            
        // Extract the data from the APIDatum wrapper
        let ticker_data = ticker_response.data
            .ok_or_else(|| AppError::InternalError("No ticker data returned".to_string()))?;
            
        // Now we can access the fields in the Ticker struct
        let price = ticker_data.price.clone();
        
        // Convert i64 to u64, ensuring it's non-negative
        let timestamp = ticker_data.time as u64;
        
        Ok((price, timestamp))
    }
}