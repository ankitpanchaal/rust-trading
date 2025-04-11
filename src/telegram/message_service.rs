use crate::error::AppError;
use crate::telegram::repository::TelegramRepository;
use std::env;
use teloxide::{prelude::*, RequestError};

#[derive(Clone)]
pub struct TelegramMessageService {
    bot: Bot,
    repository: TelegramRepository,
}

impl TelegramMessageService {
    pub fn new(repository: TelegramRepository) -> Result<Self, AppError> {
        let token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| AppError::ConfigError("TELEGRAM_BOT_TOKEN is not set".into()))?;

        Ok(Self {
            bot: Bot::new(token),
            repository,
        })
    }

    pub async fn send_notification(&self, user_id: &str, message: &str) -> Result<(), AppError> {
        if let Some(chat_id) = self.repository.get_chat_id_by_user_id(user_id).await? {
            let chat_id = ChatId(chat_id);
            self.bot
                .send_message(chat_id, message)
                .await
                .map_err(|e: RequestError| {
                    AppError::InternalError(format!("Failed to send Telegram message: {}", e))
                })?;
            Ok(())
        } else {
            Err(AppError::NotFoundError(
                "No Telegram chat ID found for this user".into(),
            ))
        }
    }
}