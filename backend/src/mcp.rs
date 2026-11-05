use axum::{
    body::Bytes,
    extract::{OriginalUri, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use ring::{digest, hmac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

use crate::{db, handlers::AppState, models::Reminder};

const TOKEN_EXPIRES_IN_SECS: u64 = 2_592_000; // 30 days
const AUTH_CODE_MAX_AGE_SECS: i64 = 600; // 10 minutes

#[derive(Debug, Deserialize, Default)]
pub struct AuthorizeParams {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TokenRequestParams {
    pub grant_type: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthCodePayload {
    u: String,
    cc: String,
    ccm: String,
    ts: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenPayload {
    u: String,
    kind: String,
    ts: i64,
}

fn determine_base_url(state: &AppState, headers: &HeaderMap) -> String {
    if let Some(configured_base) = state.config.get_file_config().base_url {
        let trimmed = configured_base.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.split(',').next())
        .map(|h| h.trim().to_string())
        .unwrap_or_else(|| format!("localhost:{}", state.config.port));

    let is_local = host.starts_with("localhost") || host.starts_with("127.0.0.1");
    let proto = if is_local {
        headers
            .get("x-forwarded-proto")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(',').next())
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|| "http".to_string())
    } else {
        "https".to_string()
    };

    format!("{}://{}", proto, host)
}

fn sign_data(secret: &str, data: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, data);
    URL_SAFE_NO_PAD.encode(tag.as_ref())
}

fn verify_signature(secret: &str, data: &[u8], sig_b64: &str) -> bool {
    let Ok(sig_bytes) = URL_SAFE_NO_PAD.decode(sig_b64) else {
        return false;
    };
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    hmac::verify(&key, data, &sig_bytes).is_ok()
}

pub fn create_auth_code(
    username: &str,
    secret: &str,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
) -> String {
    let payload = AuthCodePayload {
        u: username.to_string(),
        cc: code_challenge.unwrap_or("").to_string(),
        ccm: code_challenge_method.unwrap_or("S256").to_string(),
        ts: chrono::Utc::now().timestamp(),
    };
    let json_bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let encoded_payload = URL_SAFE_NO_PAD.encode(&json_bytes);
    let sig = sign_data(secret, encoded_payload.as_bytes());
    format!("{}.{}", encoded_payload, sig)
}

fn verify_auth_code(
    code: &str,
    keys: &HashMap<String, String>,
) -> Option<(String, String, String)> {
    let (encoded_payload, sig) = code.split_once('.')?;
    let json_bytes = URL_SAFE_NO_PAD.decode(encoded_payload).ok()?;
    let payload: AuthCodePayload = serde_json::from_slice(&json_bytes).ok()?;
    let secret = keys.get(&payload.u)?;

    if !verify_signature(secret, encoded_payload.as_bytes(), sig) {
        return None;
    }

    let now = chrono::Utc::now().timestamp();
    if (now - payload.ts).abs() > AUTH_CODE_MAX_AGE_SECS {
        return None;
    }

    Some((payload.u, payload.cc, payload.ccm))
}

pub fn create_signed_token(username: &str, secret: &str, kind: &str) -> String {
    let payload = TokenPayload {
        u: username.to_string(),
        kind: kind.to_string(),
        ts: chrono::Utc::now().timestamp(),
    };
    let json_bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let encoded_payload = URL_SAFE_NO_PAD.encode(&json_bytes);
    let sig = sign_data(secret, encoded_payload.as_bytes());
    format!("{}.{}", encoded_payload, sig)
}

pub fn verify_signed_token(
    token: &str,
    expected_kind: &str,
    keys: &HashMap<String, String>,
) -> Option<String> {
    let (encoded_payload, sig) = token.split_once('.')?;
    let json_bytes = URL_SAFE_NO_PAD.decode(encoded_payload).ok()?;
    let payload: TokenPayload = serde_json::from_slice(&json_bytes).ok()?;
    if payload.kind != expected_kind {
        return None;
    }
    let secret = keys.get(&payload.u)?;
    if !verify_signature(secret, encoded_payload.as_bytes(), sig) {
        return None;
    }
    Some(payload.u)
}

fn verify_pkce(code_verifier: &str, code_challenge: &str, method: &str) -> bool {
    if code_challenge.is_empty() {
        return true;
    }
    if method.eq_ignore_ascii_case("plain") {
        return code_verifier == code_challenge;
    }
    // Default to S256
    let digest = digest::digest(&digest::SHA256, code_verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(digest.as_ref());
    computed == code_challenge
}

fn percent_decode_str(input: &str) -> String {
    url::form_urlencoded::parse(input.as_bytes())
        .next()
        .map(|(k, v)| {
            if v.is_empty() && !input.contains('=') {
                k.into_owned()
            } else {
                format!("{}={}", k, v)
            }
        })
        .unwrap_or_else(|| input.to_string())
}

fn extract_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let auth_header = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let b64 = auth_header
        .strip_prefix("Basic ")
        .or_else(|| auth_header.strip_prefix("basic "))?
        .trim();
    let decoded_bytes = STANDARD
        .decode(b64)
        .or_else(|_| URL_SAFE_NO_PAD.decode(b64))
        .ok()?;
    let decoded_str = String::from_utf8(decoded_bytes).ok()?;
    let (id, secret) = decoded_str.split_once(':')?;
    Some((id.to_string(), secret.to_string()))
}

fn verify_client_credentials(
    client_id: &str,
    client_secret: &str,
    keys: &HashMap<String, String>,
) -> Option<String> {
    if let Some(expected) = keys.get(client_id) {
        if expected == client_secret {
            return Some(client_id.to_string());
        }
    }
    let decoded_id = percent_decode_str(client_id);
    let decoded_secret = percent_decode_str(client_secret);
    if let Some(expected) = keys.get(&decoded_id) {
        if expected == &decoded_secret || expected == client_secret {
            return Some(decoded_id);
        }
    }
    None
}

fn authenticate_mcp_request(headers: &HeaderMap, keys: &HashMap<String, String>) -> Option<String> {
    if let Some((id, secret)) = extract_basic_auth(headers) {
        if let Some(user) = verify_client_credentials(&id, &secret, keys) {
            return Some(user);
        }
    }

    let auth_header = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))?
        .trim();

    if let Some(user) = verify_signed_token(token, "access", keys) {
        return Some(user);
    }

    if let Some((id, secret)) = token.split_once('=').or_else(|| token.split_once(':')) {
        if let Some(user) = verify_client_credentials(id, secret, keys) {
            return Some(user);
        }
    }

    None
}

fn unauthorized_mcp_response(state: &AppState, headers: &HeaderMap) -> Response {
    let base_url = determine_base_url(state, headers);
    let www_auth = format!(
        "Bearer resource_metadata=\"{}/mcp/.well-known/oauth-protected-resource\"",
        base_url
    );
    tracing::info!(
        "MCP unauthorized request -> returning 401 with WWW-Authenticate: {}",
        www_auth
    );
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, www_auth)],
        Json(json!({
            "error": "unauthorized",
            "error_description": "Valid Bearer token required"
        })),
    )
        .into_response()
}

