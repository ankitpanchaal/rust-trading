use chrono::Utc;
use mongodb::bson::{doc, oid::ObjectId};
use std::str::FromStr;

use crate::auth::model::User;
use crate::error::AppError;
use crate::market::service::MarketService;
use crate::paper_trading::model::{
    CreateOrderRequest, Order, OrderResponse, OrderSide, OrderStatus, OrderType, Position,
    PositionResponse, TradingStatsResponse,
};
use crate::paper_trading::repository::PaperTradingRepository;

#[derive(Clone)]
pub struct PaperTradingService {
    repository: PaperTradingRepository,
    market_service: MarketService,
}

impl PaperTradingService {
    pub fn new(repository: PaperTradingRepository, market_service: MarketService) -> Self {
        Self {
            repository,
            market_service,
        }
    }

    // User management
    pub async fn enable_paper_trading(
        &self,
        user_id: &str,
        initial_balance: f64,
    ) -> Result<User, AppError> {
        self.repository
            .enable_paper_trading(user_id, initial_balance)
            .await
    }

    // Order processing
    pub async fn create_order(
        &self,
        user_id: &str,
        req: CreateOrderRequest,
    ) -> Result<OrderResponse, AppError> {
        // Validate user and check if paper trading is enabled
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        // Get current market price for the symbol
        let (price_str, _) = self.market_service.get_ticker_price(&req.symbol).await?;
        let price = price_str.parse::<f64>().map_err(|_| {
            AppError::InternalError(format!("Failed to parse price: {}", price_str))
        })?;

        // Calculate order cost
        let order_cost = price * req.quantity;

        // Get user balance
        let user_balance = self.repository.get_user_balance(user_id).await?;

        // Create a simple order
        let now = Utc::now();
        let mut order = Order {
            id: None,
            user_id: user_id_obj,
            symbol: req.symbol.clone(),
            order_type: req.order_type,
            side: req.side.clone(),
            quantity: req.quantity,
            price: Some(price),
            status: OrderStatus::Filled, // Market orders are filled immediately
            position_id: None,
            created_at: now,
            updated_at: now,
            filled_at: Some(now),
        };

        // Process order based on side
        match req.side {
            OrderSide::Buy => {
                // Check if user has enough balance
                if order_cost > user_balance {
                    return Err(AppError::ValidationError(
                        "Insufficient balance for this order".to_string(),
                    ));
                }
                
                // Update user balance
                let new_balance = user_balance - order_cost;
                self.repository.update_user_balance(user_id_obj, new_balance).await?;
                
                // Create or update position
                let position = self.update_position_for_buy_order(&order, price).await?;
                order.position_id = position.id;
            }
            OrderSide::Sell => {
                // Check if user has the position to sell
                let position_opt = self
                    .repository
                    .get_position_by_user_and_symbol(&user_id_obj, &req.symbol)
                    .await?;
                
                let position = match position_opt {
                    Some(pos) => {
                        if pos.quantity < req.quantity {
                            return Err(AppError::ValidationError(
                                format!("Insufficient quantity to sell: have {}, requested {}", 
                                    pos.quantity, req.quantity)
                            ));
                        }
                        pos
                    }
                    None => {
                        return Err(AppError::ValidationError(
                            format!("No position found for symbol {}", req.symbol)
                        ));
                    }
                };
                
                // Update user balance
                let new_balance = user_balance + order_cost;
                self.repository.update_user_balance(user_id_obj, new_balance).await?;
                
                // Update position
                let updated_position = self.update_position_for_sell_order(&order, &position, price).await?;
                order.position_id = updated_position.map(|p| p.id).flatten();
            }
        }
        
        // Save order
        let created_order = self.repository.create_order(order).await?;
        
        Ok(OrderResponse::from(created_order))
    }

