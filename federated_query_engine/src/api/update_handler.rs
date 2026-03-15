use axum::extract::{Json, State};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateCellRequest {
    pub table_name: String,
    pub session_id: Option<String>,
    #[serde(alias = "row")]
    pub row_idx: usize, // Row Index
    #[serde(alias = "col_name")]
    pub col: String, // Column Name
    #[serde(alias = "new_value")]
    pub val: String, // New Value
}

#[derive(Deserialize)]
pub struct BatchUpdateRequest {
    pub table_name: String,
    pub session_id: Option<String>,
    pub updates: Vec<CellUpdateItem>,
}

#[derive(Deserialize)]
pub struct CellUpdateItem {
    #[serde(alias = "row")]
    pub row_idx: usize,
    #[serde(alias = "col_name")]
    pub col: String,
    #[serde(alias = "new_value")]
    pub val: String,
}

#[derive(Deserialize)]
pub struct UpdateStyleRequest {
    pub table_name: String,
    pub row: u32,
    pub col: u32,
    pub style: crate::session_manager::CellStyle,
}

#[derive(Deserialize)]
pub struct UpdateStyleRangeRequest {
    pub table_name: String,
    pub session_id: Option<String>,
    pub range: crate::session_manager::MergeRange,
    pub style: crate::session_manager::CellStyle,
}

#[derive(Deserialize)]
pub struct UpdateMergeRequest {
    pub table_name: String,
    pub range: crate::session_manager::MergeRange,
}

pub async fn update_cell(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateCellRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .update_cell(
            &payload.table_name,
            payload.session_id.as_deref(),
            payload.row_idx,
            &payload.col,
            &payload.val,
        )
        .await
    {
        Ok((uri, sid)) => Json(serde_json::json!({
            "status": "ok",
            "message": "Cell updated",
            "lance_uri": uri,
            "session_id": sid
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e.message(),
            "code": e.code(),
            "details": e.details()
        })),
    }
}

pub async fn batch_update_cells(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchUpdateRequest>,
) -> Json<serde_json::Value> {
    // For now, loop over updates. Ideally SessionManager should support batch update.
    // TODO: Add batch_update_cells to SessionManager
    let mut last_sid = payload.session_id.clone();
    let mut success_count = 0;

    for item in payload.updates {
        match state
            .session_manager
            .update_cell(
                &payload.table_name,
                last_sid.as_deref(),
                item.row_idx,
                &item.col,
                &item.val,
            )
            .await
        {
            Ok((_, sid)) => {
                last_sid = Some(sid);
                success_count += 1;
            }
            Err(e) => {
                // **[2026-03-15]** 变更原因：前端需要 error 字段展示批量失败原因。
                // **[2026-03-15]** 变更目的：复用同一错误信息填充 message/error。
                let error_message =
                    format!("Batch update failed at row {}: {}", item.row_idx, e.message());
                return Json(serde_json::json!({
                    "status": "error",
                    "message": error_message,
                    "error": error_message,
                    "success_count": success_count
                }));
            }
        }
    }

    Json(serde_json::json!({
        "status": "ok",
        "message": format!("Batch updated {} cells", success_count),
        "session_id": last_sid,
        // **[2026-03-15]** 变更原因：前端响应结构包含可选 error 字段。
        // **[2026-03-15]** 变更目的：成功路径显式返回空 error 便于统一解析。
        "error": serde_json::Value::Null
    }))
}

pub async fn update_style(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateStyleRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .update_style(&payload.table_name, payload.row, payload.col, payload.style)
        .await
    {
        Ok(msg) => Json(serde_json::json!({
            "status": "ok",
            "message": msg
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}

// **[2026-03-15]** 变更原因：补充样式范围更新 API。
// **[2026-03-15]** 变更目的：支持可选 session_id 的样式范围更新。
pub async fn update_style_range(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateStyleRangeRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .update_style_range_with_session(
            &payload.table_name,
            payload.session_id.as_deref(),
            payload.range,
            payload.style,
        )
        .await
    {
        Ok(msg) => Json(serde_json::json!({
            "status": "ok",
            "message": msg,
            // **[2026-03-15]** 变更原因：前端期望响应含可选 error 字段。
            // **[2026-03-15]** 变更目的：成功路径显式返回空 error 便于统一解析。
            "error": serde_json::Value::Null
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e,
            // **[2026-03-15]** 变更原因：错误路径需要同时返回 message/error。
            // **[2026-03-15]** 变更目的：保证前端统一字段读取与兼容旧逻辑。
            "error": e
        })),
    }
}

pub async fn update_merge(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateMergeRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .update_merge(&payload.table_name, payload.range)
        .await
    {
        Ok(msg) => Json(serde_json::json!({
            "status": "ok",
            "message": msg
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}
