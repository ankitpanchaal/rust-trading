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
    auth_service: AuthService,
}

impl TelegramService {
    pub fn new(repository: TelegramRepository, auth_service: AuthService) -> Result<Self, AppError> {
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
                } else if text.starts_with("/enable") {
                    // Enable paper trading for the user
                    self.handle_enable_command(message.chat.id).await?;
                } else if text.starts_with("/disable") {
                    // Disable paper trading for the user
                    self.handle_disable_command(message.chat.id).await?;
                } else if text.starts_with("/positions") || text.starts_with("/stats") {
                    // Get user's positions and stats
                    self.handle_positions_command(message.chat.id).await?;
                }
                // Add other command handlers as needed
            }
        }
        
        Ok(())
    }

    async fn handle_positions_command(&self, chat_id: i64) -> Result<(), AppError> {
        // Get user_id associated with this chat_id
        match self.repository.get_user_id_by_chat_id(chat_id).await {
            Ok(Some(user_id)) => {
                // Get user's paper trading positions
                let positions = self.repository.get_user_positions(&user_id).await?;
            
            // Get user's balance
            let balance = self.repository.get_user_balance(&user_id).await?;
            
            // Format the message with positions and stats
            let mut message = format!("📊 *Account Summary*\n\nBalance: ${:.2}", balance);
            
            if positions.is_empty() {
                message.push_str("\n\nYou don't have any open positions.");
            } else {
                message.push_str("\n\n*Open Positions:*\n");
                
                let mut total_value = 0.0;
                
                for position in &positions {
                    let position_value = position.quantity * position.current_price;
                    total_value += position_value;
                    
                    let pnl = position_value - (position.quantity * position.entry_price);
                    let pnl_percentage = (pnl / (position.quantity * position.entry_price)) * 100.0;
                    
                    message.push_str(&format!(
                        "\n🔹 *{}*: {:.4} @ ${:.2}\n   Value: ${:.2} | P&L: ${:.2} ({:.2}%)",
                        escape_markdown(&position.symbol),
                        position.quantity,
                        position.current_price,
                        position_value,
                        pnl,
                        pnl_percentage
                    ));
                }
                
                message.push_str(&format!("\n\n*Total Portfolio Value:* ${:.2}", balance + total_value));
            }
            
            // Escape the entire message for MarkdownV2
            let escaped_message = escape_markdown(&message);
            
            // Send the message
            self.bot.send_message(ChatId(chat_id), escaped_message)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send positions data: {}", e)))?;
            
            Ok(())
            },
            Ok(None) => {
                // No user associated with this chat ID
                self.bot.send_message(
                    ChatId(chat_id),
                    "Your Telegram account is not connected to any user. Please connect your account first."
                )
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send message: {}", e)))?;
                
                Ok(())
            },
            Err(e) => Err(e)
        }
    }
    
    async fn handle_deep_link(&self, token: String, chat_id: i64, username: Option<String>) -> Result<(), AppError> {
        // Validate the token and get associated user
        match self.auth_service.validate_telegram_token(&token).await {
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
    }

    async fn handle_enable_command(&self, chat_id: i64) -> Result<(), AppError> {
        // Get user_id associated with this chat_id
        match self.repository.get_user_id_by_chat_id(chat_id).await {
            Ok(Some(user_id)) => {
                // Update user's paper trading status in the database
                self.repository.update_paper_trading_status(&user_id, true).await?;
                
                // Send confirmation message
                self.bot.send_message(
                    ChatId(chat_id),
                    "Paper trading has been enabled for your account."
                )
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send confirmation: {}", e)))?;
                
                Ok(())
            },
            Ok(None) => {
                // No user associated with this chat ID
                self.bot.send_message(
                    ChatId(chat_id),
                    "Your Telegram account is not connected to any user. Please connect your account first."
                )
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send message: {}", e)))?;
                
                Ok(())
            },
            Err(e) => Err(e)
        }
    }
    
    async fn handle_disable_command(&self, chat_id: i64) -> Result<(), AppError> {
        // Get user_id associated with this chat_id
        match self.repository.get_user_id_by_chat_id(chat_id).await {
            Ok(Some(user_id)) => {
                // Update user's paper trading status in the database
                self.repository.update_paper_trading_status(&user_id, false).await?;
                
                // Send confirmation message
                self.bot.send_message(
                    ChatId(chat_id),
                    "Paper trading has been disabled for your account."
                )
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send confirmation: {}", e)))?;
                
                Ok(())
            },
            Ok(None) => {
                // No user associated with this chat ID
                self.bot.send_message(
                    ChatId(chat_id),
                    "Your Telegram account is not connected to any user. Please connect your account first."
                )
                .await
                .map_err(|e| AppError::InternalError(format!("Failed to send message: {}", e)))?;
                
                Ok(())
            },
            Err(e) => Err(e)
        }
    }
    
    // Generate a new connection token for a user
    pub async fn generate_connection_token(&self, user_id: &str) -> Result<String, AppError> {
        // Generate a unique token
        let token = Uuid::new_v4().to_string();
        
        // Store token with user ID (delegated to auth service)
        self.auth_service.store_telegram_token(user_id, &token).await?;
        
        Ok(token)
    }
}

fn escape_markdown(text: &str) -> String {
    let special_chars = &[
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', 
        '-', '=', '|', '{', '}', '.', '!'
    ];
    
    let mut result = String::with_capacity(text.len() * 2);
    
    for c in text.chars() {
        if special_chars.contains(&c) || c == '\\' {
            result.push('\\');
        }
        result.push(c);
    }
    
    // Additionally handle '$' separately since it's used in currency formatting
    result = result.replace("$", "\\$");
    
    result
}