pub async fn oauth_protected_resource_metadata(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> impl IntoResponse {
    let base_url = determine_base_url(&state, &headers);
    let auth_server = if uri.path().starts_with("/mcp/") || uri.path().ends_with("/mcp") {
        format!("{}/mcp", base_url)
    } else {
        base_url.clone()
    };
    let body = json!({
        "resource": format!("{}/mcp", base_url),
        "authorization_servers": [
            auth_server
        ],
        "bearer_methods_supported": ["header"],
        "scopes_supported": ["mcp"]
    });
    tracing::info!(
        "OAuth protected resource metadata requested at '{}' (headers: {:?}) -> returning: {}",
        uri,
        headers,
        body
    );
    Json(body)
}

pub async fn oauth_authorization_server_metadata(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> impl IntoResponse {
    let base_url = determine_base_url(&state, &headers);
    let issuer = if uri.path().ends_with("/mcp") || uri.path().starts_with("/mcp/") {
        format!("{}/mcp", base_url)
    } else {
        base_url.clone()
    };
    let body = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{}/mcp/oauth/authorize", base_url),
        "token_endpoint": format!("{}/mcp/oauth/token", base_url),
        "registration_endpoint": format!("{}/mcp/oauth/register", base_url),
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token", "client_credentials"],
        "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic", "none"],
        "code_challenge_methods_supported": ["S256", "plain"],
        "scopes_supported": ["mcp"]
    });
    tracing::info!(
        "OAuth authorization server metadata requested at '{}' (headers: {:?}) -> returning: {}",
        uri,
        headers,
        body
    );
    Json(body)
}

