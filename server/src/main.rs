//! emoji-niwa multiplayer backend — composition root.
//!
//! Layering: `domain` (entities/pure logic) ← `config`/`state` ←
//! `persistence` (fs adapter) ← `api` (HTTP/WS adapters) ← `main` (wiring).
//!
//! - Members log in with GitHub OAuth (state CSRF, confidential client).
//! - Only members can ISSUE a room (`POST /room/new`); joining `/room/{id}`
//!   (WebSocket) is open to anyone with the invite link.
//! - The server is a dumb relay: it keeps each room's latest compact world
//!   snapshot (the client's `encodeWorld` string, opaque here) and fans it out.
//! - State is in-process, snapshotted to disk periodically and on shutdown.

mod api;
mod config;
mod domain;
mod persistence;
mod state;

use std::time::Duration;

use actix_web::{web, App, HttpServer};

use crate::config::Config;
use crate::domain::{now_secs, DAY_SECS, SAVE_INTERVAL};
use crate::state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cfg = Config::from_env();
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    if cfg.gh_client_id.is_empty() || cfg.gh_client_secret.is_empty() {
        eprintln!("WARNING: GH_CLIENT_ID / GH_CLIENT_SECRET not set — auth will fail.");
    }

    let state = web::Data::new(AppState::new(cfg));
    persistence::load(&state);

    // periodic persistence + GC of idle empty rooms
    let bg = state.clone();
    actix_web::rt::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(SAVE_INTERVAL)).await;
            let nowt = now_secs();
            bg.oauth_states.retain(|_, exp| *exp >= nowt);
            bg.sessions.retain(|_, s| s.exp >= nowt);
            bg.rooms.retain(|_, r| {
                let empty = r.conns.lock().unwrap().is_empty();
                let ttl = *r.ttl_days.lock().unwrap() * DAY_SECS;
                let idle = nowt.saturating_sub(*r.last_active.lock().unwrap()) > ttl;
                !(empty && idle)
            });
            persistence::save(&bg);
        }
    });

    // save on Ctrl-C / SIGTERM
    let sg = state.clone();
    actix_web::rt::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        persistence::save(&sg);
        eprintln!("shutdown: state saved");
        std::process::exit(0);
    });

    let app_origin = state.cfg.app_origin.clone();
    let dev = state.cfg.dev;
    eprintln!("emoji-niwa-server listening on {bind} (app_origin={app_origin}, dev={dev})");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(api::cors(app_origin.clone(), dev))
            .configure(api::routes)
    })
    .bind(&bind)?
    .run()
    .await
}
