use mongodb::bson::oid::ObjectId;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use super::ema_rsi::{EmaRsiConfig, EmaRsiStrategy};
use super::vwap::Signal;
use crate::auth::service::AuthService;
use crate::binance::market_service::BinanceMarketService;
use crate::binance::model::KlineSummary;
use crate::error::AppError;
use crate::paper_trading::model::{CreateOrderRequest, OrderSide, OrderType};
use crate::paper_trading::service::PaperTradingService;

#[derive(Clone)]
pub struct LiveStrategyRunner {
    market_service: BinanceMarketService,
    paper_trading_service: PaperTradingService,
    active_strategies: Arc<Mutex<HashMap<String, StrategyInstance>>>,
    kline_buffer_size: usize,
    auth_service: AuthService,
}

struct StrategyInstance {
    user_id: String,
    symbol: String,
    interval: String,
    strategy: EmaRsiStrategy,
    klines: Vec<KlineSummary>,
    last_processed_time: u64,
    task_handle: Option<JoinHandle<()>>,
}

impl LiveStrategyRunner {
    pub fn new(
        market_service: BinanceMarketService,
        paper_trading_service: PaperTradingService,
        kline_buffer_size: usize,
        auth_service: AuthService,
    ) -> Self {
        Self {
            market_service,
            paper_trading_service,
            active_strategies: Arc::new(Mutex::new(HashMap::new())),
            kline_buffer_size,
            auth_service
        }
    }

    pub async fn start_strategy_for_user(
        &self,
        user_id: &str,
        symbol: &str,
        interval: &str,
    ) -> Result<(), AppError> {
        // Check if user has paper trading enabled
        let is_paper_trading_enabled = self.is_paper_trading_enabled(user_id).await?;

        if !is_paper_trading_enabled {
            return Err(AppError::ValidationError(
                "Paper trading is not enabled for this user".to_string(),
            ));
        }

        let strategy_key = format!("{}:{}:{}", user_id, symbol, interval);

        // Check if strategy is already running
        {
            let strategies = self.active_strategies.lock().unwrap();
            if strategies.contains_key(&strategy_key) {
                return Err(AppError::ValidationError(
                    "Strategy is already running for this user, symbol and interval".to_string(),
                ));
            }
        }

        // Create EMA+RSI strategy with default configuration
        let config = EmaRsiConfig {
            ema_short_period: 5, // 5-period EMA
            ema_long_period: 12, // 12-period EMA
            rsi_period: 21,      // 21-period RSI
            rsi_threshold: 50.0, // RSI threshold of 50
            sl_percent: 2.0,     // 2% stop loss
            tp_percent: 5.0,     // 5% take profit
        };

        let strategy = EmaRsiStrategy::new(config);

        // Get historical klines to initialize the strategy
        let klines = self
            .market_service
            .get_historical_klines(symbol, interval, 50)
            .await?;

        if klines.is_empty() {
            return Err(AppError::ValidationError(
                "Could not fetch historical data to initialize strategy".to_string(),
            ));
        }

        let last_kline_time = klines.last().unwrap().close_time;

        // Create strategy instance
        let instance = StrategyInstance {
            user_id: user_id.to_string(),
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            strategy,
            klines,
            last_processed_time: last_kline_time,
            task_handle: None,
        };

        // Start the strategy monitoring task
        let (tx, rx) = mpsc::channel(100);
        let task_handle = self.spawn_strategy_task(strategy_key.clone(), rx);

        // Store the strategy instance
        {
            let mut strategies = self.active_strategies.lock().unwrap();
            let mut instance = instance;
            instance.task_handle = Some(task_handle);
            strategies.insert(strategy_key.clone(), instance);
        }

        // Subscribe to market data
        self.subscribe_to_market_data(symbol, interval, tx).await?;

        info!(
            "Started strategy for user {} on {}/{}",
            user_id, symbol, interval
        );
        Ok(())
    }

    pub async fn stop_strategy_for_user(
        &self,
        user_id: &str,
        symbol: &str,
        interval: &str,
    ) -> Result<(), AppError> {
        let strategy_key = format!("{}:{}:{}", user_id, symbol, interval);

        // Remove and get the strategy instance
        let instance = {
            let mut strategies = self.active_strategies.lock().unwrap();
            strategies.remove(&strategy_key)
        };

        // If strategy was running, stop its task
        if let Some(instance) = instance {
            if let Some(handle) = instance.task_handle {
                handle.abort();
            }
            info!(
                "Stopped strategy for user {} on {}/{}",
                user_id, symbol, interval
            );
            Ok(())
        } else {
            Err(AppError::NotFoundError(
                "No active strategy found for this configuration".to_string(),
            ))
        }
    }

    pub async fn get_active_strategies_for_user(&self, user_id: &str) -> Vec<String> {
        let strategies = self.active_strategies.lock().unwrap();
        strategies
            .iter()
            .filter(|(key, _)| key.starts_with(&format!("{}:", user_id)))
            .map(|(_, instance)| format!("{}/{}", instance.symbol, instance.interval))
            .collect()
    }