pub async fn oauth_register(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let body_str = String::from_utf8_lossy(&body);
    tracing::info!(
        "OAuth register requested at '{}' (headers: {:?}, body: {})",
        uri,
        headers,
        body_str
    );
    let req_json: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
    let file_cfg = state.config.get_file_config();

    let redirect_uris = if let Some(arr) = req_json.get("redirect_uris").and_then(|v| v.as_array())
    {
        arr.clone()
    } else if let Some(ref cfg_url) = file_cfg.redirect_url {
        vec![Value::String(cfg_url.clone())]
    } else {
        vec![]
    };

    let client_id = file_cfg
        .mcp_keys
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "mcp-client".to_string());

    let resp_body = json!({
        "client_id": client_id,
        "redirect_uris": redirect_uris,
        "grant_types": ["authorization_code", "refresh_token", "client_credentials"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "client_secret_post"
    });
    tracing::info!("OAuth register returning 201: {}", resp_body);
    (StatusCode::CREATED, Json(resp_body))
}

pub async fn oauth_authorize(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Query(query_params): Query<AuthorizeParams>,
    body: Bytes,
) -> Response {
    let body_str = String::from_utf8_lossy(&body);
    tracing::info!(
        "OAuth authorize requested at '{}' (headers: {:?}, query: {:?}, body: '{}')",
        uri,
        headers,
        query_params,
        body_str
    );
    let form_params: AuthorizeParams = if !body.is_empty() {
        url::form_urlencoded::parse(&body).fold(AuthorizeParams::default(), |mut acc, (k, v)| {
            match k.as_ref() {
                "response_type" => acc.response_type = Some(v.into_owned()),
                "client_id" => acc.client_id = Some(v.into_owned()),
                "redirect_uri" => acc.redirect_uri = Some(v.into_owned()),
                "state" => acc.state = Some(v.into_owned()),
                "code_challenge" => acc.code_challenge = Some(v.into_owned()),
                "code_challenge_method" => acc.code_challenge_method = Some(v.into_owned()),
                "scope" => acc.scope = Some(v.into_owned()),
                _ => {}
            }
            acc
        })
    } else {
        AuthorizeParams::default()
    };

    let file_cfg = state.config.get_file_config();

    let client_id = query_params
        .client_id
        .or(form_params.client_id)
        .unwrap_or_default();
    let requested_redirect_uri = query_params.redirect_uri.or(form_params.redirect_uri);

    let redirect_uri = match (requested_redirect_uri, file_cfg.redirect_url.as_deref()) {
        (Some(req_uri), Some(cfg_uri)) if !req_uri.is_empty() => {
            if req_uri != cfg_uri {
                tracing::warn!(
                    "OAuth authorize rejected redirect_uri '{}' (expected '{}')",
                    req_uri,
                    cfg_uri
                );
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "invalid_request",
                        "error_description": "redirect_uri does not match configured redirect_url"
                    })),
                )
                    .into_response();
            }
            req_uri
        }
        (Some(req_uri), None) if !req_uri.is_empty() => req_uri,
        (_, Some(cfg_uri)) => cfg_uri.to_string(),
        _ => String::new(),
    };

    let state_param = query_params.state.or(form_params.state);
    let code_challenge = query_params.code_challenge.or(form_params.code_challenge);
    let code_challenge_method = query_params
        .code_challenge_method
        .or(form_params.code_challenge_method);

    if client_id.is_empty() || redirect_uri.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_request",
                "error_description": "Missing client_id or redirect_uri"
            })),
        )
            .into_response();
    }

    let Some(secret) = file_cfg.mcp_keys.get(&client_id) else {
        tracing::warn!("OAuth authorize rejected unknown client_id: {}", client_id);
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "invalid_client",
                "error_description": "Unknown client_id"
            })),
        )
            .into_response();
    };

    let code = create_auth_code(
        &client_id,
        secret,
        code_challenge.as_deref(),
        code_challenge_method.as_deref(),
    );

    if let Ok(mut target_url) = Url::parse(&redirect_uri) {
        {
            let mut pairs = target_url.query_pairs_mut();
            pairs.append_pair("code", &code);
            if let Some(st) = &state_param {
                pairs.append_pair("state", st);
            }
        }
        tracing::info!(
            "OAuth authorize succeeded for client_id '{}', redirecting to '{}'",
            client_id,
            target_url
        );
        Redirect::to(target_url.as_str()).into_response()
    } else {
        let sep = if redirect_uri.contains('?') { '&' } else { '?' };
        let mut location = format!("{}{}code={}", redirect_uri, sep, code);
        if let Some(st) = &state_param {
            location.push_str(&format!("&state={}", st));
        }
        tracing::info!(
            "OAuth authorize succeeded for client_id '{}', redirecting to '{}'",
            client_id,
            location
        );
        Redirect::to(&location).into_response()
    }
}

