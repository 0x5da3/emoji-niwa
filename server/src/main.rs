//! emoji-niwa multiplayer backend (Actix Web + actix-ws).
//!
//! - Members log in with GitHub OAuth (state-based CSRF, confidential client).
//! - Only members can ISSUE a room (`POST /room/new`). Joining `/room/{id}` is open.
//! - The server is a dumb relay: it keeps each room's latest compact world
//!   snapshot (the client's `encodeWorld` string, opaque here) and fans it out.
//! - Rooms/sessions/members live in-process and are snapshotted to disk
//!   periodically and on shutdown, restored on boot.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_ws::AggregatedMessage;
use dashmap::DashMap;
use futures_util::StreamExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

const SESSION_TTL: u64 = 30 * 24 * 3600; // 30 days
const OAUTH_STATE_TTL: u64 = 600; // 10 min
const ROOM_IDLE_TTL: u64 = 6 * 3600; // evict empty rooms idle > 6h
const SAVE_INTERVAL: u64 = 15; // persistence flush seconds
const BCAST_CAP: usize = 64;

#[derive(Clone, Serialize, Deserialize)]
struct Member {
    uid: String,
    login: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Session {
    uid: String,
    exp: u64,
}

#[derive(Clone)]
enum Bcast {
    Snap { from: u64, d: String },
    Refresh,
}

struct Room {
    snap: Mutex<Option<String>>,
    tx: broadcast::Sender<Bcast>,
    conns: Mutex<BTreeSet<u64>>,
    creator_uid: String,
    last_active: Mutex<u64>,
}

impl Room {
    fn new(creator_uid: String, snap: Option<String>) -> Arc<Room> {
        let (tx, _rx) = broadcast::channel(BCAST_CAP);
        Arc::new(Room {
            snap: Mutex::new(snap),
            tx,
            conns: Mutex::new(BTreeSet::new()),
            creator_uid,
            last_active: Mutex::new(now_secs()),
        })
    }
    fn touch(&self) {
        *self.last_active.lock().unwrap() = now_secs();
    }
}

struct Config {
    gh_client_id: String,
    gh_client_secret: String,
    app_origin: String,  // where the browser app lives (GitHub Pages), for post-auth redirect
    public_base: String, // this server's external base URL, for the OAuth redirect_uri
    data_path: String,
    dev: bool,           // DEV=1 → also allow localhost origins (off in production)
    max_snap_bytes: usize, // reject oversize world snapshots (abuse/amplification guard)
}

/// Single source of truth for allowed browser origins (CORS + WS handshake).
/// Production = exactly APP_ORIGIN. localhost/127.0.0.1 (any port) only when DEV.
/// Exact host match — no loose prefix (avoids `http://localhost.evil.com`).
fn origin_ok(origin: &str, app_origin: &str, dev: bool) -> bool {
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

struct AppState {
    rooms: DashMap<String, Arc<Room>>,
    sessions: DashMap<String, Session>,
    members: DashMap<String, Member>,
    oauth_states: DashMap<String, u64>, // state -> expiry secs
    conn_seq: AtomicU64,
    cfg: Config,
    http: reqwest::Client,
}

type Data = web::Data<AppState>;

fn rand_token(n: usize) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut rng = rand::thread_rng();
    (0..n).map(|_| HEX[rng.gen_range(0..16)] as char).collect()
}

fn rand_room_id() -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..18).map(|_| A[rng.gen_range(0..A.len())] as char).collect()
}

fn bearer(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

fn valid_session<'a>(state: &'a AppState, req: &HttpRequest) -> Option<String> {
    let tok = bearer(req)?;
    let s = state.sessions.get(&tok)?;
    if s.exp < now_secs() {
        drop(s);
        state.sessions.remove(&tok);
        return None;
    }
    Some(s.uid.clone())
}

// ── persistence ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct PersistRoom {
    id: String,
    snap: Option<String>,
    creator_uid: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Persist {
    rooms: Vec<PersistRoom>,
    sessions: Vec<(String, Session)>,
    members: Vec<Member>,
}

