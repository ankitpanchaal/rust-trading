use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use std::env;
use sqlx::postgres::PgPoolOptions;
use log::info;

mod config;
mod db;
mod models;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    env_logger::init();

    // Read host and port from env
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("{}:{}", host, port);

    // Get the database URL from env
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create PostgreSQL connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    info!("Starting server at http://{}", bind_addr);

    // Start HTTP server with the connection pool and routes
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(routes::init_routes)
    })
    .bind(bind_addr)?
    .run()
    .await
}
