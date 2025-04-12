use crate::binance::market_service::BinanceMarketService;
use crate::strategy::vwap::{VwapConfig, VwapStrategy};
use crate::strategy::ema_rsi::{EmaRsiConfig, EmaRsiStrategy};
use crate::strategy::backtest::Backtest;

// Function to run backtest
pub async fn run_backtest() -> anyhow::Result<()> {
    // Create market service
    let market_service = BinanceMarketService::new();
    
    // Initial balance for backtest
    let initial_balance = 10000.0;
    let symbol = "BTCUSDT";
    
    // --- VWAP Strategy Backtest ---
    
    // Create VWAP strategy configs for different timeframes
    let vwap_5m_config = VwapConfig {
        period: 20,           // 20-period VWAP
        sl_percent: 2.0,      // 2% stop loss
        tp_percent: 5.0,      // 5% take profit
    };
    
    // Create strategies
    let vwap_5m = VwapStrategy::new(vwap_5m_config);
    
    // Run 5m backtest
    println!("Running VWAP backtest on 5m timeframe...");
    let backtest_5m = Backtest::new(market_service.clone(), vwap_5m, initial_balance);
    let result_5m = backtest_5m.run(symbol, "5m", 1000).await?;
    backtest_5m.save_result(&result_5m, "backtest_results/vwap_5m_btcusdt.json")?;
    
    println!("5m Backtest completed:");
    println!("  Profit: ${:.2} ({:.2}%)", result_5m.total_profit_loss, result_5m.profit_loss_percent);
    println!("  Win rate: {:.2}% ({} winning, {} losing)", 
             result_5m.win_rate, result_5m.winning_trades, result_5m.losing_trades);
    
    // --- EMA+RSI Strategy Backtest ---
    
    // Create EMA+RSI strategy config
    let ema_rsi_config = EmaRsiConfig {
        ema_short_period: 5,   // 5-period EMA (EMA5)
        ema_long_period: 12,   // 12-period EMA (EMA12)
        rsi_period: 21,        // 21-period RSI (RSI21)
        rsi_threshold: 50.0,   // RSI threshold of 50
        sl_percent: 2.0,       // 2% stop loss
        tp_percent: 5.0,       // 5% take profit
    };
    
    // Create EMA+RSI strategy
    let ema_rsi_strategy = EmaRsiStrategy::new(ema_rsi_config);
    
    // Run EMA+RSI backtest on 5m timeframe
    println!("Running EMA+RSI backtest on 5m timeframe...");
    let backtest_ema_rsi = Backtest::new(market_service.clone(), ema_rsi_strategy, initial_balance);
    let result_ema_rsi = backtest_ema_rsi.run(symbol, "5m", 14400).await?;
    backtest_ema_rsi.save_result(&result_ema_rsi, "backtest_results/ema_rsi_5m_btcusdt.json")?;
    
    println!("EMA+RSI 5m Backtest completed:");
    println!("  Profit: ${:.2} ({:.2}%)", 
             result_ema_rsi.total_profit_loss, 
             result_ema_rsi.profit_loss_percent);
    println!("  Win rate: {:.2}% ({} winning, {} losing)", 
             result_ema_rsi.win_rate, 
             result_ema_rsi.winning_trades, 
             result_ema_rsi.losing_trades);
    
    Ok(())
}