fn save_state(state: &AppState) {
    let mut p = Persist::default();
    for e in state.rooms.iter() {
        p.rooms.push(PersistRoom {
            id: e.key().clone(),
            snap: e.value().snap.lock().unwrap().clone(),
            creator_uid: e.value().creator_uid.clone(),
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
        Err(e) => eprintln!("save_state serialize error: {e}"),
    }
}

fn load_state(state: &AppState) {
    let raw = match std::fs::read(&state.cfg.data_path) {
        Ok(r) => r,
        Err(_) => return,
    };
    let p: Persist = match serde_json::from_slice(&raw) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("load_state parse error: {e}");
            return;
        }
    };
    for r in p.rooms {
        state
            .rooms
            .insert(r.id, Room::new(r.creator_uid, r.snap));
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

// ── auth (GitHub OAuth, confidential client, state CSRF) ─────────────────────

async fn auth_login(state: Data) -> HttpResponse {
    let st = rand_token(32);
    state
        .oauth_states
        .insert(st.clone(), now_secs() + OAUTH_STATE_TTL);
    let redirect_uri = format!("{}/auth/callback", state.cfg.public_base);
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user&state={}",
        urlenc(&state.cfg.gh_client_id),
        urlenc(&redirect_uri),
        urlenc(&st)
    );
    HttpResponse::Found()
        .insert_header(("Location", url))
        .finish()
}

#[derive(Deserialize)]
struct CallbackQ {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
struct GhToken {
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct GhUser {
    id: i64,
    login: String,
}

async fn auth_callback(state: Data, q: web::Query<CallbackQ>) -> HttpResponse {
    let (code, st) = match (q.code.clone(), q.state.clone()) {
        (Some(c), Some(s)) => (c, s),
        _ => return HttpResponse::BadRequest().body("missing code/state"),
    };
    // validate + consume state
    match state.oauth_states.remove(&st) {
        Some((_, exp)) if exp >= now_secs() => {}
        _ => return HttpResponse::BadRequest().body("invalid state"),
    }
    let redirect_uri = format!("{}/auth/callback", state.cfg.public_base);
    let tok: GhToken = match state
        .http
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": state.cfg.gh_client_id,
            "client_secret": state.cfg.gh_client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
            "state": st,
        }))
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => match r.json().await {
            Ok(t) => t,
            Err(e) => return HttpResponse::BadGateway().body(format!("token parse: {e}")),
        },
        Err(e) => return HttpResponse::BadGateway().body(format!("token exchange: {e}")),
    };
    let access = match tok.access_token {
        Some(a) => a,
        None => return HttpResponse::BadGateway().body("no access_token"),
    };
    let user: GhUser = match state
        .http
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access}"))
        .header("User-Agent", "emoji-niwa")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(e) => return HttpResponse::BadGateway().body(format!("user parse: {e}")),
        },
        Err(e) => return HttpResponse::BadGateway().body(format!("user fetch: {e}")),
    };
    let uid = format!("gh:{}", user.id);
    state.members.insert(
        uid.clone(),
        Member {
            uid: uid.clone(),
            login: user.login.clone(),
        },
    );
    let token = rand_token(48);
    state.sessions.insert(
        token.clone(),
        Session {
            uid,
            exp: now_secs() + SESSION_TTL,
        },
    );
    // Deliver token via fragment (not query → not logged), client stores & strips.
    let dest = format!("{}/#auth={}", state.cfg.app_origin, token);
    HttpResponse::Found()
        .insert_header(("Location", dest))
        .finish()
}

async fn auth_me(state: Data, req: HttpRequest) -> HttpResponse {
    match valid_session(&state, &req) {
        Some(uid) => {
            let login = state
                .members
                .get(&uid)
                .map(|m| m.login.clone())
                .unwrap_or_default();
            HttpResponse::Ok().json(serde_json::json!({ "uid": uid, "login": login }))
        }
        None => HttpResponse::Unauthorized().finish(),
    }
}

async fn auth_logout(state: Data, req: HttpRequest) -> HttpResponse {
    if let Some(tok) = bearer(&req) {
        state.sessions.remove(&tok);
    }
    HttpResponse::NoContent().finish()
}

// ── rooms ────────────────────────────────────────────────────────────────────

async fn room_new(state: Data, req: HttpRequest) -> HttpResponse {
    let uid = match valid_session(&state, &req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().finish(),
    };
    let mut id = rand_room_id();
    while state.rooms.contains_key(&id) {
        id = rand_room_id();
    }
    state.rooms.insert(id.clone(), Room::new(uid, None));
    HttpResponse::Ok().json(serde_json::json!({ "id": id }))
}

