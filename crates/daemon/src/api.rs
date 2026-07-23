//! HTTP API for agents and MCP.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use codepulse_indexer::StructuralSearchRequest;
use codepulse_protocol::{CreateProbeWindowRequest, ProbeAck, RuntimeStatBatch, SymbolId};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/batches", post(post_batch))
        .route("/v1/probe-commands", get(probe_commands))
        .route("/v1/probe-windows", post(create_probe_window))
        .route("/v1/probe-windows/{window_id}/ack", post(ack_probe))
        .route("/v1/reindex", post(reindex))
        .route("/v1/query/hot-paths", get(hot_paths))
        .route("/v1/query/function-summary", get(function_summary))
        .route("/v1/query/callers", get(callers))
        .route("/v1/query/callees", get(callees))
        .route("/v1/query/static-summary", get(static_summary))
        .route("/v1/query/compare", get(compare))
        .route("/v1/query/uncovered", get(uncovered))
        .route("/v1/query/structural-search", post(structural_search))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "service": "codepulse" }))
}

async fn post_batch(
    State(state): State<AppState>,
    Json(batch): Json<RuntimeStatBatch>,
) -> impl IntoResponse {
    match state.ingest.ingest(&batch) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(codepulse_ingest::IngestError::ProtocolVersion(_)) => {
            (StatusCode::CONFLICT, Json(json!({ "error": "protocol mismatch" }))).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct SessionQuery {
    session_id: Option<String>,
}

async fn probe_commands(
    State(state): State<AppState>,
    Query(q): Query<SessionQuery>,
) -> impl IntoResponse {
    let session = q.session_id.unwrap_or_default();
    match state.controller.poll_commands(&session) {
        Ok(commands) => Json(json!({ "commands": commands })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn create_probe_window(
    State(state): State<AppState>,
    Json(req): Json<CreateProbeWindowRequest>,
) -> impl IntoResponse {
    match state.controller.create_window(
        req.session_id.as_deref(),
        req.targets,
        req.duration_s,
    ) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn ack_probe(
    State(state): State<AppState>,
    Path(window_id): Path<String>,
    Json(ack): Json<ProbeAck>,
) -> impl IntoResponse {
    match state.controller.ack(&window_id, &ack.status) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn reindex(State(state): State<AppState>) -> impl IntoResponse {
    let indexer = state.indexer.lock().unwrap();
    match indexer.index_root() {
        Ok(n) => Json(json!({ "symbols": n })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct HotPathsQuery {
    session_id: Option<String>,
    limit: Option<usize>,
    metric: Option<String>,
}

async fn hot_paths(
    State(state): State<AppState>,
    Query(q): Query<HotPathsQuery>,
) -> impl IntoResponse {
    let metric = q.metric.as_deref().unwrap_or("invocations");
    let limit = q.limit.unwrap_or(20);
    match state
        .store
        .hot_paths(q.session_id.as_deref(), limit, metric)
    {
        Ok(rows) => Json(json!({ "paths": rows })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct SymbolQuery {
    language: String,
    path: String,
    qualname: String,
    session_id: Option<String>,
    limit: Option<usize>,
}

fn symbol_from(q: &SymbolQuery) -> SymbolId {
    SymbolId::new(&q.language, &q.path, &q.qualname)
}

async fn function_summary(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> impl IntoResponse {
    match state
        .store
        .function_runtime_summary(&symbol_from(&q), q.session_id.as_deref())
    {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no runtime data" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn callers(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> impl IntoResponse {
    match state.store.actual_callers(
        &symbol_from(&q),
        q.session_id.as_deref(),
        q.limit.unwrap_or(20),
    ) {
        Ok(rows) => Json(json!({ "callers": rows })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn callees(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> impl IntoResponse {
    match state.store.actual_callees(
        &symbol_from(&q),
        q.session_id.as_deref(),
        q.limit.unwrap_or(20),
    ) {
        Ok(rows) => Json(json!({ "callees": rows })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn static_summary(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> impl IntoResponse {
    match state.store.static_summary(&symbol_from(&q)) {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "symbol not indexed" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn compare(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> impl IntoResponse {
    match state
        .store
        .compare_static_vs_runtime(&symbol_from(&q), q.session_id.as_deref())
    {
        Ok(s) => Json(s).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct UncoveredQuery {
    min_complexity: Option<i64>,
    limit: Option<usize>,
    session_id: Option<String>,
}

async fn uncovered(
    State(state): State<AppState>,
    Query(q): Query<UncoveredQuery>,
) -> impl IntoResponse {
    match state.store.uncovered_hot_symbols(
        q.min_complexity.unwrap_or(10),
        q.limit.unwrap_or(20),
        q.session_id.as_deref(),
    ) {
        Ok(rows) => Json(json!({ "symbols": rows })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct StructuralSearchBody {
    language: String,
    pattern: String,
    path_prefix: Option<String>,
    limit: Option<u32>,
}

async fn structural_search(
    State(state): State<AppState>,
    Json(body): Json<StructuralSearchBody>,
) -> impl IntoResponse {
    let indexer = state.indexer.lock().unwrap();
    let req = StructuralSearchRequest {
        language: body.language,
        pattern: body.pattern,
        path_prefix: body.path_prefix,
        limit: body.limit.unwrap_or(50),
    };
    match indexer.structural_search(&req) {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
