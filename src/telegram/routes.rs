use crate::{
    auth::service::AuthService,
    config::Config,
    db::MongoDb,
    error::AppError,
    middleware::auth::auth_middleware,
    telegram::{handler, repository::TelegramRepository, service::TelegramService},
};
use axum::{
    middleware,
    routing::{get, post},
    Router,
};

pub fn telegram_routes(
    db: MongoDb,
    auth_service: AuthService,
    config: Config,
) -> Result<Router, AppError> {
    let repository = TelegramRepository::new(db);
    let service = TelegramService::new(repository, auth_service)?;

    Ok(Router::new()
        .route("/send", post(handler::send_message))
        .route("/webhook", post(handler::webhook))
        .route(
            "/token",
            get(handler::generate_token).route_layer(middleware::from_fn_with_state(
                config,
                auth_middleware,
            )),
        )
        .with_state(service))
}
