use chrono::Utc;
use mongodb::bson::{oid::ObjectId};
use std::str::FromStr;

use crate::{
    error::AppError,
    market::service::MarketService,
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
}

impl StrategyService {
    pub fn new(
        repository: StrategyRepository,
        paper_trading_service: PaperTradingService,
        market_service: MarketService,
    ) -> Self {
        Self {
            repository,
            paper_trading_service,
            market_service,
        }
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

        // Update fields if provided
        if let Some(name) = req.name {
            strategy.name = name;
        }
        if let Some(description) = req.description {
            strategy.description = description;
        }
        if let Some(status) = req.status {
            strategy.status = status;
        }
        if let Some(symbols) = req.symbols {
            strategy.symbols = symbols;
        }
        if let Some(parameters) = req.parameters {
            strategy.parameters = parameters;
        }
        if let Some(risk_parameters) = req.risk_parameters {
            strategy.risk_parameters = risk_parameters;
        }

        strategy.updated_at = Utc::now();

        // Save the updated strategy
        self.repository.update_strategy(&strategy).await?;
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
        // In a real implementation, you would fetch historical price data from your market service
        // For now, we'll just use the current price and simulate historical data
        let (price_str, _) = self.market_service.get_ticker_price(symbol).await?;
        let current_price = price_str.parse::<f64>().map_err(|_| {
            AppError::InternalError(format!("Failed to parse price: {}", price_str))
        })?;
        
        // Simulate some random historical data
        let mut prices = Vec::with_capacity(bars);
        let mut price = current_price;
        
        for _ in 0..bars {
            prices.push(price);
            // Add some random movement
            let change = price * 0.01 * (rand::random::<f64>() - 0.5);
            price += change;
        }
        
        // Reverse to get chronological order (oldest first)
        prices.reverse();
        Ok(prices)
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