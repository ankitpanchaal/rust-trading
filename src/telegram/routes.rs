use axum::{
  routing::post,
  Router,
};
use crate::{
  error::AppError,
  telegram::{handler, service::TelegramService}
};

pub fn telegram_routes() -> Result<Router, AppError> {
  let service = TelegramService::new()?;
  
  Ok(Router::new()
    .route("/send", post(handler::send_message))
    .with_state(service))
}