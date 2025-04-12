use crate::binance::market_service::BinanceMarketService;
use crate::strategy::vwap::{VwapConfig, VwapStrategy};
use crate::strategy::backtest::Backtest;

// Function to run backtest
pub async fn run_backtest() -> anyhow::Result<()> {
    // Create market service
    let market_service = BinanceMarketService::new();
    
    // Create VWAP strategy configs for different timeframes
    let vwap_5m_config = VwapConfig {
        period: 20,           // 20-period VWAP
        sl_percent: 2.0,      // 2% stop loss
        tp_percent: 5.0,      // 5% take profit
    };
    
    
    // Create strategies
    let vwap_5m = VwapStrategy::new(vwap_5m_config);
    
    // Initial balance for backtest
    let initial_balance = 10000.0;
    
    // Run 5m backtest
    println!("Running VWAP backtest on 5m timeframe...");
    let backtest_5m = Backtest::new(market_service.clone(), vwap_5m, initial_balance);
    let result_5m = backtest_5m.run("BTCUSDT", "5m", 100).await?;
    backtest_5m.save_result(&result_5m, "backtest_results/vwap_5m_btcusdt.json")?;
    
    println!("5m Backtest completed:");
    println!("  Profit: ${:.2} ({:.2}%)", result_5m.total_profit_loss, result_5m.profit_loss_percent);
    println!("  Win rate: {:.2}% ({} winning, {} losing)", 
             result_5m.win_rate, result_5m.winning_trades, result_5m.losing_trades);
    
    Ok(())
}