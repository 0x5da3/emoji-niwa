//! Configuration loaded from the environment.

use crate::domain::{clamp_ttl_days, url_origin};

pub struct Config {
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub app_url: String, // full browser app URL incl. path (post-login redirect target)
    pub app_origin: String, // origin (scheme://host[:port]) derived from app_url, CORS/WS match
    pub public_base: String, // this server's external base URL, for the OAuth redirect_uri
    pub data_path: String,
    pub dev: bool,         // DEV=1 → also allow localhost origins (off in production)
    pub max_snap_bytes: usize, // reject oversize world snapshots (abuse/amplification guard)
    pub allowed_logins: Vec<String>, // GitHub logins (lowercased) allowed to log in; empty = open
    pub max_room_peers: usize, // max concurrent connections per room (full → rejected)
    pub room_ttl_days: u64, // default empty-room retention (days); per-room override允許
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
            // ALLOWED_LOGINS = comma-separated GitHub usernames allowed to log in.
            // Empty/unset = open to any GitHub account (backward compatible).
            allowed_logins: std::env::var("ALLOWED_LOGINS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            // Max concurrent participants per room (incl. creator). Default 8.
            max_room_peers: std::env::var("MAX_ROOM_PEERS")
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|&n| n >= 1)
                .unwrap_or(8),
            // 空き部屋のデフォルト保持日数。env で運用者が調整可。既定 7、
            // 範囲外/未設定は [1,30] にクランプ（7 をフォールバック）。
            room_ttl_days: std::env::var("ROOM_TTL_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(clamp_ttl_days)
                .unwrap_or(7),
        }
    }
}
