pub mod auth;

use actix_web::web;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .service(auth::signup)
            .service(auth::login)
            .service(auth::get_user)
    );
}
