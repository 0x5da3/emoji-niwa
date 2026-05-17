//! Room issuance (member-only) + WebSocket join/relay. Adapter layer.

use std::sync::atomic::Ordering;

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt;
use tokio::sync::broadcast;

use serde::Deserialize;

use crate::api::auth_uid;
use crate::domain::{clamp_ttl_days, origin_ok, rand_room_id, Bcast, Room};
use crate::state::Data;

pub async fn new(state: Data, req: HttpRequest) -> HttpResponse {
    let uid = match auth_uid(&state, &req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().finish(),
    };
    let mut id = rand_room_id();
    while state.rooms.contains_key(&id) {
        id = rand_room_id();
    }
    let ttl = state.cfg.room_ttl_days;
    state.rooms.insert(id.clone(), Room::new(uid, None, ttl));
    HttpResponse::Ok().json(serde_json::json!({ "id": id }))
}

#[derive(Deserialize)]
pub struct TtlReq {
    pub days: u64,
}

/// オーナー（部屋の作成会員）のみ、空き部屋の保持日数を変更。値は [1,30] にクランプ。
pub async fn set_ttl(
    state: Data,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<TtlReq>,
) -> HttpResponse {
    let uid = match auth_uid(&state, &req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().finish(),
    };
    let room_id = path.into_inner();
    let room = match state.rooms.get(&room_id) {
        Some(r) => r.value().clone(),
        None => return HttpResponse::NotFound().finish(),
    };
    if room.creator_uid != uid {
        return HttpResponse::Forbidden().finish();
    }
    let days = clamp_ttl_days(body.days);
    *room.ttl_days.lock().unwrap() = days;
    let _ = room.tx.send(Bcast::Refresh); // 接続中の所有者UIへ反映
    HttpResponse::Ok().json(serde_json::json!({ "ok": true, "days": days }))
}

pub async fn ws(
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
    let cap = state.cfg.max_room_peers;
    let (response, mut session, msg_stream) = actix_ws::handle(&req, body)?;
    let mut msg_stream = msg_stream
        .aggregate_continuations()
        .max_continuation_size(max_snap.max(64 * 1024) + 4096);
    let conn_id = state.conn_seq.fetch_add(1, Ordering::Relaxed);
    let mut rx = room.tx.subscribe();

    // Capacity gate (atomic check-and-insert): reject if the room is full.
    let full = {
        let mut c = room.conns.lock().unwrap();
        if c.len() >= state.cfg.max_room_peers {
            true
        } else {
            c.insert(conn_id, String::new()); // name set on `hello`
            false
        }
    };
    if full {
        let _ = session
            .text(serde_json::json!({ "t": "full" }).to_string())
            .await;
        let _ = session.close(None).await;
        return Ok(response);
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
                                    match v.get("t").and_then(|x| x.as_str()) {
                                        Some("snap") => {
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
                                        Some("hello") => {
                                            let nm: String = v
                                                .get("name")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .chars()
                                                .filter(|c| !c.is_control())
                                                .take(24)
                                                .collect::<String>()
                                                .trim()
                                                .to_string();
                                            if let Some(slot) =
                                                room2.conns.lock().unwrap().get_mut(&conn_id)
                                            {
                                                *slot = nm;
                                            }
                                            let _ = room2.tx.send(Bcast::Refresh);
                                        }
                                        Some("chat") => {
                                            let text: String = v
                                                .get("text")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .chars()
                                                .filter(|c| !c.is_control() || *c == ' ')
                                                .take(400)
                                                .collect::<String>()
                                                .trim()
                                                .to_string();
                                            if !text.is_empty() {
                                                let name = room2
                                                    .conns
                                                    .lock()
                                                    .unwrap()
                                                    .get(&conn_id)
                                                    .cloned()
                                                    .unwrap_or_default();
                                                let _ = room2
                                                    .tx
                                                    .send(Bcast::Chat { name, text });
                                            }
                                        }
                                        _ => {}
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
                            let (n, owner, names) = {
                                let c = room2.conns.lock().unwrap();
                                let names: Vec<String> = c
                                    .values()
                                    .filter(|s| !s.is_empty())
                                    .cloned()
                                    .collect();
                                (
                                    c.len(),
                                    c.keys().next().copied() == Some(conn_id),
                                    names,
                                )
                            };
                            let _ = session
                                .text(
                                    serde_json::json!({ "t": "peers", "n": n, "cap": cap, "names": names })
                                        .to_string(),
                                )
                                .await;
                            let ttl_days = *room2.ttl_days.lock().unwrap();
                            if session
                                .text(serde_json::json!({ "t": "role", "owner": owner, "ttlDays": ttl_days }).to_string())
                                .await
                                .is_err()
                            { break; }
                        }
                        Ok(Bcast::Chat { name, text }) => {
                            if session
                                .text(
                                    serde_json::json!({ "t": "chat", "name": name, "text": text })
                                        .to_string(),
                                )
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
