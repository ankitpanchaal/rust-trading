use chrono::Utc;
use mongodb::bson::{oid::ObjectId};
use std::{str::FromStr, sync::Arc, collections::HashMap};
use tokio::sync::{RwLock, broadcast};

use crate::{
    error::AppError,
    market::service::{MarketService, PriceUpdate},
    paper_trading::{
        model::{CreateOrderRequest, OrderResponse, OrderSide, OrderType},
        service::PaperTradingService,
    },
    strategies::{
        model::{CreateStrategyRequest, Strategy, StrategyResponse, StrategyStatus, UpdateStrategyRequest},
        repository::StrategyRepository,
        indicators::{
            moving_average::MovingAverageIndicator,
            rsi::RSIIndicator,
            macd::MACDIndicator,
        },
    },
};

#[derive(Clone)]
pub struct StrategyService {
    repository: StrategyRepository,
    paper_trading_service: PaperTradingService,
    market_service: MarketService,
    // Map symbol to list of active strategy IDs
    active_strategies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // Cache of strategy data
    strategy_cache: Arc<RwLock<HashMap<String, Strategy>>>,
}

impl StrategyService {
    pub fn new(
        repository: StrategyRepository,
        paper_trading_service: PaperTradingService,
        market_service: MarketService,
    ) -> Self {
        let service = Self {
            repository,
            paper_trading_service,
            market_service: market_service.clone(),
            active_strategies: Arc::new(RwLock::new(HashMap::new())),
            strategy_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start the price listener in the background
        let service_clone = service.clone();
        tokio::spawn(async move {
            let mut rx = market_service.subscribe_to_price_updates();
            
            while let Ok(price_update) = rx.recv().await {
                if let Err(e) = service_clone.process_price_update(price_update).await {
                    eprintln!("Error processing price update: {}", e);
                }
            }
        });
        
        // Load active strategies on startup
        let service_clone = service.clone();
        tokio::spawn(async move {
            if let Err(e) = service_clone.load_active_strategies().await {
                eprintln!("Error loading active strategies: {}", e);
            }
        });
        
        service
    }

    async fn load_active_strategies(&self) -> Result<(), AppError> {
        let active_strategies = self.repository.get_active_strategies().await?;
        
        for strategy in active_strategies {
            self.cache_strategy(strategy.clone()).await?;
            
            for symbol in &strategy.symbols {
                // Subscribe to market data for this symbol
                self.market_service.subscribe_to_symbol(symbol).await?;
                
                // Add strategy to the active strategies map
                self.add_strategy_to_symbol(symbol, &strategy.id.unwrap().to_string()).await;
            }
        }
        
        Ok(())
    }

    async fn add_strategy_to_symbol(&self, symbol: &str, strategy_id: &str) {
        let mut active_strategies = self.active_strategies.write().await;
        
        if let Some(list) = active_strategies.get_mut(symbol) {
            if !list.contains(&strategy_id.to_string()) {
                list.push(strategy_id.to_string());
            }
        } else {
            active_strategies.insert(symbol.to_string(), vec![strategy_id.to_string()]);
        }
    }

    async fn remove_strategy_from_symbol(&self, symbol: &str, strategy_id: &str) {
        let mut active_strategies = self.active_strategies.write().await;
        
        if let Some(list) = active_strategies.get_mut(symbol) {
            list.retain(|id| id != strategy_id);
            
            // If no more strategies for this symbol, unsubscribe
            if list.is_empty() {
                active_strategies.remove(symbol);
                let _ = self.market_service.unsubscribe_from_symbol(symbol).await;
            }
        }
    }

    async fn update_strategy_status(
        &self, 
        strategy_id: &str,
        old_status: &StrategyStatus,
        new_status: &StrategyStatus,
        symbols: &[String]
    ) -> Result<(), AppError> {
        // If changed from active to inactive
        if matches!(old_status, StrategyStatus::Active) && !matches!(new_status, StrategyStatus::Active) {
            // Remove from active strategies
            for symbol in symbols {
                self.remove_strategy_from_symbol(symbol, strategy_id).await;
            }
            
            // Remove from cache
            let mut cache = self.strategy_cache.write().await;
            cache.remove(strategy_id);
        }
        // If changed from inactive to active
        else if !matches!(old_status, StrategyStatus::Active) && matches!(new_status, StrategyStatus::Active) {
            // Get the strategy
            let strategy = self.repository.get_strategy_by_id(strategy_id).await?
                .ok_or_else(|| AppError::NotFoundError("Strategy not found".to_string()))?;
                
            // Cache the strategy
            self.cache_strategy(strategy.clone()).await?;
            
            // Add to active strategies
            for symbol in symbols {
                // Subscribe to symbol
                self.market_service.subscribe_to_symbol(symbol).await?;
                
                // Add to active strategies
                self.add_strategy_to_symbol(symbol, strategy_id).await;
            }
        }
        
        Ok(())
    }
    
    async fn process_price_update(&self, price_update: PriceUpdate) -> Result<(), AppError> {
        let symbol = price_update.symbol;
        let price = price_update.price;
        
        // Get strategies for this symbol
        let strategies = {
            let active_strategies = self.active_strategies.read().await;
            match active_strategies.get(&symbol) {
                Some(list) => list.clone(),
                None => return Ok(()),
            }
        };
        
        // Process each strategy
        for strategy_id in strategies {
            if let Some(strategy) = self.get_cached_strategy(&strategy_id).await {
                // Get user ID
                let user_id = strategy.user_id.to_string();
                
                // Convert price to string format for compatibility
                let price_str = price.to_string();
                let timestamp = price_update.timestamp;
                
                // Update price cache for this symbol (not implemented here)
                
                // Execute strategy based on type
                match strategy.strategy_type {
                    crate::strategies::model::StrategyType::MovingAverageCrossover => {
                        self.execute_ma_crossover_strategy(&user_id, &symbol, &strategy).await?;
                    }
                    crate::strategies::model::StrategyType::RSIStrategy => {
                        self.execute_rsi_strategy(&user_id, &symbol, &strategy).await?;
                    }
                    crate::strategies::model::StrategyType::MACDStrategy => {
                        self.execute_macd_strategy(&user_id, &symbol, &strategy).await?;
                    }
                }
                
                // Update last executed time
                let mut updated_strategy = strategy.clone();
                updated_strategy.last_executed_at = Some(Utc::now());
                self.repository.update_strategy(&updated_strategy).await?;
                
                // Update cache
                self.cache_strategy(updated_strategy).await?;
            }
        }
        
        Ok(())
    }

    async fn cache_strategy(&self, strategy: Strategy) -> Result<(), AppError> {
        if let Some(id) = &strategy.id {
            let mut cache = self.strategy_cache.write().await;
            cache.insert(id.to_string(), strategy);
        }
        Ok(())
    }

    async fn get_cached_strategy(&self, strategy_id: &str) -> Option<Strategy> {
        let cache = self.strategy_cache.read().await;
        cache.get(strategy_id).cloned()
    }

    pub async fn create_strategy(
        &self,
        user_id: &str,
        req: CreateStrategyRequest,
    ) -> Result<StrategyResponse, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        let now = Utc::now();
        let strategy = Strategy {
            id: None,
            user_id: user_id_obj,
            name: req.name,
            description: req.description,
            strategy_type: req.strategy_type,
            status: StrategyStatus::Paused, // Start paused by default
            symbols: req.symbols,
            parameters: req.parameters,
            risk_parameters: req.risk_parameters,
            created_at: now,
            updated_at: now,
            last_executed_at: None,
        };

        let created_strategy = self.repository.create_strategy(strategy).await?;
        Ok(StrategyResponse::from(created_strategy))
    }

