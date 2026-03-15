use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::services::register_service::{register_table, RegisterTableRequest};
use crate::AppState;

/// 通过显式请求注册外部数据源。
///
/// # Arguments
/// - `state`: 全局应用状态。
/// - `payload`: 注册请求。
///
/// # Returns
/// - 注册结果与错误信息。
pub(crate) async fn register_table_endpoint(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterTableRequest>,
) -> Json<serde_json::Value> {
    Json(register_table(&state, payload).await)
}