    async fn subscribe_to_market_data(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::Sender<KlineSummary>,
    ) -> Result<(), AppError> {
        // In a real implementation, you would subscribe to live kline updates
        // For now, we'll create a background task that polls for new klines
        let market_service = self.market_service.clone();
        let symbol = symbol.to_string();
        let interval = interval.to_string();

        tokio::spawn(async move {
            let mut last_time = 0;

            loop {
                // Poll for new klines every minute
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                match market_service
                    .get_historical_klines(&symbol, &interval, 5)
                    .await
                {
                    Ok(klines) => {
                        for kline in klines {
                            if kline.close_time > last_time {
                                last_time = kline.close_time;
                                if let Err(e) = tx.send(kline).await {
                                    error!("Failed to send kline update: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch klines: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    fn spawn_strategy_task(
        &self,
        strategy_key: String,
        mut rx: mpsc::Receiver<KlineSummary>,
    ) -> JoinHandle<()> {
        let active_strategies = self.active_strategies.clone();
        let paper_trading_service = self.paper_trading_service.clone();
        let buffer_size = self.kline_buffer_size;

        tokio::spawn(async move {
            while let Some(kline) = rx.recv().await {
                // Scope the mutex guard so it's released before any await points
                let process_result = {
                    let mut strategies = active_strategies.lock().unwrap();

                    if let Some(instance) = strategies.get_mut(&strategy_key) {
                        // Add new kline to the buffer
                        instance.klines.push(kline);

                        // Keep only the most recent klines
                        if instance.klines.len() > buffer_size {
                            instance.klines.remove(0);
                        }

                        // Get the close_time from the last kline
                        if let Some(last_kline) = instance.klines.last() {
                            // Check if this kline has already been processed
                            if last_kline.close_time <= instance.last_processed_time {
                                None
                            } else {
                                // Update last processed time
                                instance.last_processed_time = last_kline.close_time;

                                // Apply strategy to generate signals
                                let signals = instance.strategy.apply_strategy(&instance.klines);

                                // Get a copy of the values we need outside the lock
                                let user_id = instance.user_id.clone();
                                let symbol = instance.symbol.clone();
                                let close_price = last_kline.close.parse::<f64>().unwrap_or(0.0);

                                // Return the data we need for processing after releasing the lock
                                if let Some(signal) = signals.last() {
                                    Some((user_id, symbol, signal.clone(), close_price))
                                } else {
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                // Now the mutex guard is dropped, we can safely use await
                if let Some((user_id, symbol, signal, price)) = process_result {
                    // Execute trade based on the signal with all data already extracted
                    if let Err(e) = Self::execute_signal(
                        paper_trading_service.clone(),
                        signal,
                        user_id,
                        symbol,
                        price,
                    )
                    .await
                    {
                        error!("Failed to execute signal: {}", e);
                    }
                }
            }
        })
    }

    async fn execute_signal(
        paper_trading_service: PaperTradingService, // Take ownership instead of reference
        signal: Signal,                             // Take ownership instead of reference
        user_id: String,                            // Take ownership instead of reference
        symbol: String,                             // Take ownership instead of reference
        price: f64,
    ) -> Result<(), AppError> {
        match signal {
            Signal::Buy => {
                // Check if we already have a position
                let positions = paper_trading_service.get_positions(&user_id).await?;
                let has_position = positions.iter().any(|p| p.symbol == symbol);

                if !has_position {
                    // Calculate quantity - invest 10% of available balance
                    let balance = paper_trading_service.get_user_balance(&user_id).await?;
                    let position_size = balance * 0.1;
                    let quantity = position_size / price;

                    if quantity > 0.0 {
                        // Create buy order
                        let order_request = CreateOrderRequest {
                            symbol: symbol.to_string(),
                            side: OrderSide::Buy,
                            order_type: OrderType::Market,
                            quantity,
                        };

                        paper_trading_service
                            .create_order(&user_id, order_request)
                            .await?;
                        info!(
                            "EMA+RSI Strategy: Executed BUY for user {} on symbol {} at price {}",
                            user_id, symbol, price
                        );
                    }
                }
            }
            Signal::Sell => {
                // Check if we have a position to sell
                let positions = paper_trading_service.get_positions(&user_id).await?;
                let position = positions.iter().find(|p| p.symbol == symbol);

                if let Some(position) = position {
                    // Create sell order for the full position
                    let order_request = CreateOrderRequest {
                        symbol: symbol.to_string(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Market,
                        quantity: position.quantity,
                    };

                    paper_trading_service
                        .create_order(&user_id, order_request)
                        .await?;
                    info!(
                        "EMA+RSI Strategy: Executed SELL for user {} on symbol {} at price {}",
                        user_id, symbol, price
                    );
                }
            }
            Signal::Hold => {
                // Do nothing on HOLD signal
            }
        }

        Ok(())
    }

    async fn is_paper_trading_enabled(&self, user_id: &str) -> Result<bool, AppError> {
        match self.auth_service.get_user_by_id(user_id).await {
            Ok(user) => {
                Ok(user.paper_trading_enabled)
            }
            Err(AppError::NotFoundError(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
