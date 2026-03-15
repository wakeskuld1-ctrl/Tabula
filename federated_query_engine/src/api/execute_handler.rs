use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::services::execute_service::{execute_sql as execute_sql_service, ExecuteResponse};
use crate::AppState;

#[derive(Deserialize)]
pub(crate) struct ExecuteRequest {
    sql: String,
}

pub(crate) async fn execute_sql(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    Json(execute_sql_service(&state, payload.sql).await)
}
