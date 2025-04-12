use serde::{Deserialize, Serialize};
use ta::indicators::{ExponentialMovingAverage, RelativeStrengthIndex};
use ta::Next;
use crate::binance::model::KlineSummary;
use super::vwap::Signal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmaRsiConfig {
    pub ema_short_period: u32,    // Short EMA period (e.g., 5)
    pub ema_long_period: u32,     // Long EMA period (e.g., 12)
    pub rsi_period: u32,          // RSI period (e.g., 21)
    pub rsi_threshold: f64,       // RSI threshold for buy signal (e.g., 50)
    pub sl_percent: f64,          // Stop loss percentage
    pub tp_percent: f64,          // Take profit percentage
}

pub struct EmaRsiStrategy {
    config: EmaRsiConfig,
}

impl EmaRsiStrategy {
    pub fn new(config: EmaRsiConfig) -> Self {
        Self { config }
    }
    
    // Access the configuration
    pub fn config(&self) -> &EmaRsiConfig {
        &self.config
    }
    
    // Calculate EMAs and RSI for a series of klines
    pub fn calculate_indicators(&self, klines: &[KlineSummary]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut ema_short_values = Vec::with_capacity(klines.len());
        let mut ema_long_values = Vec::with_capacity(klines.len());
        let mut rsi_values = Vec::with_capacity(klines.len());
        
        // Initialize indicators
        let mut ema_short = ExponentialMovingAverage::new(self.config.ema_short_period as usize)
            .expect("Failed to create short EMA");
        let mut ema_long = ExponentialMovingAverage::new(self.config.ema_long_period as usize)
            .expect("Failed to create long EMA");
        let mut rsi = RelativeStrengthIndex::new(self.config.rsi_period as usize)
            .expect("Failed to create RSI");
        
        // Process each kline
        for kline in klines {
            let close = kline.close.parse::<f64>().unwrap_or(0.0);
            
            // Calculate indicator values
            let ema_short_value = ema_short.next(close);
            let ema_long_value = ema_long.next(close);
            let rsi_value = rsi.next(close);
            
            // Store values
            ema_short_values.push(ema_short_value);
            ema_long_values.push(ema_long_value);
            rsi_values.push(rsi_value);
        }
        
        (ema_short_values, ema_long_values, rsi_values)
    }
    
    // Apply strategy to generate signals for a series of klines
    pub fn apply_strategy(&self, klines: &[KlineSummary]) -> Vec<Signal> {
        let (ema_short_values, ema_long_values, rsi_values) = self.calculate_indicators(klines);
        let mut signals = Vec::with_capacity(klines.len());
        
        // First few candles will have insufficient data for proper signals
        // (need at least max(ema_long_period, rsi_period) candles)
        let warm_up_period = self.config.ema_long_period.max(self.config.rsi_period) as usize;
        
        for i in 0..klines.len() {
            if i < warm_up_period {
                signals.push(Signal::Hold);
                continue;
            }
            
            let ema_short_current = ema_short_values[i];
            let ema_short_previous = ema_short_values[i - 1];
            let ema_long_current = ema_long_values[i];
            let ema_long_previous = ema_long_values[i - 1];
            let rsi_current = rsi_values[i];
            
            // Buy signal: EMA5 crosses above EMA12 and RSI21 > 50
            if ema_short_current > ema_long_current && 
               ema_short_previous <= ema_long_previous && 
               rsi_current > self.config.rsi_threshold {
                signals.push(Signal::Buy);
            }
            // Sell signal: EMA5 crosses below EMA12 or RSI21 < 50
            else if (ema_short_current < ema_long_current && 
                    ema_short_previous >= ema_long_previous) || 
                    rsi_current < self.config.rsi_threshold {
                signals.push(Signal::Sell);
            }
            // Otherwise hold
            else {
                signals.push(Signal::Hold);
            }
        }
        
        signals
    }
}