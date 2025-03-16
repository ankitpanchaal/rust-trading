use mongodb::{
  bson::{self, Document},
  options::{ClientOptions, ServerApi, ServerApiVersion},
  Client, Database,
};

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct MongoDb {
  pub client: Client,
  pub db: Database,
}

pub async fn connect(uri: &str, db_name: &str) -> Result<MongoDb, AppError> {
  // Parse connection string
  let mut client_options = ClientOptions::parse(uri)
      .await
      .map_err(|e| AppError::DatabaseError(format!("Failed to parse MongoDB connection string: {}", e)))?;

  // Set server API if using MongoDB Atlas (5.0 or later)
  let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
  client_options.server_api = Some(server_api);

  // Create and return database client and connection
  let client = Client::with_options(client_options)
      .map_err(|e| AppError::DatabaseError(format!("Failed to create MongoDB client: {}", e)))?;

  // Ping the database to confirm connection - FIX: Use a valid ping command document
  client
      .database("admin")
      .run_command(bson::doc! { "ping": 1 }, None)
      .await
      .map_err(|e| {
          AppError::DatabaseError(format!("Failed to connect to MongoDB: {}", e))
      })?;

  // Get database
  let db = client.database(db_name);

  Ok(MongoDb { client, db })
}

impl MongoDb {
  pub fn collection(&self, name: &str) -> mongodb::Collection<Document> {
      self.db.collection(name)
  }
}