use axum::extract::{Multipart, Query, State};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::services::upload_service::handle_upload;
use crate::AppState;

#[derive(Deserialize)]
pub(crate) struct UploadParams {
    #[serde(default)]
    header_rows: Option<usize>,
    #[serde(default)]
    header_mode: Option<String>,
}

/// 接收上传文件并注册为数据源。
///
/// # Arguments
/// - `state`: 全局应用状态。
/// - `params`: 上传参数，包含表头配置。
/// - `multipart`: 文件内容流。
///
/// # Returns
/// - 上传与注册结果。
pub(crate) async fn upload_file(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UploadParams>,
    multipart: Multipart,
) -> Json<serde_json::Value> {
    let header_rows = params.header_rows.unwrap_or(0);
    let header_mode = params
        .header_mode
        .clone()
        .unwrap_or_else(|| "none".to_string());

    Json(handle_upload(&state, header_rows, header_mode, multipart).await)
}
