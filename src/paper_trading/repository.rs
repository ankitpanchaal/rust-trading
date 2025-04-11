use futures::stream::TryStreamExt;
use mongodb::bson::{self, doc, oid::ObjectId, Document};
use std::str::FromStr;

use crate::{
    auth::model::User,
    db::MongoDb,
    error::AppError,
    market::service::MarketService,
    telegram::service::TelegramService,
    telegram::repository::TelegramRepository
};

use super::model::{Order, Position, OrderSide};

#[derive(Clone)]
pub struct PaperTradingRepository {
    pub db: MongoDb,
    pub tg_service  : TelegramService,
}

impl PaperTradingRepository {
    pub fn new(db: MongoDb, _market_service: MarketService) -> Self {
        let tg_repository = TelegramRepository::new(db.clone());
        let tg_service = TelegramService::new(tg_repository, None)
            .expect("Failed to create TelegramService");
        Self { db, tg_service }
    }

    // User-related methods
    pub async fn enable_paper_trading(&self, user_id: &str, initial_balance: f64) -> Result<User, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        let users_collection = self.db.collection("users");

        // Check if user exists
        let filter = doc! { "_id": user_id_obj };
        let _user_doc = users_collection
            .find_one(filter.clone(), None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("User not found".to_string()))?;

        // Enable paper trading and set initial balance
        let update = doc! {
            "$set": {
                "paper_trading_enabled": true,
                "initial_paper_balance_usd": initial_balance,
                "paper_balance_usd": initial_balance
            }
        };

        users_collection.update_one(filter, update, None).await?;

        // Get updated user
        let updated_user_doc = users_collection
            .find_one(doc! { "_id": user_id_obj }, None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("User not found".to_string()))?;

        let user: User = bson::from_document(updated_user_doc)
            .map_err(|e| AppError::InternalError(format!("Failed to deserialize user: {}", e)))?;
            
        // Send notification about paper trading being enabled
        let message = format!(
            "🎮 Paper trading enabled!\nInitial balance: *${:.2}*\n\nYou'll now receive notifications about your paper trading activities.",
            initial_balance
        );
        
        if let Err(e) = self.tg_service.send_message_to_user(user_id, &message).await {
            tracing::warn!("Failed to send paper trading activation notification: {}", e);
        }

