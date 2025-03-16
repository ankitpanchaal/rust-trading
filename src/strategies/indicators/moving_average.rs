pub struct MovingAverageIndicator;

impl MovingAverageIndicator {
    pub fn new() -> Self {
        Self
    }

    // Simple Moving Average (SMA)
    pub fn calculate_sma(&self, prices: &[f64], period: usize) -> Vec<f64> {
        if prices.len() < period {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(prices.len() - period + 1);
        
        for i in period - 1..prices.len() {
            let sum: f64 = prices[i - (period - 1)..=i].iter().sum();
            let sma = sum / period as f64;
            result.push(sma);
        }
        
        result
    }

    // Exponential Moving Average (EMA)
    pub fn calculate_ema(&self, prices: &[f64], period: usize) -> Vec<f64> {
        if prices.len() < period {
            return Vec::new();
        }
        
        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut result = Vec::with_capacity(prices.len() - period + 1);
        
        // First EMA is SMA
        let sma = prices[0..period].iter().sum::<f64>() / period as f64;
        result.push(sma);
        
        // Calculate subsequent EMAs
        for i in period..prices.len() {
            let ema = (prices[i] - result.last().unwrap()) * multiplier + result.last().unwrap();
            result.push(ema);
        }
        
        result
    }
}