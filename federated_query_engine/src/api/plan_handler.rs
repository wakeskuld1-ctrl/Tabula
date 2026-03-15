use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::services::execute_service::{get_plan, PlanResponse};
use crate::AppState;

#[derive(Deserialize)]
pub(crate) struct PlanRequest {
    sql: String,
    #[allow(dead_code)]
    #[serde(default)]
    dry_run: bool,
    #[allow(dead_code)]
    #[serde(default)]
    runtime_filter: bool,
}

pub(crate) async fn plan(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PlanRequest>,
) -> Json<PlanResponse> {
    Json(get_plan(&state, payload.sql).await)
}
