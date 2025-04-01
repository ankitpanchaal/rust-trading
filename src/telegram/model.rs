use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageRequest {
    #[validate(length(min = 1, message = "Message cannot be empty"))]
    pub message: String,
    
    #[validate(length(min = 1, message = "Chat ID cannot be empty"))]
    pub chat_id: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

// Telegram Update models
#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>, // Optional user field
    pub chat: Chat,
    pub text: Option<String>,
    pub date: i64,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    #[serde(rename = "type")]
    pub chat_type: String,
}

// Deep link registration model
#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramRegistration {
    pub user_id: String,
    pub chat_id: i64,
    pub username: Option<String>,
}