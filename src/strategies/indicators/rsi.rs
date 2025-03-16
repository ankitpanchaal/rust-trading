pub struct RSIIndicator;

impl RSIIndicator {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate(&self, prices: &[f64], period: usize) -> Vec<f64> {
        if prices.len() <= period {
            return Vec::new();
        }

        let mut gains = Vec::with_capacity(prices.len() - 1);
        let mut losses = Vec::with_capacity(prices.len() - 1);
        
        // Calculate price changes
        for i in 1..prices.len() {
            let change = prices[i] - prices[i - 1];
            gains.push(if change > 0.0 { change } else { 0.0 });
            losses.push(if change < 0.0 { -change } else { 0.0 });
        }
        
        let mut result = Vec::with_capacity(prices.len() - period);
        
        // Calculate first average gain and loss
        let first_avg_gain = gains[0..period].iter().sum::<f64>() / period as f64;
        let first_avg_loss = losses[0..period].iter().sum::<f64>() / period as f64;
        
        let mut avg_gain = first_avg_gain;
        let mut avg_loss = first_avg_loss;
        
        // Calculate first RSI
        let mut rsi = 100.0 - (100.0 / (1.0 + avg_gain / avg_loss.max(0.00001)));
        result.push(rsi);
        
        // Calculate subsequent values
        for i in period..gains.len() {
            avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;
            
            rsi = 100.0 - (100.0 / (1.0 + avg_gain / avg_loss.max(0.00001)));
            result.push(rsi);
        }
        
        result
    }
}