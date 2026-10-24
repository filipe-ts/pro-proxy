use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use pro_proxy::auth;
use pro_proxy::db::Db;
use pro_proxy::proxy::{create_router, is_whitelisted_route, AppState, SESSION_COOKIE_NAME, SESSION_TTL_SECS};
use serde_json::json;
use std::net::SocketAddr;
use tower::ServiceExt;

fn setup_test_app() -> (axum::Router, Db, AppState) {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().join("test_pro_proxy.db");
    let db = Db::open(&db_path).expect("Failed to open test DB");

    // Add a test user
    let hash = auth::hash_password("testpass123").expect("Failed to hash password");
    db.set_user("admin", &hash).expect("Failed to set user");

    let state = AppState::new(db.clone(), 1025);
    let app = create_router(state.clone());

    (app, db, state)
}

#[tokio::test]
async fn test_unauthenticated_access_redirects_to_login() {
    let (app, _, _) = setup_test_app();

    // GET / should redirect to /login
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get(header::LOCATION).unwrap(),
        "/login"
    );

    // GET /html/pages/messages without cookie should redirect to /login
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/html/pages/messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get(header::LOCATION).unwrap(),
        "/login"
    );

    // API request without cookie should return 401 Unauthorized
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/messages")
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_non_whitelisted_routes_are_forbidden() {
    let (app, db, state) = setup_test_app();

    // Create valid session
    let token = auth::generate_session_token();
    db.create_session(&token, "admin", SESSION_TTL_SECS).unwrap();
    let cookie_header = format!("{SESSION_COOKIE_NAME}={token}");

    // Test whitelist function directly
    assert_eq!(SESSION_TTL_SECS, 4 * 3600);
    assert!(!is_whitelisted_route(&Method::GET, "/v1/control", &state));
    assert!(!is_whitelisted_route(&Method::GET, "/v1/library", &state));
    assert!(!is_whitelisted_route(&Method::GET, "/v1/looks", &state));
    assert!(!is_whitelisted_route(&Method::POST, "/v1/presentation/current", &state));
    assert!(!is_whitelisted_route(&Method::GET, "/v1/status/layers", &state));
    assert!(!is_whitelisted_route(&Method::GET, "/html/assets/../../v1/control", &state));
    assert!(!is_whitelisted_route(&Method::GET, "/html/assets/%2e%2e/%2e%2e/v1/control", &state));

    // Test via HTTP router
    let blocked_routes = [
        "/v1/control",
        "/v1/library",
        "/v1/looks",
        "/v1/presentation/slide_index",
        "/admin/secret",
        "/html/assets/../../v1/control",
        "/html/assets/%2e%2e/%2e%2e/v1/control",
        "/html/assets//evil",
        "/html/assets\\something",
    ];

    for route in blocked_routes {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(route)
                    .header(header::COOKIE, &cookie_header)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Route {} should be blocked with 403 Forbidden",
            route
        );
    }
}

#[tokio::test]
async fn test_login_and_logout_flow() {
    let (app, _, _) = setup_test_app();
    let addr: SocketAddr = "127.0.0.1:54321".parse().unwrap();

    // 1. Invalid login attempt
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/login")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("username=admin&password=wrongpassword"))
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(addr));

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/login?error=invalid");

    // 2. Valid login attempt
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/login")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("username=admin&password=testpass123"))
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(addr));

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/html/pages/messages");
    let set_cookie = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(set_cookie.contains(SESSION_COOKIE_NAME));

    // Extract session token
    let token = set_cookie.split(';').next().unwrap().split('=').nth(1).unwrap();

    // 3. Authenticated request using cookie
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/html/pages/messages")
                .header(header::COOKIE, format!("{SESSION_COOKIE_NAME}={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4. Logout
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/logout")
                .header(header::COOKIE, format!("{SESSION_COOKIE_NAME}={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/login");

    // 5. Verify token is invalidated
    let response = app
        .oneshot(
            Request::builder()
                .uri("/html/pages/messages")
                .header(header::COOKIE, format!("{SESSION_COOKIE_NAME}={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/login");
}

#[tokio::test]
async fn test_standby_page_when_propresenter_offline() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let db_path = temp_dir.path().join("test_pro_proxy.db");
    let db = Db::open(&db_path).expect("Failed to open test DB");

    // Port 10999 is definitely not listening
    let state = AppState::new(db.clone(), 10999);
    let app = create_router(state);

    let token = auth::generate_session_token();
    db.create_session(&token, "admin", SESSION_TTL_SECS).unwrap();
    let cookie_header = format!("{SESSION_COOKIE_NAME}={token}");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/html/pages/messages")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("Aguardando o ProPresenter 7"), "Should render standby page");
}

#[tokio::test]
async fn test_authenticated_propresenter_messages_flow() {
    let (app, db, _) = setup_test_app();

    // Authenticate
    let token = auth::generate_session_token();
    db.create_session(&token, "admin", SESSION_TTL_SECS).unwrap();
    let cookie_header = format!("{SESSION_COOKIE_NAME}={token}");

    // 1. Fetch /html/pages/messages through proxy
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/html/pages/messages")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("ProPresenter Messages"), "HTML should contain ProPresenter Messages title");

    // 2. Fetch /v1/messages through proxy
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/messages")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    if response.status() != StatusCode::OK {
        println!("⚠️ ProPresenter 7 is offline (status {}). Skipping live trigger test.", response.status());
        return;
    }

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let messages_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(j) => j,
        Err(_) => {
            println!("⚠️ ProPresenter returned non-JSON response. Skipping live trigger test.");
            return;
        }
    };
    let messages_arr = match messages_json.as_array() {
        Some(arr) => arr,
        None => {
            println!("⚠️ ProPresenter messages response is not an array. Skipping live trigger test.");
            return;
        }
    };

    // Find the template message with visible_on_network: true
    let active_msg = match messages_arr
        .iter()
        .find(|m| m.get("visible_on_network").and_then(|v| v.as_bool()).unwrap_or(false))
    {
        Some(m) => m,
        None => {
            println!("⚠️ No message with 'Allow Web Notifications' enabled found on ProPresenter. See README for setup instructions.");
            return;
        }
    };

    let uuid = active_msg["id"]["uuid"].as_str().unwrap();
    let msg_name = active_msg["id"]["name"].as_str().unwrap_or("Unknown");
    println!("Found network-visible message: '{}' (UUID: {})", msg_name, uuid);

    // 3. Trigger message dynamically populating all defined tokens
    let trigger_payload: Vec<serde_json::Value> = active_msg["tokens"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|t| {
            let name = t["name"].as_str().unwrap_or("Token");
            json!({
                "name": name,
                "text": { "text": format!("Teste {name}") }
            })
        })
        .collect();

    let trigger_uri = format!("/v1/message/{uuid}/trigger?requires_confirmation=true");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&trigger_uri)
                .header(header::COOKIE, &cookie_header)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&trigger_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "Triggering message should return 204 No Content"
    );

    // 4. Clear/hide the message after test run
    let clear_uri = format!("/v1/message/{uuid}/clear");
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&clear_uri)
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "Clearing message should return 204 No Content"
    );
}
