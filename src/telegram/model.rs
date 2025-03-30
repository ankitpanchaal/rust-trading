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