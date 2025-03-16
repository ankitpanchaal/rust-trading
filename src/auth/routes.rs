use axum::{
  middleware,
  routing::{get, post},
  Router,
};

use crate::{auth::handler, auth::service::AuthService, middleware::auth::auth_middleware};

pub fn auth_routes(service: AuthService) -> Router {
  // Create a copy of config outside the closure for auth_middleware
  let auth_config = service.get_config().clone(); // Assuming a getter method exists
  
  Router::new()
      .route("/register", post(handler::register))
      .route("/login", post(handler::login))
      .route("/refresh", post(handler::refresh_token))
      .route(
          "/me",
          get(handler::me).route_layer(middleware::from_fn_with_state(
              auth_config,
              auth_middleware,
          )),
      )
      .with_state(service)
}