        Ok(user)
    }

    pub async fn get_user_balance(&self, user_id: &str) -> Result<f64, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        let users_collection = self.db.collection("users");
        let user_doc = users_collection
            .find_one(doc! { "_id": user_id_obj }, None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("User not found".to_string()))?;

        let paper_balance = user_doc
            .get("paper_balance_usd")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);

        Ok(paper_balance)
    }

    // Order-related methods
    pub async fn create_order(&self, order: Order) -> Result<Order, AppError> {
        let orders_collection = self.db.collection("paper_trading_orders");
        
        // Convert Order to Document
        let order_doc = bson::to_document(&order)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize order: {}", e)))?;
        
        // Insert the new order
        let insert_result = orders_collection.insert_one(order_doc, None).await?;
        
        // Get the inserted ID
        let id = insert_result
            .inserted_id
            .as_object_id()
            .ok_or_else(|| AppError::InternalError("Failed to get inserted order ID".to_string()))?;
        
        // Return the complete order with ID
        let mut order_with_id = order;
        order_with_id.id = Some(id);

        // Send notification to user
        let user_id = order_with_id.user_id.to_hex();
        let side_str = match order_with_id.side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        };
        
        let price_str = match order_with_id.price {
            Some(price) => format!("${:.2}", price),
            None => "market price".to_string(),
        };
        
        let message = format!(
            "🔔 New order placed:\n*{}* {} {} of *{}* @ {}", 
            side_str, 
            order_with_id.quantity, 
            side_str.to_lowercase(), 
            order_with_id.symbol,
            price_str
        );
        
        // Send notification but don't fail if it doesn't work
        if let Err(e) = self.tg_service.send_message_to_user(&user_id, &message).await {
            // Log error but don't propagate it
            tracing::warn!("Failed to send order notification: {}", e);
        }
        
        Ok(order_with_id)
    }

    pub async fn get_orders_by_user_id(&self, user_id: &str) -> Result<Vec<Order>, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;
        
        let orders_collection = self.db.collection("paper_trading_orders");
        
        let cursor = orders_collection
            .find(doc! { "user_id": user_id_obj }, None)
            .await?;
        
        // Use try_collect from futures::stream::TryStreamExt instead of collect+manual error handling
        let orders: Vec<Document> = cursor.try_collect().await?;
        
        // Convert documents to Order objects
        let orders = orders
            .into_iter()
            .map(|doc| {
                bson::from_document::<Order>(doc)
                    .map_err(|e| AppError::InternalError(format!("Failed to deserialize order: {}", e)))
            })
            .collect::<Result<Vec<Order>, AppError>>()?;
        
        Ok(orders)
    }

    // Position-related methods
    pub async fn create_position(&self, position: Position) -> Result<Position, AppError> {
        let positions_collection = self.db.collection("paper_trading_positions");
        
        // Convert Position to Document
        let position_doc = bson::to_document(&position)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize position: {}", e)))?;
        
        // Insert the new position
        let insert_result = positions_collection.insert_one(position_doc, None).await?;
        
        // Get the inserted ID
        let id = insert_result
            .inserted_id
            .as_object_id()
            .ok_or_else(|| AppError::InternalError("Failed to get inserted position ID".to_string()))?;
        
        // Return the complete position with ID
        let mut position_with_id = position;
        position_with_id.id = Some(id);

        // Send notification to user
        let user_id = position_with_id.user_id.to_hex();
        let position_type = if position_with_id.quantity > 0.0 { "LONG" } else { "SHORT" };
        let quantity = position_with_id.quantity.abs();
        
        let message = format!(
            "🟢 New position opened:\n*{}* position in *{}*\nQuantity: {}\nEntry price: ${:.2}", 
            position_type,
            position_with_id.symbol,
            quantity,
            position_with_id.entry_price
        );
        
        if let Err(e) = self.tg_service.send_message_to_user(&user_id, &message).await {
            tracing::warn!("Failed to send position creation notification: {}", e);
        }
        
        Ok(position_with_id)
    }

    pub async fn update_position(&self, position: &Position) -> Result<(), AppError> {
        let position_id = position.id.ok_or_else(|| {
            AppError::ValidationError("Position ID is required for update".to_string())
        })?;
        
        // Get current position to compare changes
        let positions_collection = self.db.collection("paper_trading_positions");
        let filter = doc! { "_id": position_id };
        
        // Get the current state before updating
        let current_position_doc = positions_collection.find_one(filter.clone(), None).await?;
        
        // Update the position
        let position_doc = bson::to_document(position)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize position: {}", e)))?;
        
        positions_collection
            .replace_one(filter, position_doc, None)
            .await?;
        
        // Send notification to user about position update
        let user_id = position.user_id.to_hex();
        let position_type = if position.quantity > 0.0 { "LONG" } else { "SHORT" };
        
        // Calculate PnL if we have the current position data
        if let Some(current_doc) = current_position_doc {
            if let Ok(current_position) = bson::from_document::<Position>(current_doc) {
                let pnl_change = position.unrealized_pnl - current_position.unrealized_pnl;
                let pnl_emoji = if pnl_change >= 0.0 { "📈" } else { "📉" };
                
                let message = format!(
                    "🔄 Position updated:\n*{}* position in *{}*\nQuantity: {}\nCurrent price: ${:.2}\nUnrealized P&L: ${:.2} ({}${:.2})", 
                    position_type,
                    position.symbol,
                    position.quantity.abs(),
                    position.current_price,
                    position.unrealized_pnl,
                    pnl_emoji,
                    pnl_change.abs()
                );
                
                if let Err(e) = self.tg_service.send_message_to_user(&user_id, &message).await {
                    tracing::warn!("Failed to send position update notification: {}", e);
                }
            }
        }
        
        Ok(())
    }

    pub async fn delete_position(&self, position_id: &ObjectId) -> Result<(), AppError> {
        let positions_collection = self.db.collection("paper_trading_positions");
        
        // Get position details before deleting
        let position_doc = positions_collection
            .find_one(doc! { "_id": position_id }, None)
            .await?;
            
        if let Some(doc) = position_doc {
            let position: Position = bson::from_document(doc)
                .map_err(|e| AppError::InternalError(format!("Failed to deserialize position: {}", e)))?;
                
            // Delete the position
            positions_collection
                .delete_one(doc! { "_id": position_id }, None)
                .await?;
            
            // Send notification
            let user_id = position.user_id.to_hex();
            let position_type = if position.quantity > 0.0 { "LONG" } else { "SHORT" };
            let pnl_emoji = if position.realized_pnl >= 0.0 { "🟩" } else { "🟥" };
            
            let message = format!(
                "🔴 Position closed:\n*{}* position in *{}*\nQuantity: {}\nEntry price: ${:.2}\nExit price: ${:.2}\nRealized P&L: {}${:.2}", 
                position_type,
                position.symbol,
                position.quantity.abs(),
                position.entry_price,
                position.current_price,
                pnl_emoji,
                position.realized_pnl.abs()
            );
            
            if let Err(e) = self.tg_service.send_message_to_user(&user_id, &message).await {
                tracing::warn!("Failed to send position closure notification: {}", e);
            }
        } else {
            // If position not found, just delete (idempotent)
            positions_collection
                .delete_one(doc! { "_id": position_id }, None)
                .await?;
        }
        
        Ok(())
    }

    // Update update_user_balance method to send notification
    pub async fn update_user_balance(&self, user_id: ObjectId, new_balance: f64) -> Result<(), AppError> {
        let users_collection = self.db.collection("users");
        let filter = doc! { "_id": user_id.clone() };
        
        // Get current balance before updating
        let user_doc = users_collection
            .find_one(filter.clone(), None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("User not found".to_string()))?;
        
        let old_balance = user_doc
            .get("paper_balance_usd")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        
        // Update the balance
        let update = doc! { "$set": { "paper_balance_usd": new_balance } };
        users_collection.update_one(filter, update, None).await?;
        
        // Send notification about balance change
        let balance_diff = new_balance - old_balance;
        let user_id_str = user_id.to_hex();
        let emoji = if balance_diff >= 0.0 { "💰" } else { "📉" };
        let sign = if balance_diff >= 0.0 { "+" } else { "" }; // Minus sign is included in the number
        
        let message = format!(
            "{} Balance updated:\nNew balance: *${:.2}*\nChange: *{}${:.2}*",
            emoji,
            new_balance,
            sign,
            balance_diff
        );
        
        if let Err(e) = self.tg_service.send_message_to_user(&user_id_str, &message).await {
            tracing::warn!("Failed to send balance update notification: {}", e);
        }
        
        Ok(())
    }

    pub async fn get_position_by_user_and_symbol(&self, user_id: &ObjectId, symbol: &str) -> Result<Option<Position>, AppError> {
        let positions_collection = self.db.collection("paper_trading_positions");
        
        let position_doc = positions_collection
            .find_one(doc! { "user_id": user_id, "symbol": symbol }, None)
            .await?;
            
        match position_doc {
            Some(doc) => {
                let position = bson::from_document::<Position>(doc)
                    .map_err(|e| AppError::InternalError(format!("Failed to deserialize position: {}", e)))?;
                Ok(Some(position))
            }
            None => Ok(None)
        }
    }

    pub async fn get_positions_by_user_id(&self, user_id: &str) -> Result<Vec<Position>, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;
        
        let positions_collection = self.db.collection("paper_trading_positions");
        
        let cursor = positions_collection
            .find(doc! { "user_id": user_id_obj }, None)
            .await?;
        
        // Use try_collect from futures::stream::TryStreamExt
        let positions: Vec<Document> = cursor.try_collect().await?;
        
        // Convert documents to Position objects
        let positions = positions
            .into_iter()
            .map(|doc| {
                bson::from_document::<Position>(doc)
                    .map_err(|e| AppError::InternalError(format!("Failed to deserialize position: {}", e)))
            })
            .collect::<Result<Vec<Position>, AppError>>()?;
        
        Ok(positions)
    }
}