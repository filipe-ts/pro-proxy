use crate::auth::LoginRateLimiter;
use crate::db::Db;
use crate::ui::{render_login_page, render_standby_page};
use axum::{
    body::Body,
    extract::{ConnectInfo, Query, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use regex::Regex;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{debug, info, warn};

pub const SESSION_COOKIE_NAME: &str = "pm_session";
pub const SESSION_TTL_SECS: u64 = 86400 * 7; // 7 days

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub pro_url: String, // e.g. "http://127.0.0.1:1025"
    pub login_limiter: LoginRateLimiter,
    pub client: reqwest::Client,
    pub trigger_regex: Regex,
    pub clear_regex: Regex,
}

impl AppState {
    pub fn new(db: Db, pro_port: u16) -> Self {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            db,
            pro_url: format!("http://127.0.0.1:{pro_port}"),
            login_limiter: LoginRateLimiter::new(5, Duration::from_secs(60)),
            client,
            trigger_regex: Regex::new(r"^/v1/message/[0-9a-zA-Z-]+/trigger$").unwrap(),
            clear_regex: Regex::new(r"^/v1/message/[0-9a-zA-Z-]+/clear$").unwrap(),
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handle_root))
        .route("/login", get(show_login).post(handle_login))
        .route("/logout", get(handle_logout).post(handle_logout))
        .route("/healthz", get(healthz))
        .fallback(handle_proxy_fallback)
        .with_state(state)
}

async fn handle_root(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if is_authenticated(&state, &headers) {
        Redirect::temporary("/html/pages/messages").into_response()
    } else {
        Redirect::temporary("/login").into_response()
    }
}

#[derive(serde::Deserialize)]
struct LoginParams {
    error: Option<String>,
}

async fn show_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<LoginParams>,
) -> impl IntoResponse {
    if is_authenticated(&state, &headers) {
        return Redirect::temporary("/html/pages/messages").into_response();
    }
    Html(render_login_page(params.error.as_deref())).into_response()
}

#[derive(serde::Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn handle_login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    form: axum::extract::Form<LoginForm>,
) -> impl IntoResponse {
    let ip = addr.ip();
    if !state.login_limiter.check_and_record(ip) {
        warn!("Rate limit exceeded for IP: {ip}");
        return Redirect::to("/login?error=rate_limited").into_response();
    }

    let username = form.username.trim();
    let password = &form.password;

    match state.db.get_user_password_hash(username) {
        Ok(Some(hash)) => {
            if crate::auth::verify_password(password, &hash) {
                let token = crate::auth::generate_session_token();
                if let Err(e) = state.db.create_session(&token, username, SESSION_TTL_SECS) {
                    warn!("Failed to create session: {e}");
                    return Redirect::to("/login?error=invalid").into_response();
                }

                info!("User '{}' authenticated successfully from {}", username, ip);
                let cookie = Cookie::build((SESSION_COOKIE_NAME, token))
                    .path("/")
                    .http_only(true)
                    .secure(true) // will work over HTTPS/Cloudflare tunnel
                    .same_site(SameSite::Lax)
                    .max_age(time::Duration::seconds(SESSION_TTL_SECS as i64))
                    .build();

                let mut response = Redirect::to("/html/pages/messages").into_response();
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    HeaderValue::from_str(&cookie.to_string()).unwrap(),
                );
                return response;
            } else {
                warn!("Invalid password attempt for user '{}' from {}", username, ip);
            }
        }
        Ok(None) => {
            warn!("Login attempt for non-existent user '{}' from {}", username, ip);
        }
        Err(e) => {
            warn!("Database error during login: {e}");
        }
    }

    Redirect::to("/login?error=invalid").into_response()
}

async fn handle_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(token) = extract_session_token(&headers) {
        let _ = state.db.delete_session(&token);
    }

    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build();

    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );
    response
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub fn is_whitelisted_route(method: &Method, path: &str, state: &AppState) -> bool {
    if (method == Method::GET || method == Method::HEAD) && path == "/html/pages/messages" {
        return true;
    }
    if (method == Method::GET || method == Method::HEAD) && path.starts_with("/html/assets/") {
        return true;
    }
    if (method == Method::GET || method == Method::HEAD) && path == "/v1/messages" {
        return true;
    }
    if method == Method::POST && state.trigger_regex.is_match(path) {
        return true;
    }
    if (method == Method::GET || method == Method::HEAD) && state.clear_regex.is_match(path) {
        return true;
    }
    false
}

fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for piece in cookie_header.split(';') {
        let mut parts = piece.trim().splitn(2, '=');
        if let (Some(name), Some(val)) = (parts.next(), parts.next()) {
            if name.trim() == SESSION_COOKIE_NAME {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

fn is_authenticated(state: &AppState, headers: &HeaderMap) -> bool {
    if let Some(token) = extract_session_token(headers) {
        match state.db.validate_session(&token) {
            Ok(Some(_user)) => true,
            _ => false,
        }
    } else {
        false
    }
}

fn is_api_or_xhr_request(path: &str, headers: &HeaderMap) -> bool {
    if path.starts_with("/v1/") {
        return true;
    }
    if let Some(accept) = headers.get(header::ACCEPT).and_then(|h| h.to_str().ok()) {
        if accept.contains("application/json") {
            return true;
        }
    }
    if let Some(xhr) = headers.get("x-requested-with").and_then(|h| h.to_str().ok()) {
        if xhr.eq_ignore_ascii_case("xmlhttprequest") {
            return true;
        }
    }
    false
}

async fn handle_proxy_fallback(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let headers = req.headers().clone();

    // 1. Check Route Whitelist
    if !is_whitelisted_route(&method, &path, &state) {
        debug!("Access denied by whitelist: {} {}", method, path);
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from("403 Forbidden: Route not permitted"))
            .unwrap();
    }

    // 2. Check Authentication
    if !is_authenticated(&state, &headers) {
        if is_api_or_xhr_request(&path, &headers) {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"error":"Unauthorized"}"#))
                .unwrap();
        } else {
            return Redirect::temporary("/login").into_response();
        }
    }

    // 3. ProPresenter Reachability & Reverse Proxy
    proxy_to_propresenter(state, req).await
}

async fn proxy_to_propresenter(
    state: AppState,
    req: Request<Body>,
) -> Response<Body> {
    let uri = req.uri();
    let path = uri.path().to_string();
    let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or(uri.path()).to_string();
    let is_api_or_xhr = is_api_or_xhr_request(&path, req.headers());
    let target_url = format!("{}{}", state.pro_url, path_and_query);

    let method = req.method().clone();
    let mut client_req = state.client.request(method.clone(), &target_url);

    // Forward headers excluding hop-by-hop
    for (name, value) in req.headers() {
        if is_hop_by_hop(name) {
            continue;
        }
        client_req = client_req.header(name.clone(), value.clone());
    }

    // Forward body for POST/PUT/PATCH
    if method == Method::POST || method == Method::PUT || method == Method::PATCH {
        let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to read request body: {e}");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Failed to read request body"))
                    .unwrap();
            }
        };
        client_req = client_req.body(body_bytes);
    }

    // Send request to ProPresenter
    match client_req.send().await {
        Ok(pro_resp) => {
            let status = pro_resp.status();
            let mut builder = Response::builder().status(status.as_u16());

            for (k, v) in pro_resp.headers() {
                if !is_hop_by_hop(k) {
                    builder = builder.header(k.clone(), v.clone());
                }
            }

            let stream = pro_resp.bytes_stream();
            let body = Body::from_stream(stream);
            builder.body(body).unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal proxy error"))
                    .unwrap()
            })
        }
        Err(err) => {
            warn!("Failed to connect to ProPresenter at {}: {}", target_url, err);
            // If the user is loading the messages page, show the standby UI
            if path == "/html/pages/messages" || !is_api_or_xhr {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(render_standby_page()))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"error":"ProPresenter is currently offline"}"#))
                    .unwrap()
            }
        }
    }
}

fn is_hop_by_hop(header_name: &HeaderName) -> bool {
    let name = header_name.as_str();
    name.eq_ignore_ascii_case("connection")
        || name.eq_ignore_ascii_case("keep-alive")
        || name.eq_ignore_ascii_case("proxy-authenticate")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("te")
        || name.eq_ignore_ascii_case("trailer")
        || name.eq_ignore_ascii_case("transfer-encoding")
        || name.eq_ignore_ascii_case("upgrade")
        || name.eq_ignore_ascii_case("host")
}
