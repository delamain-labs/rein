use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;

/// Build all API routes.
pub fn build(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/agents", get(list_agents))
        .route("/api/v1/workflows", get(list_workflows))
        .route("/api/v1/types", get(list_types))
        .route("/api/v1/audit", get(query_audit))
        .with_state(state)
}

// ── Health ──────────────────────────────────────────────────────────────

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

// ── Agents ──────────────────────────────────────────────────────────────

async fn list_agents(State(state): State<Arc<AppState>>) -> Json<AgentsResponse> {
    let agents: Vec<AgentSummary> = state
        .rein_file
        .agents
        .iter()
        .map(|a| AgentSummary {
            name: a.name.clone(),
            model: a.model.as_ref().map_or_else(String::new, |m| match m {
                crate::ast::ValueExpr::Literal(s) => s.clone(),
                crate::ast::ValueExpr::EnvRef { var_name, .. } => format!("env({var_name})"),
            }),
            capabilities: a
                .can
                .iter()
                .map(|c| format!("{}.{}", c.namespace, c.action))
                .collect(),
        })
        .collect();
    Json(AgentsResponse { agents })
}

#[derive(Serialize)]
struct AgentsResponse {
    agents: Vec<AgentSummary>,
}

#[derive(Serialize)]
struct AgentSummary {
    name: String,
    model: String,
    capabilities: Vec<String>,
}

// ── Workflows ───────────────────────────────────────────────────────────

async fn list_workflows(State(state): State<Arc<AppState>>) -> Json<WorkflowsResponse> {
    let workflows: Vec<WorkflowSummary> = state
        .rein_file
        .workflows
        .iter()
        .map(|w| WorkflowSummary {
            name: w.name.clone(),
            trigger: w.trigger.clone(),
            stages: w.stages.iter().map(|s| s.name.clone()).collect(),
            steps: w.steps.iter().map(|s| s.name.clone()).collect(),
        })
        .collect();
    Json(WorkflowsResponse { workflows })
}

#[derive(Serialize)]
struct WorkflowsResponse {
    workflows: Vec<WorkflowSummary>,
}

#[derive(Serialize)]
struct WorkflowSummary {
    name: String,
    trigger: String,
    stages: Vec<String>,
    steps: Vec<String>,
}

// ── Types ───────────────────────────────────────────────────────────────

async fn list_types(State(state): State<Arc<AppState>>) -> Json<TypesResponse> {
    let types: Vec<TypeSummary> = state
        .rein_file
        .types
        .iter()
        .map(|t| TypeSummary {
            name: t.name.clone(),
            fields: t.fields.iter().map(|f| f.name.clone()).collect(),
        })
        .collect();
    Json(TypesResponse { types })
}

#[derive(Serialize)]
struct TypesResponse {
    types: Vec<TypeSummary>,
}

#[derive(Serialize)]
struct TypeSummary {
    name: String,
    fields: Vec<String>,
}

// ── Audit ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AuditQuery {
    workflow: Option<String>,
    limit: Option<usize>,
}

async fn query_audit(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<AuditQuery>,
) -> Result<Json<AuditResponse>, StatusCode> {
    let Some(ref audit_log) = state.audit_log else {
        return Ok(Json(AuditResponse { entries: vec![] }));
    };

    let entries = if let Some(ref wf) = query.workflow {
        audit_log.query_by_workflow(wf)
    } else {
        audit_log.read_all()
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let limit = query.limit.unwrap_or(100);
    let entries: Vec<_> = entries.into_iter().take(limit).collect();

    Ok(Json(AuditResponse { entries }))
}

#[derive(Serialize)]
struct AuditResponse {
    entries: Vec<crate::runtime::audit::AuditEntry>,
}
