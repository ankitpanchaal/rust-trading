use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod auth;
mod market;
mod config;
mod db;
mod error;
mod middleware;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");
    
    // Load configuration
    let config = config::Config::from_env()?;
    
    // Connect to MongoDB
    let db = db::mongodb::connect(&config.mongodb_uri, &config.mongodb_name).await?;
    info!("Connected to MongoDB");
    
    // Build our application with routes
    let app = api::router::create_router(db).await?;
    
    // Run our application
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Server listening on {}", addr);
    
    // Create a TCP listener and use axum::serve instead of Server::bind
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}