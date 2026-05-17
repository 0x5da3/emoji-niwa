//! Interface adapters: HTTP/WebSocket handlers, CORS, routing.

pub mod auth;
pub mod rooms;

use actix_cors::Cors;
use actix_web::{web, HttpRequest, HttpResponse};

use crate::domain::origin_ok;
use crate::state::AppState;

/// Extract a `Bearer <token>` value from the Authorization header.
pub fn bearer(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// Authenticated member uid for the request, or None.
pub fn auth_uid(state: &AppState, req: &HttpRequest) -> Option<String> {
    state.session_uid(&bearer(req)?)
}

pub async fn healthz() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

/// CORS policy: only the app origin (prod) / localhost (dev).
pub fn cors(app_origin: String, dev: bool) -> Cors {
    Cors::default()
        .allowed_origin_fn(move |origin, _req| {
            origin
                .to_str()
                .map(|o| origin_ok(o, &app_origin, dev))
                .unwrap_or(false)
        })
        .allow_any_method()
        .allow_any_header()
        .max_age(3600)
}

pub fn routes(c: &mut web::ServiceConfig) {
    c.route("/healthz", web::get().to(healthz))
        .route("/auth/login", web::get().to(auth::login))
        .route("/auth/callback", web::get().to(auth::callback))
        .route("/auth/me", web::get().to(auth::me))
        .route("/auth/logout", web::post().to(auth::logout))
        .route("/room/new", web::post().to(rooms::new))
        .route("/room/{id}/ttl", web::post().to(rooms::set_ttl))
        .route("/room/{id}", web::get().to(rooms::ws));
}
