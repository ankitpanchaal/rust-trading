use std::net::SocketAddr;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod auth;
mod market;
mod paper_trading;
mod strategies;
mod config;
mod db;
mod error;
mod middleware;
mod utils;

use crate::strategies::repository::StrategyRepository;
use crate::strategies::service::StrategyService;

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
    
    // Create services
    let market_service = market::service::MarketService::new(); // Remove the parameter
    let paper_trading_repository = paper_trading::repository::PaperTradingRepository::new(
        db.clone(), 
        market_service.clone()
    );
    let paper_trading_service = paper_trading::service::PaperTradingService::new(
        paper_trading_repository, 
        market_service.clone()
    );
    
    // Create strategy service for the background task
    let strategy_repository = StrategyRepository::new(db.clone());
    let strategy_service = StrategyService::new(
        strategy_repository,
        paper_trading_service.clone(),
        market_service.clone(),
    );

    // Strategy execution background task
    let strategy_service_clone = strategy_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Run every minute
        
        loop {
            interval.tick().await;
            info!("Running scheduled strategy execution");
            match strategy_service_clone.execute_strategies().await {
                Ok(_) => {
                    info!("Strategy execution completed successfully");
                }
                Err(e) => {
                    eprintln!("Error executing strategies: {}", e);
                }
            }
        }
    });
    
    // Build our application with routes - fix the function call to match its definition
    let app = api::router::create_router(db).await?;
    
    // Run our application
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Server listening on {}", addr);
    
    // Create a TCP listener and use axum::serve instead of Server::bind
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}