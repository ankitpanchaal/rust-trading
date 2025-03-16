use mongodb::{
  bson::{self, doc, oid::ObjectId, Document},
  options::FindOneOptions,
};

use crate::{db::MongoDb, error::AppError};

use super::model::User;

#[derive(Clone)]
pub struct AuthRepository {
  db: MongoDb,
}

impl AuthRepository {
  pub fn new(db: MongoDb) -> Self {
      Self { db }
  }

  pub async fn create_user(&self, user: User) -> Result<User, AppError> {
      let collection = self.db.collection("users");
      
      // Check if user with same email already exists
      let existing_user = collection
          .find_one(doc! { "email": &user.email }, None)
          .await?;
          
      if existing_user.is_some() {
          return Err(AppError::ValidationError("Email already in use".into()));
      }
      
      // Insert new user
      let result = collection.insert_one(user.into_document(), None).await?;
      
      // Get inserted user ID
      let id = result
          .inserted_id
          .as_object_id()
          .ok_or_else(|| AppError::DatabaseError("Failed to get inserted ID".into()))?;
          
      // Fetch and return the created user
      self.find_user_by_id(&id).await
  }

  pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
      let collection = self.db.collection("users");
      
      let user_doc = collection
          .find_one(doc! { "email": email }, None)
          .await?;
          
      match user_doc {
          Some(doc) => {
              let user: User = bson::from_document(doc)
                  .map_err(|e| AppError::DatabaseError(format!("Failed to deserialize user: {}", e)))?;
              Ok(Some(user))
          }
          None => Ok(None),
      }
  }

  pub async fn find_user_by_id(&self, id: &ObjectId) -> Result<User, AppError> {
      let collection = self.db.collection("users");
      
      let user_doc = collection
          .find_one(doc! { "_id": id }, None)
          .await?
          .ok_or_else(|| AppError::NotFoundError(format!("User with ID {} not found", id)))?;
          
      let user: User = bson::from_document(user_doc)
          .map_err(|e| AppError::DatabaseError(format!("Failed to deserialize user: {}", e)))?;
          
      Ok(user)
  }
}