//! Domain layer: core entities + pure helpers. No web framework deps.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub const SESSION_TTL: u64 = 30 * 24 * 3600; // 30 days
pub const OAUTH_STATE_TTL: u64 = 600; // 10 min
pub const ROOM_IDLE_TTL: u64 = 6 * 3600; // evict empty rooms idle > 6h
pub const SAVE_INTERVAL: u64 = 15; // persistence flush seconds
pub const BCAST_CAP: usize = 64;

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Member {
    pub uid: String,
    pub login: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub uid: String,
    pub exp: u64,
}

#[derive(Clone)]
pub enum Bcast {
    Snap { from: u64, d: String },
    Refresh,
}

/// One collaborative room: latest world snapshot + fan-out channel + members.
pub struct Room {
    pub snap: Mutex<Option<String>>,
    pub tx: broadcast::Sender<Bcast>,
    pub conns: Mutex<BTreeSet<u64>>,
    pub creator_uid: String,
    pub last_active: Mutex<u64>,
}

impl Room {
    pub fn new(creator_uid: String, snap: Option<String>) -> Arc<Room> {
        let (tx, _rx) = broadcast::channel(BCAST_CAP);
        Arc::new(Room {
            snap: Mutex::new(snap),
            tx,
            conns: Mutex::new(BTreeSet::new()),
            creator_uid,
            last_active: Mutex::new(now_secs()),
        })
    }
    pub fn touch(&self) {
        *self.last_active.lock().unwrap() = now_secs();
    }
}

/// Origin (scheme://host[:port], no path) of an http(s) URL.
/// e.g. `https://0x5da3.github.io/emoji-niwa` → `https://0x5da3.github.io`.
pub fn url_origin(u: &str) -> String {
    if let Some(i) = u.find("://") {
        let after = &u[i + 3..];
        let host = after.split('/').next().unwrap_or(after);
        return format!("{}://{}", &u[..i], host);
    }
    u.to_string()
}

/// Single source of truth for allowed browser origins (CORS + WS handshake).
/// Production = exactly the app origin (derived from APP_URL). localhost/
/// 127.0.0.1 (any port) only when DEV. Exact host match — no loose prefix
/// (avoids `http://localhost.evil.com`).
pub fn origin_ok(origin: &str, app_origin: &str, dev: bool) -> bool {
    if origin == app_origin {
        return true;
    }
    if dev {
        if let Some(rest) = origin.strip_prefix("http://") {
            let host = rest.split('/').next().unwrap_or("");
            let host = host.split(':').next().unwrap_or("");
            return host == "localhost" || host == "127.0.0.1";
        }
    }
    false
}

pub fn rand_token(n: usize) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut rng = rand::thread_rng();
    (0..n).map(|_| HEX[rng.gen_range(0..16)] as char).collect()
}

pub fn rand_room_id() -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..18)
        .map(|_| A[rng.gen_range(0..A.len())] as char)
        .collect()
}

/// Minimal application/x-www-form-urlencoded component encoder.
pub fn urlenc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
