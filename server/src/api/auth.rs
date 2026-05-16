//! GitHub OAuth (confidential client, state CSRF). Adapter layer.

use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::api::{auth_uid, bearer};
use crate::domain::{now_secs, rand_token, urlenc, Member, Session, OAUTH_STATE_TTL, SESSION_TTL};
use crate::state::Data;

pub async fn login(state: Data) -> HttpResponse {
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
pub struct CallbackQ {
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

pub async fn callback(state: Data, q: web::Query<CallbackQ>) -> HttpResponse {
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
    let dest = format!("{}/#auth={}", state.cfg.app_url, token);
    HttpResponse::Found()
        .insert_header(("Location", dest))
        .finish()
}

pub async fn me(state: Data, req: HttpRequest) -> HttpResponse {
    match auth_uid(&state, &req) {
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

pub async fn logout(state: Data, req: HttpRequest) -> HttpResponse {
    if let Some(tok) = bearer(&req) {
        state.sessions.remove(&tok);
    }
    HttpResponse::NoContent().finish()
}
