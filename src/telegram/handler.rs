use axum::{
  extract::State,
  http::StatusCode,
  Json,
};
use validator::Validate;

use crate::{
  error::AppError,
  telegram::{
      model::{MessageResponse, SendMessageRequest},
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