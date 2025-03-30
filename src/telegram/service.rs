use std::env;
use teloxide::{prelude::*, RequestError};
use crate::error::AppError;

#[derive(Clone)]
pub struct TelegramService {
    bot: Bot,
}

impl TelegramService {
    pub fn new() -> Result<Self, AppError> {
        let token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| AppError::ConfigError("TELEGRAM_BOT_TOKEN is not set".into()))?;
        
        Ok(Self {
            bot: Bot::new(token),
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
}