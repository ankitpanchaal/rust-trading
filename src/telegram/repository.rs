use crate::{
    db::MongoDb,
    error::AppError,
    paper_trading::{model::Position, repository::PaperTradingRepository},
    telegram::model::TelegramRegistration,
};
use mongodb::{
    bson::{doc, Document},
    Collection,
};

#[derive(Clone)]
pub struct TelegramRepository {
    collection: Collection<Document>,
    db: MongoDb,
}

impl TelegramRepository {
    pub fn new(db: MongoDb) -> Self {
        Self {
            collection: db.collection("telegram_users"),
            db,
        }
    }

    pub async fn associate_user(
        &self,
        user_id: &str,
        chat_id: i64,
        username: Option<String>,
    ) -> Result<(), AppError> {
        let registration = TelegramRegistration {
            user_id: user_id.to_string(),
            chat_id,
            username,
        };

        let doc = mongodb::bson::to_document(&registration)
            .map_err(|e| AppError::InternalError(format!("Failed to convert to BSON: {}", e)))?;

        // Upsert the document based on user_id
        self.collection
            .update_one(
                doc! { "user_id": user_id },
                doc! { "$set": doc },
                mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            )
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to save Telegram association: {}", e))
            })?;

        Ok(())
    }

    pub async fn get_chat_id_by_user_id(&self, user_id: &str) -> Result<Option<i64>, AppError> {
        let result = self
            .collection
            .find_one(doc! { "user_id": user_id }, None)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to query Telegram user: {}", e))
            })?;

        match result {
            Some(doc) => {
                let chat_id = doc.get_i64("chat_id").map_err(|e| {
                    AppError::InternalError(format!("Failed to parse chat_id: {}", e))
                })?;
                Ok(Some(chat_id))
            }
            None => Ok(None),
        }
    }

    pub async fn get_user_id_by_chat_id(&self, chat_id: i64) -> Result<Option<String>, AppError> {
        let result = self
            .collection
            .find_one(doc! { "chat_id": chat_id }, None)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to query Telegram user: {}", e))
            })?;

        match result {
            Some(doc) => {
                let user_id = doc.get_str("user_id").map_err(|e| {
                    AppError::InternalError(format!("Failed to parse user_id: {}", e))
                })?;
                Ok(Some(user_id.to_string()))
            }
            None => Ok(None),
        }
    }

    pub async fn update_paper_trading_status(
        &self,
        user_id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        use mongodb::bson::oid::ObjectId;
        use std::str::FromStr;

        // Convert string user_id to ObjectId for MongoDB
        let user_id_obj = ObjectId::from_str(user_id)
            .map_err(|_| AppError::ValidationError("Invalid user ID".to_string()))?;

        // Get reference to users collection
        let users_collection = self.db.collection("users");

        // Update the user document
        let filter = doc! { "_id": user_id_obj };
        let update = doc! { "$set": { "paper_trading_enabled": enabled } };

        users_collection
            .update_one(filter, update, None)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to update paper trading status: {}", e))
            })?;

        Ok(())
    }

    pub async fn get_user_positions(&self, user_id: &str) -> Result<Vec<Position>, AppError> {
        // We'll create a PaperTradingRepository to access the positions
        let paper_trading_repo = PaperTradingRepository::new(
            self.db.clone(),
            crate::binance::market_service::BinanceMarketService::new(),
        );

        paper_trading_repo.get_positions_by_user_id(user_id).await
    }

    pub async fn get_user_balance(&self, user_id: &str) -> Result<f64, AppError> {
        // We'll create a PaperTradingRepository to access the balance
        let paper_trading_repo = PaperTradingRepository::new(
            self.db.clone(),
            crate::binance::market_service::BinanceMarketService::new()
        );

        paper_trading_repo.get_user_balance(user_id).await
    }
}