async fn room_ws(
    state: Data,
    req: HttpRequest,
    body: web::Payload,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let room_id = path.into_inner();
    let room = match state.rooms.get(&room_id) {
        Some(r) => r.value().clone(),
        None => {
            // Only member-issued rooms exist; unknown id = not joinable.
            return Ok(HttpResponse::NotFound().body("no such room"));
        }
    };

    // WebSocket is not covered by CORS — enforce the same origin allowlist here.
    let origin_allowed = match req.headers().get("origin").and_then(|h| h.to_str().ok()) {
        Some(o) => origin_ok(o, &state.cfg.app_origin, state.cfg.dev),
        None => state.cfg.dev, // browsers always send Origin for WS; blank only ok in dev
    };
    if !origin_allowed {
        return Ok(HttpResponse::Forbidden().body("origin not allowed"));
    }

    let max_snap = state.cfg.max_snap_bytes;
    let (response, mut session, msg_stream) = actix_ws::handle(&req, body)?;
    let mut msg_stream = msg_stream
        .aggregate_continuations()
        .max_continuation_size(max_snap.max(64 * 1024) + 4096);
    let conn_id = state.conn_seq.fetch_add(1, Ordering::Relaxed);
    let mut rx = room.tx.subscribe();

    {
        let mut c = room.conns.lock().unwrap();
        c.insert(conn_id);
    }
    room.touch();

    // initial snapshot to the newcomer
    if let Some(d) = room.snap.lock().unwrap().clone() {
        let _ = session
            .text(serde_json::json!({ "t": "snap", "d": d }).to_string())
            .await;
    }
    let _ = room.tx.send(Bcast::Refresh);

    let room2 = room.clone();
    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(AggregatedMessage::Text(t))) => {
                            if t.len() > 2 * max_snap { break; }     // egregious → drop connection
                            if t.len() <= max_snap {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                                    if v.get("t").and_then(|x| x.as_str()) == Some("snap") {
                                        if let Some(d) = v.get("d").and_then(|x| x.as_str()) {
                                            if d.len() <= max_snap {
                                                *room2.snap.lock().unwrap() = Some(d.to_string());
                                                room2.touch();
                                                let _ = room2.tx.send(Bcast::Snap {
                                                    from: conn_id,
                                                    d: d.to_string(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            // max_snap < len <= 2*max_snap → ignore silently
                        }
                        Some(Ok(AggregatedMessage::Ping(p))) => {
                            let _ = session.pong(&p).await;
                        }
                        Some(Ok(AggregatedMessage::Close(_))) | None => break,
                        Some(Err(_)) => break,
                        _ => {}
                    }
                }
                b = rx.recv() => {
                    match b {
                        Ok(Bcast::Snap { from, d }) => {
                            if from != conn_id {
                                if session
                                    .text(serde_json::json!({ "t": "snap", "d": d }).to_string())
                                    .await
                                    .is_err()
                                { break; }
                            }
                        }
                        Ok(Bcast::Refresh) => {
                            let (n, owner) = {
                                let c = room2.conns.lock().unwrap();
                                (c.len(), c.iter().next().copied() == Some(conn_id))
                            };
                            let _ = session
                                .text(serde_json::json!({ "t": "peers", "n": n }).to_string())
                                .await;
                            if session
                                .text(serde_json::json!({ "t": "role", "owner": owner }).to_string())
                                .await
                                .is_err()
                            { break; }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        // cleanup
        {
            let mut c = room2.conns.lock().unwrap();
            c.remove(&conn_id);
        }
        room2.touch();
        let _ = room2.tx.send(Bcast::Refresh);
        let _ = session.close(None).await;
    });

    Ok(response)
}

async fn healthz() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

// minimal application/x-www-form-urlencoded component encoder
fn urlenc(s: &str) -> String {
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

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cfg = Config {
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        app_origin: std::env::var("APP_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:8000".into()),
        public_base: std::env::var("PUBLIC_BASE")
            .unwrap_or_else(|_| "http://localhost:8080".into()),
        data_path: std::env::var("DATA_PATH").unwrap_or_else(|_| "./data/state.json".into()),
        dev: std::env::var("DEV")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false),
        max_snap_bytes: std::env::var("MAX_SNAP_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(262144), // 256 KB; raise via env, or chunk if ever needed
    };
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    if cfg.gh_client_id.is_empty() || cfg.gh_client_secret.is_empty() {
        eprintln!("WARNING: GH_CLIENT_ID / GH_CLIENT_SECRET not set — auth will fail.");
    }

    let state = web::Data::new(AppState {
        rooms: DashMap::new(),
        sessions: DashMap::new(),
        members: DashMap::new(),
        oauth_states: DashMap::new(),
        conn_seq: AtomicU64::new(1),
        cfg,
        http: reqwest::Client::new(),
    });
    load_state(&state);

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
                let idle = nowt.saturating_sub(*r.last_active.lock().unwrap()) > ROOM_IDLE_TTL;
                !(empty && idle)
            });
            save_state(&bg);
        }
    });

    // save on Ctrl-C / SIGTERM
    let sg = state.clone();
    actix_web::rt::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        save_state(&sg);
        eprintln!("shutdown: state saved");
        std::process::exit(0);
    });

    let app_origin = state.cfg.app_origin.clone();
    let dev = state.cfg.dev;
    eprintln!(
        "emoji-niwa-server listening on {bind} (app_origin={app_origin}, dev={dev})"
    );

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn({
                let ao = app_origin.clone();
                move |origin, _req| {
                    origin
                        .to_str()
                        .map(|o| origin_ok(o, &ao, dev))
                        .unwrap_or(false)
                }
            })
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        App::new()
            .app_data(state.clone())
            .wrap(cors)
            .route("/healthz", web::get().to(healthz))
            .route("/auth/login", web::get().to(auth_login))
            .route("/auth/callback", web::get().to(auth_callback))
            .route("/auth/me", web::get().to(auth_me))
            .route("/auth/logout", web::post().to(auth_logout))
            .route("/room/new", web::post().to(room_new))
            .route("/room/{id}", web::get().to(room_ws))
    })
    .bind(&bind)?
    .run()
    .await
}