    pub async fn update_strategy(
        &self,
        user_id: &str,
        strategy_id: &str,
        req: UpdateStrategyRequest,
    ) -> Result<StrategyResponse, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        // Get the existing strategy
        let strategy_opt = self.repository.get_strategy_by_id(strategy_id).await?;
        let mut strategy = match strategy_opt {
            Some(s) if s.user_id == user_id_obj => s,
            Some(_) => return Err(AppError::AuthorizationError("You don't own this strategy".to_string())),
            None => return Err(AppError::NotFoundError("Strategy not found".to_string())),
        };

        let old_status = strategy.status.clone();
        let old_symbols = strategy.symbols.clone();
        
        // Update fields if provided
        if let Some(name) = req.name {
            strategy.name = name;
        }
        if let Some(description) = req.description {
            strategy.description = description;
        }
        if let Some(symbols) = req.symbols.clone() {
            strategy.symbols = symbols;
        }
        if let Some(parameters) = req.parameters {
            strategy.parameters = parameters;
        }
        if let Some(risk_parameters) = req.risk_parameters {
            strategy.risk_parameters = risk_parameters;
        }
        
        // Update status last - need to handle subscriptions
        let mut status_changed = false;
        if let Some(status) = req.status {
            if !std::mem::discriminant(&status).eq(&std::mem::discriminant(&strategy.status)) {
                status_changed = true;
            }
            strategy.status = status;
        }

