use std::env;
use chrono::Duration;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub app_env: AppEnv,
    pub mongodb_uri: String,
    pub mongodb_name: String,
    pub jwt_secret: String,
    pub jwt_expires_in: Duration,
    pub jwt_refresh_expires_in: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppEnv {
    Development,
    Production,
    Test,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        // Server config
        let port = env::var("PORT")
            .unwrap_or_else(|_| "5000".into())
            .parse::<u16>()
            .map_err(|_| AppError::ConfigError("Invalid PORT".into()))?;
            
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
        
        let app_env_str = env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        let app_env = match app_env_str.to_lowercase().as_str() {
            "production" => AppEnv::Production,
            "test" => AppEnv::Test,
            _ => AppEnv::Development,
        };
        
        // MongoDB config
        let mongodb_uri = env::var("MONGODB_URI")
            .map_err(|_| AppError::ConfigError("MONGODB_URI must be set".into()))?;
            
        let mongodb_name = env::var("MONGODB_NAME")
            .map_err(|_| AppError::ConfigError("MONGODB_NAME must be set".into()))?;
        
        // JWT config
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| AppError::ConfigError("JWT_SECRET must be set".into()))?;
            
        let jwt_expires_in_str = env::var("JWT_EXPIRES_IN").unwrap_or_else(|_| "60m".into());
        let jwt_expires_in = parse_duration(&jwt_expires_in_str)
            .map_err(|_| AppError::ConfigError("Invalid JWT_EXPIRES_IN format".into()))?;
            
        let jwt_refresh_expires_in_str = env::var("JWT_REFRESH_EXPIRES_IN").unwrap_or_else(|_| "7d".into());
        let jwt_refresh_expires_in = parse_duration(&jwt_refresh_expires_in_str)
            .map_err(|_| AppError::ConfigError("Invalid JWT_REFRESH_EXPIRES_IN format".into()))?;
        
        Ok(Self {
            port,
            host,
            app_env,
            mongodb_uri,
            mongodb_name,
            jwt_secret,
            jwt_expires_in,
            jwt_refresh_expires_in,
        })
    }
}

fn parse_duration(duration_str: &str) -> Result<Duration, &'static str> {
    let duration_str = duration_str.trim();
    
    if duration_str.is_empty() {
        return Err("Duration string is empty");
    }
    
    // Extract the number and unit parts
    let len = duration_str.len();
    let (num_part, unit_part) = duration_str.split_at(
        duration_str
            .chars()
            .position(|c| !c.is_ascii_digit())
            .unwrap_or(len)
    );
    
    let num = num_part.parse::<i64>().map_err(|_| "Invalid number")?;
    
    match unit_part {
        "s" => Ok(Duration::seconds(num)),
        "m" => Ok(Duration::minutes(num)),
        "h" => Ok(Duration::hours(num)),
        "d" => Ok(Duration::days(num)),
        _ => Err("Unknown time unit, use s, m, h, or d"),
    }
}