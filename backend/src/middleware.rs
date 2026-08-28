use crate::config::Config;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

#[derive(Clone)]
pub struct UserInfo {
    pub username: String,
    pub name: String,
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let config = req
        .extensions()
        .get::<Config>()
        .expect("Config not found in extensions")
        .clone();

    if config.profile == "dev" {
        req.extensions_mut().insert(UserInfo {
            username: config.dev_username.clone(),
            name: config.dev_name.clone(),
        });
        return Ok(next.run(req).await);
    }

    let username = req
        .headers()
        .get("X-authentik-username")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let name = req
        .headers()
        .get("X-authentik-name")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if let (Some(u), Some(n)) = (username, name) {
        if !u.is_empty() && !n.is_empty() {
            req.extensions_mut().insert(UserInfo {
                username: u,
                name: n,
            });
            return Ok(next.run(req).await);
        }
    }

    tracing::warn!("Unauthorized request missing X-authentik headers");
    Err(StatusCode::UNAUTHORIZED)
}
