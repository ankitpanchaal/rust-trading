use std::env;
use teloxide::{prelude::*, RequestError};
use crate::{
    error::AppError,
    auth::service::AuthService,
    telegram::{
        model::TelegramUpdate,
        repository::TelegramRepository,
    },
};
use uuid::Uuid;

#[derive(Clone)]
pub struct TelegramService {
    bot: Bot,
    repository: TelegramRepository,
    auth_service: Option<AuthService>,
}

impl TelegramService {
    pub fn new(repository: TelegramRepository, auth_service: Option<AuthService>) -> Result<Self, AppError> {
        let token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| AppError::ConfigError("TELEGRAM_BOT_TOKEN is not set".into()))?;
        
        Ok(Self {
            bot: Bot::new(token),
            repository,
            auth_service,
        })
    }
    
    pub async fn send_message(&self, chat_id: &str, message: &str) -> Result<(), AppError> {
        let chat_id = ChatId(chat_id.parse::<i64>()
            .map_err(|_| AppError::ValidationError("Invalid chat ID format".into()))?);
            
        self.bot.send_message(chat_id, message)
            .await
            .map_err(|e: RequestError| AppError::InternalError(format!("Failed to send Telegram message: {}", e)))?;
            
        Ok(())
    }
    
    // New method to send message by user ID
    pub async fn send_message_to_user(&self, user_id: &str, message: &str) -> Result<(), AppError> {
        if let Some(chat_id) = self.repository.get_chat_id_by_user_id(user_id).await? {
            let chat_id = ChatId(chat_id);
            self.bot.send_message(chat_id, message)
                .await
                .map_err(|e: RequestError| AppError::InternalError(format!("Failed to send Telegram message: {}", e)))?;
            Ok(())
        } else {
            Err(AppError::NotFoundError("No Telegram chat ID found for this user".into()))
        }
    }
    
    pub async fn process_update(&self, update: TelegramUpdate) -> Result<(), AppError> {
        if let Some(message) = update.message {
            if let Some(text) = message.text {
                // Handle /start command with deep link
                if text.starts_with("/start ") {
                    let token = text.trim_start_matches("/start ").to_string();
                    self.handle_deep_link(token, message.chat.id, message.from.and_then(|u| u.username)).await?;
                } else if text.starts_with("/start") {
                    // Regular /start command without token
                    self.bot.send_message(
                        ChatId(message.chat.id), 
                        "Welcome! Please use the deep link from the website to connect your account."
                    )
                    .await
                    .map_err(|e| AppError::InternalError(format!("Failed to send message: {}", e)))?;
                }
                // Add other command handlers as needed
            }
        }
        
        Ok(())
    }
    
    async fn handle_deep_link(&self, token: String, chat_id: i64, username: Option<String>) -> Result<(), AppError> {
        // Validate the token and get associated user
        match self.auth_service.as_ref() {
            Some(auth_service) => {
                match auth_service.validate_telegram_token(&token).await {
                    Ok(user_id) => {
                        // Associate the chat_id with the user_id
                        self.repository.associate_user(&user_id, chat_id, username).await?;
                        
                        // Confirm successful connection
                        self.bot.send_message(
                            ChatId(chat_id), 
                            "Your Telegram account is now connected to your trading account! You will receive notifications here."
                        )
                        .await
                        .map_err(|e| AppError::InternalError(format!("Failed to send confirmation: {}", e)))?;
                        
                        Ok(())
                    },
                    Err(_) => {
                        // Invalid or expired token
                        self.bot.send_message(
                            ChatId(chat_id), 
                            "Invalid or expired connection token. Please generate a new link from the website."
                        )
                        .await
                        .map_err(|e| AppError::InternalError(format!("Failed to send error message: {}", e)))?;
                        
                        Err(AppError::AuthError("Invalid Telegram connection token".into()))
                    }
                }
            },
            None => {
                // Auth service not available
                self.bot.send_message(
                    ChatId(chat_id), 
                    "Account linking is currently unavailable. Please try again later."
                )
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send error message: {}", e)))?;
                
                Err(AppError::ConfigError("Auth service not configured for Telegram integration".into()))
            }
        }
    }
    
    // Generate a new connection token for a user
    pub async fn generate_connection_token(&self, user_id: &str) -> Result<String, AppError> {
        // Check if auth service is available
        let auth_service = self.auth_service.as_ref()
            .ok_or_else(|| AppError::ConfigError("Auth service not configured for Telegram integration".into()))?;
        
        // Generate a unique token
        let token = Uuid::new_v4().to_string();
        
        // Store token with user ID (delegated to auth service)
        auth_service.store_telegram_token(user_id, &token).await?;
        
        Ok(token)
    }
}