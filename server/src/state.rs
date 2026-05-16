//! Application state: in-memory stores + shared resources.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use dashmap::DashMap;

use crate::config::Config;
use crate::domain::{now_secs, Member, Room, Session};

pub struct AppState {
    pub rooms: DashMap<String, Arc<Room>>,
    pub sessions: DashMap<String, Session>,
    pub members: DashMap<String, Member>,
    pub oauth_states: DashMap<String, u64>, // state -> expiry secs
    pub conn_seq: AtomicU64,
    pub cfg: Config,
    pub http: reqwest::Client,
}

pub type Data = actix_web::web::Data<AppState>;

impl AppState {
    pub fn new(cfg: Config) -> AppState {
        AppState {
            rooms: DashMap::new(),
            sessions: DashMap::new(),
            members: DashMap::new(),
            oauth_states: DashMap::new(),
            conn_seq: AtomicU64::new(1),
            cfg,
            http: reqwest::Client::new(),
        }
    }

    /// Resolve a session token → member uid, pruning if expired.
    pub fn session_uid(&self, token: &str) -> Option<String> {
        let s = self.sessions.get(token)?;
        if s.exp < now_secs() {
            drop(s);
            self.sessions.remove(token);
            return None;
        }
        Some(s.uid.clone())
    }
}
