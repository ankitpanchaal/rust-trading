use super::moving_average::MovingAverageIndicator;

pub struct MACDIndicator;

impl MACDIndicator {
    pub fn new() -> Self {
        Self
    }

    // Calculate MACD line, signal line, and histogram
    pub fn calculate(
        &self, 
        prices: &[f64], 
        fast_period: usize, 
        slow_period: usize, 
        signal_period: usize
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let ma_indicator = MovingAverageIndicator::new();
        
        // Calculate EMAs
        let fast_ema = ma_indicator.calculate_ema(prices, fast_period);
        let slow_ema = ma_indicator.calculate_ema(prices, slow_period);
        
        if fast_ema.is_empty() || slow_ema.is_empty() {
            return (Vec::new(), Vec::new(), Vec::new());
        }
        
        // Adjust lengths (fast EMA is longer)
        let len_diff = fast_ema.len() - slow_ema.len();
        let adjusted_fast_ema = &fast_ema[len_diff..];
        
        // Calculate MACD line (fast EMA - slow EMA)
        let mut macd_line = Vec::with_capacity(slow_ema.len());
        for i in 0..slow_ema.len() {
            macd_line.push(adjusted_fast_ema[i] - slow_ema[i]);
        }
        
        // Calculate signal line (EMA of MACD line)
        let signal_line = ma_indicator.calculate_ema(&macd_line, signal_period);
        
        // Calculate histogram (MACD line - signal line)
        let len_diff = macd_line.len() - signal_line.len();
        let adjusted_macd_line = &macd_line[len_diff..];
        
        let mut histogram = Vec::with_capacity(signal_line.len());
        for i in 0..signal_line.len() {
            histogram.push(adjusted_macd_line[i] - signal_line[i]);
        }
        
        (macd_line, signal_line, histogram)
    }
}