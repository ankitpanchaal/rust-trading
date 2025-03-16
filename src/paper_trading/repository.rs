use futures::stream::TryStreamExt;
use mongodb::bson::{self, doc, oid::ObjectId, Document};
use std::str::FromStr;

use crate::{
    auth::model::User,
    db::MongoDb,
    error::AppError,
    market::service::MarketService,
};

use super::model::{Order, Position};

#[derive(Clone)]
pub struct PaperTradingRepository {
    pub db: MongoDb,
}

impl PaperTradingRepository {
    pub fn new(db: MongoDb, _market_service: MarketService) -> Self {
        Self { db }
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

        Ok(user)
    }

    pub async fn update_user_balance(&self, user_id: ObjectId, new_balance: f64) -> Result<(), AppError> {
        let users_collection = self.db.collection("users");
        let filter = doc! { "_id": user_id };
        let update = doc! { "$set": { "paper_balance_usd": new_balance } };
        
        users_collection.update_one(filter, update, None).await?;
        
        Ok(())
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
        
        Ok(position_with_id)
    }

    pub async fn update_position(&self, position: &Position) -> Result<(), AppError> {
        let position_id = position.id.ok_or_else(|| {
            AppError::ValidationError("Position ID is required for update".to_string())
        })?;
        
        let positions_collection = self.db.collection("paper_trading_positions");
        
        let filter = doc! { "_id": position_id };
        let position_doc = bson::to_document(position)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize position: {}", e)))?;
        
        positions_collection
            .replace_one(filter, position_doc, None)
            .await?;
        
        Ok(())
    }

    pub async fn delete_position(&self, position_id: &ObjectId) -> Result<(), AppError> {
        let positions_collection = self.db.collection("paper_trading_positions");
        
        positions_collection
            .delete_one(doc! { "_id": position_id }, None)
            .await?;
        
        Ok(())
    }

    pub async fn get_position_by_id(&self, position_id: &str) -> Result<Position, AppError> {
        let position_id_obj = ObjectId::from_str(position_id)
            .map_err(|_| AppError::ValidationError("Invalid position ID".to_string()))?;
        
        let positions_collection = self.db.collection("paper_trading_positions");
        
        let position_doc = positions_collection
            .find_one(doc! { "_id": position_id_obj }, None)
            .await?
            .ok_or_else(|| AppError::NotFoundError("Position not found".to_string()))?;
            
        let position = bson::from_document::<Position>(position_doc)
            .map_err(|e| AppError::InternalError(format!("Failed to deserialize position: {}", e)))?;
        
        Ok(position)
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