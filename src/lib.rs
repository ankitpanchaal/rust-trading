pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod utils;

// Re-export common modules
pub use api::router;
pub use config::Config;
pub use db::MongoDb;
pub use error::AppError;
pub mod market;
pub mod paper_trading;