        strategy.updated_at = Utc::now();

        // Save the updated strategy
        self.repository.update_strategy(&strategy).await?;
        
        // Handle status changes
        if status_changed {
            self.update_strategy_status(
                strategy_id, 
                &old_status, 
                &strategy.status,
                &strategy.symbols
            ).await?;
        }
        // If symbols changed but status is active, update subscriptions
        else if old_symbols != strategy.symbols && matches!(strategy.status, StrategyStatus::Active) {
            // Remove old symbol subscriptions
            for symbol in &old_symbols {
                if !strategy.symbols.contains(symbol) {
                    self.remove_strategy_from_symbol(symbol, strategy_id).await;
                }
            }
            
            // Add new symbol subscriptions
            for symbol in &strategy.symbols {
                if !old_symbols.contains(symbol) {
                    self.market_service.subscribe_to_symbol(symbol).await?;
                    self.add_strategy_to_symbol(symbol, strategy_id).await;
                }
            }
            
            // Update cache
            self.cache_strategy(strategy.clone()).await?;
        }
        
        Ok(StrategyResponse::from(strategy))
    }

    pub async fn get_strategy(
        &self,
        user_id: &str,
        strategy_id: &str,
    ) -> Result<StrategyResponse, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        let strategy_opt = self.repository.get_strategy_by_id(strategy_id).await?;
        match strategy_opt {
            Some(s) if s.user_id == user_id_obj => Ok(StrategyResponse::from(s)),
            Some(_) => Err(AppError::AuthorizationError("You don't own this strategy".to_string())),
            None => Err(AppError::NotFoundError("Strategy not found".to_string())),
        }
    }

    pub async fn get_user_strategies(&self, user_id: &str) -> Result<Vec<StrategyResponse>, AppError> {
        let strategies = self.repository.get_strategies_by_user_id(user_id).await?;
        Ok(strategies.into_iter().map(StrategyResponse::from).collect())
    }

    pub async fn delete_strategy(&self, user_id: &str, strategy_id: &str) -> Result<bool, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        let strategy_opt = self.repository.get_strategy_by_id(strategy_id).await?;
        match strategy_opt {
            Some(s) if s.user_id == user_id_obj => self.repository.delete_strategy(strategy_id).await,
            Some(_) => Err(AppError::AuthorizationError("You don't own this strategy".to_string())),
            None => Err(AppError::NotFoundError("Strategy not found".to_string())),
        }
    }

    pub async fn execute_strategies(&self) -> Result<(), AppError> {
        // Get all active strategies
        let active_strategies = self.repository.get_active_strategies().await?;
        
        for strategy in active_strategies {
            let user_id = strategy.user_id.to_string();
            
            // Check each symbol in the strategy
            for symbol in &strategy.symbols {
                match strategy.strategy_type {
                    crate::strategies::model::StrategyType::MovingAverageCrossover => {
                        self.execute_ma_crossover_strategy(&user_id, &symbol, &strategy).await?;
                    }
                    crate::strategies::model::StrategyType::RSIStrategy => {
                        self.execute_rsi_strategy(&user_id, &symbol, &strategy).await?;
                    }
                    crate::strategies::model::StrategyType::MACDStrategy => {
                        self.execute_macd_strategy(&user_id, &symbol, &strategy).await?;
                    }
                }
            }

            // Update last execution time
            let mut updated_strategy = strategy;
            updated_strategy.last_executed_at = Some(Utc::now());
            self.repository.update_strategy(&updated_strategy).await?;
        }

        Ok(())
    }

    async fn execute_ma_crossover_strategy(
        &self, 
        user_id: &str, 
        symbol: &str, 
        strategy: &Strategy
    ) -> Result<(), AppError> {
        // Extract parameters
        let fast_ma_period = strategy.parameters["fastMAPeriod"]
            .as_u64()
            .unwrap_or(9) as usize;
        let slow_ma_period = strategy.parameters["slowMAPeriod"]
            .as_u64()
            .unwrap_or(21) as usize;
        
        // Get historical prices (simplified - in a real system, you would fetch more data)
        let price_data = self.get_historical_prices(symbol, 100).await?;
        
        // Calculate indicators
        let ma_indicator = MovingAverageIndicator::new();
        let fast_ma = ma_indicator.calculate_sma(&price_data, fast_ma_period);
        let slow_ma = ma_indicator.calculate_sma(&price_data, slow_ma_period);
        
        // Check for signals
        if fast_ma.len() < 2 || slow_ma.len() < 2 {
            return Ok(());
        }
        
        let current_fast = fast_ma[fast_ma.len() - 1];
        let prev_fast = fast_ma[fast_ma.len() - 2];
        let current_slow = slow_ma[slow_ma.len() - 1];
        let prev_slow = slow_ma[slow_ma.len() - 2];
        
        // Check for crossover (bullish)
        if prev_fast <= prev_slow && current_fast > current_slow {
            // Generate buy signal
            self.place_order(user_id, symbol, OrderSide::Buy, strategy).await?;
        }
        // Check for crossover (bearish)
        else if prev_fast >= prev_slow && current_fast < current_slow {
            // Generate sell signal
            self.place_order(user_id, symbol, OrderSide::Sell, strategy).await?;
        }
        
        Ok(())
    }

    async fn execute_rsi_strategy(
        &self, 
        user_id: &str, 
        symbol: &str, 
        strategy: &Strategy
    ) -> Result<(), AppError> {
        // Extract parameters
        let rsi_period = strategy.parameters["rsiPeriod"]
            .as_u64()
            .unwrap_or(14) as usize;
        let oversold_threshold = strategy.parameters["oversoldThreshold"]
            .as_f64()
            .unwrap_or(30.0);
        let overbought_threshold = strategy.parameters["overboughtThreshold"]
            .as_f64()
            .unwrap_or(70.0);
        
        // Get historical prices
        let price_data = self.get_historical_prices(symbol, 100).await?;
        
        // Calculate RSI
        let rsi_indicator = RSIIndicator::new();
        let rsi_values = rsi_indicator.calculate(&price_data, rsi_period);
        
        if rsi_values.len() < 2 {
            return Ok(());
        }
        
        let current_rsi = rsi_values[rsi_values.len() - 1];
        let previous_rsi = rsi_values[rsi_values.len() - 2];
        
        // Oversold -> Buy signal
        if previous_rsi < oversold_threshold && current_rsi > oversold_threshold {
            self.place_order(user_id, symbol, OrderSide::Buy, strategy).await?;
        }
        // Overbought -> Sell signal
        else if previous_rsi > overbought_threshold && current_rsi < overbought_threshold {
            self.place_order(user_id, symbol, OrderSide::Sell, strategy).await?;
        }
        
        Ok(())
    }

    async fn execute_macd_strategy(
        &self, 
        user_id: &str, 
        symbol: &str, 
        strategy: &Strategy
    ) -> Result<(), AppError> {
        // Extract parameters
        let fast_period = strategy.parameters["fastPeriod"]
            .as_u64()
            .unwrap_or(12) as usize;
        let slow_period = strategy.parameters["slowPeriod"]
            .as_u64()
            .unwrap_or(26) as usize;
        let signal_period = strategy.parameters["signalPeriod"]
            .as_u64()
            .unwrap_or(9) as usize;
        
        // Get historical prices
        let price_data = self.get_historical_prices(symbol, 100).await?;
        
        // Calculate MACD
        let macd_indicator = MACDIndicator::new();
        let (macd_line, signal_line, _) = macd_indicator.calculate(
            &price_data, fast_period, slow_period, signal_period
        );
        
        if macd_line.len() < 2 || signal_line.len() < 2 {
            return Ok(());
        }
        
        let current_macd = macd_line[macd_line.len() - 1];
        let prev_macd = macd_line[macd_line.len() - 2];
        let current_signal = signal_line[signal_line.len() - 1];
        let prev_signal = signal_line[signal_line.len() - 2];
        
        // MACD crosses above signal line (bullish)
        if prev_macd <= prev_signal && current_macd > current_signal {
            self.place_order(user_id, symbol, OrderSide::Buy, strategy).await?;
        }
        // MACD crosses below signal line (bearish)
        else if prev_macd >= prev_signal && current_macd < current_signal {
            self.place_order(user_id, symbol, OrderSide::Sell, strategy).await?;
        }
        
        Ok(())
    }

    async fn get_historical_prices(&self, symbol: &str, bars: usize) -> Result<Vec<f64>, AppError> {
        self.market_service.get_historical_klines(symbol, "1m", bars).await
    }

    async fn place_order(
        &self,
        user_id: &str,
        symbol: &str,
        side: OrderSide,
        strategy: &Strategy,
    ) -> Result<OrderResponse, AppError> {
        // Get current price
        let (price_str, _) = self.market_service.get_ticker_price(symbol).await?;
        let current_price = price_str.parse::<f64>().map_err(|_| {
            AppError::InternalError(format!("Failed to parse price: {}", price_str))
        })?;
        
        // Calculate position size based on risk parameters
        let user_balance = self.paper_trading_service.get_user_balance(user_id).await?;
        let risk_amount = user_balance * 0.02; // Risk 2% of balance by default
        
        // Get position size from strategy parameters or use default
        let position_size = strategy.risk_parameters.max_position_size;
        
        // Calculate quantity
        let quantity = position_size / current_price;
        
        // Create order request
        let order_request = CreateOrderRequest {
            symbol: symbol.to_string(),
            order_type: OrderType::Market,
            side: side.clone(),
            quantity,
        };
        
        // Place the order
        let order_response = self.paper_trading_service.create_order(user_id, order_request).await?;
        
        // If this is a buy order, set up stop loss and take profit orders
        if matches!(side, OrderSide::Buy) {  // Using matches! instead of == for enum comparison
            // Set stop loss
            let stop_loss_price = current_price * (1.0 - strategy.risk_parameters.stop_loss_percentage / 100.0);
            
            // Set take profit
            let take_profit_price = current_price * (1.0 + strategy.risk_parameters.take_profit_percentage / 100.0);
            
            // Here you would place conditional orders for SL and TP
            // In a real implementation, these would be separate orders with appropriate types
            // For now, we'll just log the intentions
            println!("Setting stop loss at {} for {} {}", stop_loss_price, symbol, order_response.id);
            println!("Setting take profit at {} for {} {}", take_profit_price, symbol, order_response.id);
        }
        
        Ok(order_response)
    }
}