pub async fn oauth_token(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let body_str = String::from_utf8_lossy(&body);
    tracing::info!(
        "OAuth token requested at '{}' (has_auth_header: {}, content_type: {:?}, body_len: {})",
        uri,
        headers.contains_key(header::AUTHORIZATION),
        headers.get(header::CONTENT_TYPE),
        body_str.len()
    );
    let params: TokenRequestParams = if let Ok(json_params) = serde_json::from_slice(&body) {
        json_params
    } else {
        url::form_urlencoded::parse(&body).fold(TokenRequestParams::default(), |mut acc, (k, v)| {
            match k.as_ref() {
                "grant_type" => acc.grant_type = Some(v.into_owned()),
                "code" => acc.code = Some(v.into_owned()),
                "redirect_uri" => acc.redirect_uri = Some(v.into_owned()),
                "client_id" => acc.client_id = Some(v.into_owned()),
                "client_secret" => acc.client_secret = Some(v.into_owned()),
                "code_verifier" => acc.code_verifier = Some(v.into_owned()),
                "refresh_token" => acc.refresh_token = Some(v.into_owned()),
                _ => {}
            }
            acc
        })
    };

    let basic_creds = extract_basic_auth(&headers);
    let client_id = basic_creds
        .as_ref()
        .map(|(id, _)| id.clone())
        .or(params.client_id);
    let client_secret = basic_creds
        .as_ref()
        .map(|(_, sec)| sec.clone())
        .or(params.client_secret);

    let grant_type = params
        .grant_type
        .unwrap_or_else(|| "authorization_code".to_string());
    let file_cfg = state.config.get_file_config();
    let keys = &file_cfg.mcp_keys;

    if let (Some(req_uri), Some(cfg_uri)) = (
        params.redirect_uri.as_deref(),
        file_cfg.redirect_url.as_deref(),
    ) {
        if !req_uri.is_empty() && req_uri != cfg_uri {
            tracing::warn!(
                "OAuth token rejected redirect_uri '{}' (expected '{}')",
                req_uri,
                cfg_uri
            );
            return oauth_error_response(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "redirect_uri does not match configured redirect_url",
            );
        }
    }

    let authenticated_user = match grant_type.as_str() {
        "authorization_code" => {
            let (Some(cid), Some(csec)) = (client_id.as_deref(), client_secret.as_deref()) else {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    "invalid_client",
                    "Client credentials required",
                );
            };
            let Some(verified_user) = verify_client_credentials(cid, csec, keys) else {
                tracing::warn!(
                    "OAuth token rejected invalid credentials for client_id: {}",
                    cid
                );
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    "invalid_client",
                    "Invalid client_id or client_secret",
                );
            };
            let Some(code) = params.code.as_deref() else {
                return oauth_error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "Missing authorization code",
                );
            };
            let Some((code_user, code_challenge, code_challenge_method)) =
                verify_auth_code(code, keys)
            else {
                return oauth_error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "Invalid or expired authorization code",
                );
            };
            if code_user != verified_user {
                return oauth_error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "Authorization code does not match client_id",
                );
            }
            if !code_challenge.is_empty() {
                let verifier = params.code_verifier.as_deref().unwrap_or("");
                if !verify_pkce(verifier, &code_challenge, &code_challenge_method) {
                    return oauth_error_response(
                        StatusCode::BAD_REQUEST,
                        "invalid_grant",
                        "PKCE verification failed",
                    );
                }
            }
            verified_user
        }
        "client_credentials" => {
            let (Some(cid), Some(csec)) = (client_id.as_deref(), client_secret.as_deref()) else {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    "invalid_client",
                    "Client credentials required",
                );
            };
            let Some(verified_user) = verify_client_credentials(cid, csec, keys) else {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    "invalid_client",
                    "Invalid client_id or client_secret",
                );
            };
            verified_user
        }
        "refresh_token" => {
            let Some(rt) = params.refresh_token.as_deref() else {
                return oauth_error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "Missing refresh_token",
                );
            };
            let Some(token_user) = verify_signed_token(rt, "refresh", keys) else {
                return oauth_error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "Invalid refresh_token",
                );
            };
            if let (Some(cid), Some(csec)) = (client_id.as_deref(), client_secret.as_deref()) {
                let Some(verified_user) = verify_client_credentials(cid, csec, keys) else {
                    return oauth_error_response(
                        StatusCode::UNAUTHORIZED,
                        "invalid_client",
                        "Invalid client_id or client_secret",
                    );
                };
                if verified_user != token_user {
                    return oauth_error_response(
                        StatusCode::BAD_REQUEST,
                        "invalid_grant",
                        "refresh_token does not match client_id",
                    );
                }
            }
            token_user
        }
        _ => {
            return oauth_error_response(
                StatusCode::BAD_REQUEST,
                "unsupported_grant_type",
                "Unsupported grant_type",
            );
        }
    };

    let Some(secret) = keys.get(&authenticated_user) else {
        return oauth_error_response(StatusCode::UNAUTHORIZED, "invalid_client", "Unknown client");
    };

    let access_token = create_signed_token(&authenticated_user, secret, "access");
    let refresh_token = create_signed_token(&authenticated_user, secret, "refresh");

    tracing::info!(
        "OAuth token issued successfully for user '{}' (grant_type: '{}')",
        authenticated_user,
        grant_type
    );

    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": TOKEN_EXPIRES_IN_SECS,
            "refresh_token": refresh_token,
            "scope": "mcp"
        })),
    )
        .into_response()
}

