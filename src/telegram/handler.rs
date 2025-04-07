use axum::{extract::State, http::StatusCode, Json};
use validator::Validate;

use crate::{
  error::AppError,
  telegram::{
      model::{MessageResponse, SendMessageRequest, TelegramUpdate},
      service::TelegramService,
  },
};

pub async fn send_message(
    State(service): State<TelegramService>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Validate input
    if let Err(e) = req.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        ));
    }

  // Send message
  match service.send_message(&req.chat_id, &req.message).await {
      Ok(_) => Ok(Json(MessageResponse {
          success: true,
          message: "Message sent successfully".into(),
      })),
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}

// New webhook handler
pub async fn webhook(
  State(service): State<TelegramService>,
  Json(update): Json<TelegramUpdate>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
  match service.process_update(update).await {
      Ok(_) => Ok(StatusCode::OK),
      Err(e) => {
          // Log the error but return OK to Telegram
          // We don't want Telegram to retry failed requests
          tracing::error!("Error processing Telegram update: {}", e);
          Ok(StatusCode::OK)
      }
  }
}

// Generate a connection token for a user
pub async fn generate_token(
  State(service): State<TelegramService>,
  user_id: String, // This would typically come from an authenticated session
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
  match service.generate_connection_token(&user_id).await {
      Ok(token) => {
          let bot_username = std::env::var("TELEGRAM_BOT_USERNAME").unwrap_or_else(|_| "your_bot".to_string());
          let deep_link = format!("https://t.me/{}?start={}", bot_username, token);
          
          Ok(Json(serde_json::json!({
              "success": true,
              "token": token,
              "deep_link": deep_link
          })))
      },
      Err(e) => Err((
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(serde_json::json!({ "error": format!("{}", e) })),
      )),
  }
}
