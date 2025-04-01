use mongodb::{
  bson::{doc, Document},
  Collection,
};
use crate::{
  error::AppError,
  telegram::model::TelegramRegistration,
  db::MongoDb,
};

#[derive(Clone)]
pub struct TelegramRepository {
  collection: Collection<Document>,
}

impl TelegramRepository {
  pub fn new(db: MongoDb) -> Self {
      Self {
          collection: db.collection("telegram_users"),
      }
  }

  pub async fn associate_user(&self, user_id: &str, chat_id: i64, username: Option<String>) -> Result<(), AppError> {
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
              mongodb::options::UpdateOptions::builder().upsert(true).build(),
          )
          .await
          .map_err(|e| AppError::DatabaseError(format!("Failed to save Telegram association: {}", e)))?;
          
      Ok(())
  }
  
  pub async fn get_chat_id_by_user_id(&self, user_id: &str) -> Result<Option<i64>, AppError> {
      let result = self.collection
          .find_one(doc! { "user_id": user_id }, None)
          .await
          .map_err(|e| AppError::DatabaseError(format!("Failed to query Telegram user: {}", e)))?;
          
      match result {
          Some(doc) => {
              let chat_id = doc.get_i64("chat_id")
                  .map_err(|e| AppError::InternalError(format!("Failed to parse chat_id: {}", e)))?;
              Ok(Some(chat_id))
          },
          None => Ok(None),
      }
  }
  
  pub async fn get_user_id_by_chat_id(&self, chat_id: i64) -> Result<Option<String>, AppError> {
      let result = self.collection
          .find_one(doc! { "chat_id": chat_id }, None)
          .await
          .map_err(|e| AppError::DatabaseError(format!("Failed to query Telegram user: {}", e)))?;
          
      match result {
          Some(doc) => {
              let user_id = doc.get_str("user_id")
                  .map_err(|e| AppError::InternalError(format!("Failed to parse user_id: {}", e)))?;
              Ok(Some(user_id.to_string()))
          },
          None => Ok(None),
      }
  }
}