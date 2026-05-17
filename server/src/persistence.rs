//! Snapshot persistence (filesystem adapter): periodic + on-shutdown save,
//! restore on boot. Single instance / friend scale.

use serde::{Deserialize, Serialize};

use crate::domain::{now_secs, Member, Room, Session};
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
struct PersistRoom {
    id: String,
    snap: Option<String>,
    creator_uid: String,
    #[serde(default)]
    ttl_days: Option<u64>, // 旧 state.json は無 → 既定値で補完
}

#[derive(Serialize, Deserialize, Default)]
struct Persist {
    rooms: Vec<PersistRoom>,
    sessions: Vec<(String, Session)>,
    members: Vec<Member>,
}

pub fn save(state: &AppState) {
    let mut p = Persist::default();
    for e in state.rooms.iter() {
        p.rooms.push(PersistRoom {
            id: e.key().clone(),
            snap: e.value().snap.lock().unwrap().clone(),
            creator_uid: e.value().creator_uid.clone(),
            ttl_days: Some(*e.value().ttl_days.lock().unwrap()),
        });
    }
    let nowt = now_secs();
    for e in state.sessions.iter() {
        if e.value().exp >= nowt {
            p.sessions.push((e.key().clone(), e.value().clone()));
        }
    }
    for e in state.members.iter() {
        p.members.push(e.value().clone());
    }
    let path = &state.cfg.data_path;
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_vec(&p) {
        Ok(bytes) => {
            let tmp = format!("{path}.tmp");
            if std::fs::write(&tmp, &bytes).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
        Err(e) => eprintln!("save serialize error: {e}"),
    }
}

pub fn load(state: &AppState) {
    let raw = match std::fs::read(&state.cfg.data_path) {
        Ok(r) => r,
        Err(_) => return,
    };
    let p: Persist = match serde_json::from_slice(&raw) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("load parse error: {e}");
            return;
        }
    };
    for r in p.rooms {
        let ttl = r.ttl_days.unwrap_or(state.cfg.room_ttl_days);
        state
            .rooms
            .insert(r.id, Room::new(r.creator_uid, r.snap, ttl));
    }
    let nowt = now_secs();
    for (t, s) in p.sessions {
        if s.exp >= nowt {
            state.sessions.insert(t, s);
        }
    }
    for m in p.members {
        state.members.insert(m.uid.clone(), m);
    }
    eprintln!(
        "loaded {} rooms, {} sessions, {} members",
        state.rooms.len(),
        state.sessions.len(),
        state.members.len()
    );
}
