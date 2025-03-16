use mongodb::bson::DateTime as BsonDateTime;
use mongodb::bson::doc;
use chrono::{DateTime, Utc};
use mongodb::bson::{self, oid::ObjectId, Document};
use serde::{Deserialize, Serialize};
use validator::Validate;

// User document stored in MongoDB
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
    // Paper trading properties
    #[serde(default = "default_paper_trading_enabled")]
    pub paper_trading_enabled: bool,
    #[serde(default = "default_paper_balance")]
    pub paper_balance_usd: f64,
    // We'll keep track of initial paper balance for performance tracking
    #[serde(default = "default_paper_balance")]
    pub initial_paper_balance_usd: f64,
}

fn default_paper_trading_enabled() -> bool {
    false
}

fn default_paper_balance() -> f64 {
    10000.0  // Default $10,000 USD for paper trading
}

// User role enum
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum UserRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "admin")]
    Admin,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

// User registration request
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, message = "First name is required"))]
    pub first_name: String,
    #[validate(length(min = 1, message = "Last name is required"))]
    pub last_name: String,
}

// User login request
#[derive(Debug, Deserialize, Validate)]
pub struct LoginUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

// User information sent to client
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub paper_trading_enabled: bool,
    pub paper_balance_usd: f64,
    pub initial_paper_balance_usd: f64,
}

// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String, // User ID
    pub email: String,
    pub role: String,
    pub exp: usize, // Expiration time
    pub iat: usize, // Issued at
}

// Authentication response (tokens)
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

// Refresh token request
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.unwrap_or_default().to_hex(),
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            role: user.role,
            created_at: user.created_at,
            paper_trading_enabled: user.paper_trading_enabled,
            paper_balance_usd: user.paper_balance_usd,
            initial_paper_balance_usd: user.initial_paper_balance_usd,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct EnablePaperTradingRequest {
    #[validate(range(min = 100.0, max = 1000000.0, message = "Balance must be between $100 and $1,000,000"))]
    pub initial_balance_usd: f64,
}

impl User {
    pub fn new(email: String, hashed_password: String, first_name: String, last_name: String) -> Self {
        let now = Utc::now();
        
        Self {
            id: None,
            email,
            password: hashed_password,
            first_name: Some(first_name),
            last_name: Some(last_name),
            role: UserRole::default(),
            created_at: now,
            updated_at: now,
            initial_paper_balance_usd: default_paper_balance(),
            paper_balance_usd: default_paper_balance(),
            paper_trading_enabled: default_paper_trading_enabled(),
        }
    }
    
    pub fn into_document(self) -> Document {
        use mongodb::bson::DateTime as BsonDateTime;
    
        let mut doc = doc! {
            "email": self.email,
            "password": self.password,
            "first_name": self.first_name,
            "last_name": self.last_name,
            "role": bson::to_bson(&self.role).unwrap(),
            "created_at": BsonDateTime::from_chrono(self.created_at),
            "updated_at": BsonDateTime::from_chrono(self.updated_at),
            "paper_trading_enabled": self.paper_trading_enabled,
            "paper_balance_usd": self.paper_balance_usd,
            "initial_paper_balance_usd": self.initial_paper_balance_usd
        };
        
        if let Some(id) = self.id {
            doc.insert("_id", id);
        }
        
        doc
    }
}