    // Helper method to update position for buy orders
    async fn update_position_for_buy_order(&self, order: &Order, price: f64) -> Result<Position, AppError> {
        let position_opt = self
            .repository
            .get_position_by_user_and_symbol(&order.user_id, &order.symbol)
            .await?;
            
        match position_opt {
            Some(mut position) => {
                // Update existing position
                let total_quantity = position.quantity + order.quantity;
                let total_cost = (position.quantity * position.entry_price) + (order.quantity * price);
                position.entry_price = total_cost / total_quantity;
                position.quantity = total_quantity;
                position.current_price = price;
                position.updated_at = Utc::now();
                
                self.repository.update_position(&position).await?;
                Ok(position)
            }
            None => {
                // Create new position
                let new_position = Position {
                    id: None,
                    user_id: order.user_id,
                    symbol: order.symbol.clone(),
                    quantity: order.quantity,
                    entry_price: price,
                    current_price: price,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                    side: OrderSide::Buy,
                    opened_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                
                self.repository.create_position(new_position).await
            }
        }
    }
    
    // Helper method to update position for sell orders
    async fn update_position_for_sell_order(
        &self, 
        order: &Order, 
        position: &Position, 
        price: f64
    ) -> Result<Option<Position>, AppError> {
        let realized_pnl = (price - position.entry_price) * order.quantity;
        
        if position.quantity == order.quantity {
            // Close position completely
            if let Some(position_id) = &position.id {
                self.repository.delete_position(position_id).await?;
            } else {
                return Err(AppError::ValidationError("Position ID not found".to_string()));
            }
            return Ok(None);
        } else {
            // Reduce position
            let mut updated_position = position.clone();
            updated_position.quantity -= order.quantity;
            updated_position.realized_pnl += realized_pnl;
            updated_position.updated_at = Utc::now();
            
            self.repository.update_position(&updated_position).await?;
            return Ok(Some(updated_position));
        }
    }

    // Position management
    pub async fn get_positions(&self, user_id: &str) -> Result<Vec<PositionResponse>, AppError> {
        let positions = self.repository.get_positions_by_user_id(user_id).await?;
        
        // Update current prices and unrealized PnL
        let mut position_responses = Vec::new();
        
        for mut position in positions {
            // Get current price
            let (price_str, _) = self.market_service.get_ticker_price(&position.symbol).await?;
            let price = price_str.parse::<f64>().map_err(|_| {
                AppError::InternalError(format!("Failed to parse price: {}", price_str))
            })?;
            
            // Update position price and PnL
            position.current_price = price;
            position.unrealized_pnl = (price - position.entry_price) * position.quantity;
            
            // Save updates to database
            if let Some(_) = position.id {
                self.repository.update_position(&position).await?;
            }
            
            position_responses.push(PositionResponse::from(position));
        }
    
        Ok(position_responses)
    }

    pub async fn get_orders(&self, user_id: &str) -> Result<Vec<OrderResponse>, AppError> {
        let orders = self.repository.get_orders_by_user_id(user_id).await?;
        let responses = orders.into_iter().map(OrderResponse::from).collect();
        Ok(responses)
    }

    // Get balance information
    pub async fn get_user_balance_details(&self, user_id: &str) -> Result<serde_json::Value, AppError> {
        // Get user balance
        let balance = self.repository.get_user_balance(user_id).await?;

        // Get user to get initial balance
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;
        let users_collection = self.repository.db.collection("users");
        let user_doc = users_collection
            .find_one(doc! { "_id": user_id_obj }, None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("User not found".to_string()))?;

        let user: User = bson::from_document(user_doc)
            .map_err(|e| AppError::InternalError(format!("Failed to deserialize user: {}", e)))?;

        // Calculate total position value and unrealized PnL
        let positions = self.repository.get_positions_by_user_id(user_id).await?;
        let mut total_position_value = 0.0;
        let mut unrealized_pnl = 0.0;

        for position in positions {
            let (price_str, _) = self.market_service.get_ticker_price(&position.symbol).await?;
            let current_price = price_str.parse::<f64>().map_err(|_| {
                AppError::InternalError(format!("Failed to parse price: {}", price_str))
            })?;
            
            let position_value = position.quantity * current_price;
            let position_pnl = (current_price - position.entry_price) * position.quantity;
            
            total_position_value += position_value;
            unrealized_pnl += position_pnl;
        }

        // Calculate total account value and performance
        let total_account_value = balance + total_position_value;
        let initial_balance = user.initial_paper_balance_usd;
        let performance = (total_account_value - initial_balance) / initial_balance * 100.0;

        Ok(serde_json::json!({
            "cash_balance": balance,
            "position_value": total_position_value,
            "unrealized_pnl": unrealized_pnl,
            "total_account_value": total_account_value,
            "initial_balance": initial_balance,
            "performance_percentage": performance
        }))
    }

    // Simplified trading stats
    pub async fn get_trading_stats(&self, user_id: &str) -> Result<TradingStatsResponse, AppError> {
        // Get user balance and positions
        let user_balance = self.repository.get_user_balance(user_id).await?;
        
        // Get user for initial balance
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;
        let users_collection = self.repository.db.collection("users");
        let user_doc = users_collection
            .find_one(doc! { "_id": user_id_obj }, None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("User not found".to_string()))?;

        let user: User = bson::from_document(user_doc)
            .map_err(|e| AppError::InternalError(format!("Failed to deserialize user: {}", e)))?;

        // Get positions and calculate unrealized PnL
        let positions = self.repository.get_positions_by_user_id(user_id).await?;
        let mut unrealized_pnl = 0.0;

        for position in positions {
            let (price_str, _) = self.market_service.get_ticker_price(&position.symbol).await?;
            let current_price = price_str.parse::<f64>().map_err(|_| {
                AppError::InternalError(format!("Failed to parse price: {}", price_str))
            })?;
            
            unrealized_pnl += (current_price - position.entry_price) * position.quantity;
        }

        // Get orders for basic trade statistics
        let orders = self.repository.get_orders_by_user_id(user_id).await?;
        let total_trades = orders.len() as u32;

        // Calculate basic performance metrics
        let initial_balance = user.initial_paper_balance_usd;
        let current_total = user_balance + unrealized_pnl;
        let total_pnl = current_total - initial_balance;
        let pnl_percentage = (total_pnl / initial_balance) * 100.0;

        Ok(TradingStatsResponse {
            total_trades,
            winning_trades: 0, // Simplified - not tracking individual trade outcome
            losing_trades: 0,  // Simplified - not tracking individual trade outcome
            win_rate: 0.0,     // Simplified - not tracking individual trade outcome
            total_pnl,
            pnl_percentage,
            average_profit: 0.0, // Simplified - not calculating detailed metrics
            average_loss: 0.0,   // Simplified - not calculating detailed metrics
            risk_reward_ratio: 0.0, // Simplified - not calculating detailed metrics
            current_balance: user_balance,
        })
    }
}