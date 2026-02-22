//! REST API server for Rein.
//!
//! Provides HTTP endpoints for triggering workflows, listing agents,
//! querying the audit log, and health checks.

mod routes;

#[cfg(test)]
mod tests;

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::ast::ReinFile;
use crate::runtime::audit::AuditLog;

/// Shared application state for the API server.
pub struct AppState {
    /// The parsed .rein file(s).
    pub rein_file: ReinFile,
    /// The audit log.
    pub audit_log: Option<AuditLog>,
}

/// Build the Axum router with all API routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    routes::build(state).layer(CorsLayer::permissive())
}

/// Start the API server on the given address.
///
/// # Errors
/// Returns an error if the server fails to bind or serve.
pub async fn serve(state: Arc<AppState>, addr: SocketAddr) -> Result<(), std::io::Error> {
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