fn oauth_error_response(status: StatusCode, error: &str, description: &str) -> Response {
    tracing::warn!(
        "OAuth token error response (status: {}): error='{}', description='{}'",
        status,
        error,
        description
    );
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "error": error,
            "error_description": description
        })),
    )
        .into_response()
}

pub async fn handle_mcp_get(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    tracing::info!("MCP GET request at '{}' (headers: {:?})", uri, headers);
    let keys = state.config.get_mcp_keys();
    let Some(username) = authenticate_mcp_request(&headers, &keys) else {
        return unauthorized_mcp_response(&state, &headers);
    };
    tracing::info!(
        "MCP GET authenticated for user '{}', returning 405 Method Not Allowed",
        username
    );
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

pub async fn handle_mcp_delete(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    tracing::info!("MCP DELETE request at '{}' (headers: {:?})", uri, headers);
    let keys = state.config.get_mcp_keys();
    if authenticate_mcp_request(&headers, &keys).is_none() {
        return unauthorized_mcp_response(&state, &headers);
    }
    StatusCode::OK.into_response()
}

pub async fn handle_mcp_post(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let body_str = String::from_utf8_lossy(&body);
    tracing::info!(
        "MCP POST request at '{}' (headers: {:?}, body: {})",
        uri,
        headers,
        body_str
    );
    let keys = state.config.get_mcp_keys();
    let Some(username) = authenticate_mcp_request(&headers, &keys) else {
        return unauthorized_mcp_response(&state, &headers);
    };
    tracing::info!("MCP POST authenticated as user '{}'", username);

    let payload: Value = match serde_json::from_slice(&body) {
        Ok(val) => val,
        Err(e) => {
            tracing::warn!("Invalid JSON in MCP request: {}", e);
            return (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                })),
            )
                .into_response();
        }
    };

    if let Some(batch) = payload.as_array() {
        let mut responses = Vec::new();
        for item in batch {
            if let Some(resp) = process_jsonrpc_message(&state, &username, item).await {
                responses.push(resp);
            }
        }
        if responses.is_empty() {
            return StatusCode::ACCEPTED.into_response();
        }
        return (StatusCode::OK, Json(Value::Array(responses))).into_response();
    }

    match process_jsonrpc_message(&state, &username, &payload).await {
        Some(resp) => (StatusCode::OK, Json(resp)).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn process_jsonrpc_message(
    state: &Arc<AppState>,
    username: &str,
    msg: &Value,
) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));

    // JSON-RPC notifications do not have an `id` and expect no JSON-RPC response object
    let id = id?;

    match method {
        "initialize" => {
            let requested_version = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2025-03-26");
            let protocol_version = match requested_version {
                "2025-06-18" | "2025-03-26" | "2024-11-05" => requested_version,
                _ => "2025-03-26",
            };
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": protocol_version,
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "reminders-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }))
        }
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "create_reminder",
                        "description": "Create a new reminder for the user",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "reminder_message": {
                                    "type": "string",
                                    "description": "The message content of the reminder to create"
                                }
                            },
                            "required": ["reminder_message"]
                        }
                    },
                    {
                        "name": "list_reminders",
                        "description": "List all existing reminders for the user",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ]
            }
        })),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Some(handle_tool_call(state, username, id, tool_name, &arguments).await)
        }
        "resources/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": []
            }
        })),
        "prompts/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "prompts": []
            }
        })),
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", method)
            }
        })),
    }
}

