use futures::stream::TryStreamExt;
use mongodb::bson::{self, doc, oid::ObjectId, Document};
use std::str::FromStr;

use crate::{
    db::MongoDb,
    error::AppError,
    strategies::model::Strategy,
};

#[derive(Clone)]
pub struct StrategyRepository {
    pub db: MongoDb,
}

impl StrategyRepository {
    pub fn new(db: MongoDb) -> Self {
        Self { db }
    }

    pub async fn create_strategy(&self, strategy: Strategy) -> Result<Strategy, AppError> {
        let strategies_collection = self.db.collection("strategies");
        
        let strategy_doc = bson::to_document(&strategy)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize strategy: {}", e)))?;
        
        let insert_result = strategies_collection.insert_one(strategy_doc, None).await?;
        
        let id = insert_result
            .inserted_id
            .as_object_id()
            .ok_or_else(|| AppError::InternalError("Failed to get inserted strategy ID".to_string()))?;
        
        let mut strategy_with_id = strategy;
        strategy_with_id.id = Some(id);
        
        Ok(strategy_with_id)
    }

    pub async fn update_strategy(&self, strategy: &Strategy) -> Result<(), AppError> {
        if strategy.id.is_none() {
            return Err(AppError::ValidationError("Strategy ID is required".to_string()));
        }

        let strategies_collection = self.db.collection("strategies");
        
        let strategy_doc = bson::to_document(&strategy)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize strategy: {}", e)))?;
        
        let filter = doc! { "_id": strategy.id };
        let update = doc! { "$set": strategy_doc };
        
        strategies_collection.update_one(filter, update, None).await?;
        
        Ok(())
    }

    pub async fn get_strategy_by_id(&self, strategy_id: &str) -> Result<Option<Strategy>, AppError> {
        let strategy_id_obj = ObjectId::from_str(strategy_id)
            .map_err(|_| AppError::ValidationError("Invalid strategy ID".to_string()))?;
        
        let strategies_collection = self.db.collection("strategies");
        
        let strategy_doc = strategies_collection
            .find_one(doc! { "_id": strategy_id_obj }, None)
            .await?;
        
        match strategy_doc {
            Some(doc) => {
                let strategy = bson::from_document::<Strategy>(doc)
                    .map_err(|e| AppError::InternalError(format!("Failed to deserialize strategy: {}", e)))?;
                Ok(Some(strategy))
            }
            None => Ok(None),
        }
    }

    pub async fn get_strategies_by_user_id(&self, user_id: &str) -> Result<Vec<Strategy>, AppError> {
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;
        
        let strategies_collection = self.db.collection("strategies");
        
        let cursor = strategies_collection
            .find(doc! { "user_id": user_id_obj }, None)
            .await?;
        
        let strategies: Vec<Document> = cursor.try_collect().await?;
        
        let strategies = strategies
            .into_iter()
            .map(|doc| {
                bson::from_document::<Strategy>(doc)
                    .map_err(|e| AppError::InternalError(format!("Failed to deserialize strategy: {}", e)))
            })
            .collect::<Result<Vec<Strategy>, AppError>>()?;
        
        Ok(strategies)
    }

    pub async fn get_active_strategies(&self) -> Result<Vec<Strategy>, AppError> {
        let strategies_collection = self.db.collection("strategies");
        
        let cursor = strategies_collection
            .find(doc! { "status": "Active" }, None)
            .await?;
        
        let strategies: Vec<Document> = cursor.try_collect().await?;
        
        let strategies = strategies
            .into_iter()
            .map(|doc| {
                bson::from_document::<Strategy>(doc)
                    .map_err(|e| AppError::InternalError(format!("Failed to deserialize strategy: {}", e)))
            })
            .collect::<Result<Vec<Strategy>, AppError>>()?;
        
        Ok(strategies)
    }

    pub async fn delete_strategy(&self, strategy_id: &str) -> Result<bool, AppError> {
        let strategy_id_obj = ObjectId::from_str(strategy_id)
            .map_err(|_| AppError::ValidationError("Invalid strategy ID".to_string()))?;
        
        let strategies_collection = self.db.collection("strategies");
        
        let delete_result = strategies_collection
            .delete_one(doc! { "_id": strategy_id_obj }, None)
            .await?;
        
        Ok(delete_result.deleted_count > 0)
    }
}