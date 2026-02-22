use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

use crate::ast::ReinFile;
use crate::parser::parse;

use super::{AppState, build_router};

fn test_state() -> Arc<AppState> {
    let rein_file = parse(
        r#"
        agent triage { model: "gpt-4o" can [tools.classify] }
        agent writer { model: "claude-3" }
        workflow support {
            trigger: new_ticket
            step classify { agent: triage }
            step respond { agent: writer }
        }
    "#,
    )
    .unwrap();
    Arc::new(AppState {
        rein_file,
        audit_log: None,
    })
}

#[tokio::test]
async fn health_endpoint() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn list_agents_endpoint() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::get("/api/v1/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let agents = json["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0]["name"], "triage");
}

#[tokio::test]
async fn list_workflows_endpoint() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::get("/api/v1/workflows")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let workflows = json["workflows"].as_array().unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0]["name"], "support");
}

#[tokio::test]
async fn audit_empty() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::get("/api/v1/audit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn not_found() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::get("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