async fn handle_tool_call(
    state: &Arc<AppState>,
    username: &str,
    id: Value,
    tool_name: &str,
    arguments: &Value,
) -> Value {
    match tool_name {
        "create_reminder" => {
            let message = arguments
                .get("reminder_message")
                .or_else(|| arguments.get("reminderMessage"))
                .or_else(|| arguments.get("message"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if message.is_empty() {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": "Error: reminder_message is required and cannot be empty"
                            }
                        ],
                        "isError": true
                    }
                });
            }

            let reminder = Reminder {
                id: None,
                username: Some(username.to_string()),
                created_time: None,
                reminder_message: message,
            };

            tracing::info!("MCP user {} is creating a new reminder", username);

            match db::save_reminder(&state.db, reminder).await {
                Ok(saved) => {
                    let text = serde_json::to_string(&saved).unwrap_or_default();
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ],
                            "isError": false
                        }
                    })
                }
                Err(e) => {
                    tracing::error!("MCP failed to save reminder: {}", e);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Failed to save reminder: {}", e)
                                }
                            ],
                            "isError": true
                        }
                    })
                }
            }
        }
        "list_reminders" => {
            tracing::info!("MCP user {} is listing reminders", username);
            match db::find_all_by_username(&state.db, username.to_string()).await {
                Ok(reminders) => {
                    let text = serde_json::to_string(&reminders).unwrap_or_default();
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ],
                            "isError": false
                        }
                    })
                }
                Err(e) => {
                    tracing::error!("MCP failed to list reminders: {}", e);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Failed to list reminders: {}", e)
                                }
                            ],
                            "isError": true
                        }
                    })
                }
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": format!("Unknown tool: {}", tool_name)
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    async fn setup_test_state() -> Arc<AppState> {
        let conn = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
        conn.call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS reminders (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username VARCHAR(32) NOT NULL,
                    creation_time DATETIME NOT NULL,
                    reminder_message TEXT NOT NULL
                )",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();

        let mut mcp_keys = HashMap::new();
        mcp_keys.insert("dudu".to_string(), "someSuperSecretKey".to_string());

        let config = Config {
            db_folder: ".".to_string(),
            apprise_url: "http://localhost:8000/notify".to_string(),
            apprise_tag_prefix: "reminders".to_string(),
            dev_username: "dev_user".to_string(),
            dev_name: "Dev User".to_string(),
            port: 8080,
            profile: "prod".to_string(),
            config_file: "/nonexistent/config.yaml".to_string(),
            base_url: Some("https://reminders.example.com".to_string()),
            redirect_url: Some("https://gemini.google.com/callback".to_string()),
            mcp_keys,
        };

        Arc::new(AppState {
            db: conn,
            config,
            web_client: reqwest::Client::new(),
        })
    }

    #[tokio::test]
    async fn test_oauth_flow_and_mcp_tools() {
        let state = setup_test_state().await;
        let keys = state.config.get_mcp_keys();

        // 1. Test PKCE S256 challenge & code generation (also test fallback to configured redirect_url)
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let digest = digest::digest(&digest::SHA256, verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(digest.as_ref());

        let auth_resp = oauth_authorize(
            State(state.clone()),
            OriginalUri("/mcp/oauth/authorize".parse().unwrap()),
            HeaderMap::new(),
            Query(AuthorizeParams {
                response_type: Some("code".to_string()),
                client_id: Some("dudu".to_string()),
                redirect_uri: None,
                state: Some("xyz123".to_string()),
                code_challenge: Some(challenge.clone()),
                code_challenge_method: Some("S256".to_string()),
                scope: Some("mcp".to_string()),
            }),
            Bytes::new(),
        )
        .await;

        assert_eq!(auth_resp.status(), StatusCode::SEE_OTHER);
        let location = auth_resp
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.starts_with("https://gemini.google.com/callback?"));
        let redirected_url = Url::parse(location).unwrap();
        let query_map: HashMap<_, _> = redirected_url.query_pairs().into_owned().collect();
        assert_eq!(query_map.get("state").map(String::as_str), Some("xyz123"));
        let code = query_map.get("code").unwrap().clone();

        // Reject mismatched redirect_uri
        let bad_auth_resp = oauth_authorize(
            State(state.clone()),
            OriginalUri("/mcp/oauth/authorize".parse().unwrap()),
            HeaderMap::new(),
            Query(AuthorizeParams {
                response_type: Some("code".to_string()),
                client_id: Some("dudu".to_string()),
                redirect_uri: Some("https://evil.example.com/callback".to_string()),
                state: Some("xyz123".to_string()),
                code_challenge: Some(challenge),
                code_challenge_method: Some("S256".to_string()),
                scope: Some("mcp".to_string()),
            }),
            Bytes::new(),
        )
        .await;
        assert_eq!(bad_auth_resp.status(), StatusCode::BAD_REQUEST);

        // 2. Exchange authorization code for access_token
        let form_body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "authorization_code")
            .append_pair("code", &code)
            .append_pair("client_id", "dudu")
            .append_pair("client_secret", "someSuperSecretKey")
            .append_pair("redirect_uri", "https://gemini.google.com/callback")
            .append_pair("code_verifier", verifier)
            .finish();

        let token_resp = oauth_token(
            State(state.clone()),
            OriginalUri("/mcp/oauth/token".parse().unwrap()),
            HeaderMap::new(),
            Bytes::from(form_body),
        )
        .await;
        assert_eq!(token_resp.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(token_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        let access_token = token_json["access_token"].as_str().unwrap();
        assert_eq!(
            verify_signed_token(access_token, "access", &keys).as_deref(),
            Some("dudu")
        );

        // 3. Call MCP tools/list and tools/call with Bearer token
        let mut mcp_headers = HeaderMap::new();
        mcp_headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", access_token).parse().unwrap(),
        );

        let list_tools_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });
        let resp = handle_mcp_post(
            State(state.clone()),
            OriginalUri("/mcp".parse().unwrap()),
            mcp_headers.clone(),
            Bytes::from(serde_json::to_vec(&list_tools_req).unwrap()),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let resp_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp_json: Value = serde_json::from_slice(&resp_bytes).unwrap();
        let tools = resp_json["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);

        // 4. Create a reminder via MCP
        let create_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_reminder",
                "arguments": {
                    "reminder_message": "Buy milk"
                }
            }
        });
        let create_resp = handle_mcp_post(
            State(state.clone()),
            OriginalUri("/mcp".parse().unwrap()),
            mcp_headers.clone(),
            Bytes::from(serde_json::to_vec(&create_req).unwrap()),
        )
        .await;
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_bytes = axum::body::to_bytes(create_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_json: Value = serde_json::from_slice(&create_bytes).unwrap();
        assert_eq!(create_json["result"]["isError"], false);

        // 5. List reminders via MCP
        let list_req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "list_reminders",
                "arguments": {}
            }
        });
        let list_resp = handle_mcp_post(
            State(state.clone()),
            OriginalUri("/mcp".parse().unwrap()),
            mcp_headers,
            Bytes::from(serde_json::to_vec(&list_req).unwrap()),
        )
        .await;
        assert_eq!(list_resp.status(), StatusCode::OK);
        let list_bytes = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_json: Value = serde_json::from_slice(&list_bytes).unwrap();
        let text = list_json["result"]["content"][0]["text"].as_str().unwrap();
        let reminders: Vec<Reminder> = serde_json::from_str(text).unwrap();
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].reminder_message, "Buy milk");
        assert_eq!(reminders[0].username.as_deref(), Some("dudu"));
    }
}
