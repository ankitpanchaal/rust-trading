use serde::{Deserialize, Serialize};
use crate::binance::model::KlineSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VwapConfig {
    pub period: u32,        // Period for VWAP calculation
    pub sl_percent: f64,    // Stop loss percentage
    pub tp_percent: f64,    // Take profit percentage
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub struct VwapStrategy {
    config: VwapConfig,
}

impl VwapStrategy {
    pub fn new(config: VwapConfig) -> Self {
        Self { config }
    }
    
    // Access the configuration
    pub fn config(&self) -> &VwapConfig {
        &self.config
    }
    
    // Calculate VWAP for a series of klines
    pub fn calculate_vwap(&self, klines: &[KlineSummary]) -> Vec<f64> {
        let period = self.config.period as usize;
        let mut vwaps = Vec::with_capacity(klines.len());
        
        for i in 0..klines.len() {
            let start_idx = if i >= period { i - period + 1 } else { 0 };
            let window = &klines[start_idx..=i];
            
            let mut sum_price_volume = 0.0;
            let mut sum_volume = 0.0;
            
            for kline in window {
                let typical_price = (kline.high.parse::<f64>().unwrap_or(0.0) +
                                     kline.low.parse::<f64>().unwrap_or(0.0) +
                                     kline.close.parse::<f64>().unwrap_or(0.0)) / 3.0;
                let volume = kline.volume.parse::<f64>().unwrap_or(0.0);
                
                sum_price_volume += typical_price * volume;
                sum_volume += volume;
            }
            
            let vwap = if sum_volume > 0.0 { sum_price_volume / sum_volume } else { 0.0 };
            vwaps.push(vwap);
        }
        
        vwaps
    }
    
    // Generate a signal for a single kline
    pub fn generate_signal(&self, kline: &KlineSummary, vwap: f64, previous_kline: Option<&KlineSummary>) -> Signal {
        let current_close = kline.close.parse::<f64>().unwrap_or(0.0);
        
        if let Some(prev) = previous_kline {
            let previous_close = prev.close.parse::<f64>().unwrap_or(0.0);
            
            // Buy when price crosses above VWAP
            if previous_close < vwap && current_close > vwap {
                return Signal::Buy;
            }
            
            // Sell when price crosses below VWAP
            if previous_close > vwap && current_close < vwap {
                return Signal::Sell;
            }
        }
        
        Signal::Hold
    }
    
    // Apply strategy to generate signals for a series of klines
    pub fn apply_strategy(&self, klines: &[KlineSummary]) -> Vec<Signal> {
        let vwaps = self.calculate_vwap(klines);
        let mut signals = Vec::with_capacity(klines.len());
        
        // First candle has no previous to compare
        signals.push(Signal::Hold);
        
        for i in 1..klines.len() {
            let signal = self.generate_signal(&klines[i], vwaps[i], Some(&klines[i-1]));
            signals.push(signal);
        }
        
        signals
    }
}