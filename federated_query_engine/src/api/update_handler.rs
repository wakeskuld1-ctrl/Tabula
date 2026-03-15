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
                return Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Batch update failed at row {}: {}", item.row_idx, e.message()),
                    "success_count": success_count
                }));
            }
        }
    }

    Json(serde_json::json!({
        "status": "ok",
        "message": format!("Batch updated {} cells", success_count),
        "session_id": last_sid
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
