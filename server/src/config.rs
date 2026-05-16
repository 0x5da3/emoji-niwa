//! Configuration loaded from the environment.

use crate::domain::url_origin;

pub struct Config {
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub app_url: String, // full browser app URL incl. path (post-login redirect target)
    pub app_origin: String, // origin (scheme://host[:port]) derived from app_url, CORS/WS match
    pub public_base: String, // this server's external base URL, for the OAuth redirect_uri
    pub data_path: String,
    pub dev: bool,         // DEV=1 → also allow localhost origins (off in production)
    pub max_snap_bytes: usize, // reject oversize world snapshots (abuse/amplification guard)
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

impl Config {
    pub fn from_env() -> Config {
        // APP_URL = full app URL incl. path (post-login redirect target).
        // APP_ORIGIN is a deprecated alias accepted for backward compatibility.
        let app_url = std::env::var("APP_URL")
            .or_else(|_| std::env::var("APP_ORIGIN"))
            .unwrap_or_else(|_| "http://localhost:8000".into())
            .trim_end_matches('/')
            .to_string();
        Config {
            gh_client_id: env("GH_CLIENT_ID"),
            gh_client_secret: env("GH_CLIENT_SECRET"),
            app_origin: url_origin(&app_url), // scheme://host[:port] for CORS/WS match
            app_url,
            public_base: std::env::var("PUBLIC_BASE")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            data_path: std::env::var("DATA_PATH")
                .unwrap_or_else(|_| "./data/state.json".into()),
            dev: std::env::var("DEV")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            max_snap_bytes: std::env::var("MAX_SNAP_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(262144), // 256 KB; raise via env, or chunk if ever needed
        }
    }
}
