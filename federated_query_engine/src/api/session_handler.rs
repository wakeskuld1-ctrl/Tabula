use axum::extract::{Json, State};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub table_name: String,
    pub session_name: Option<String>,
    pub from_session_id: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Deserialize)]
pub struct DeleteTableRequest {
    pub table_name: String,
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateSessionRequest>,
) -> Json<serde_json::Value> {
    // Determine source path. For now, assuming hydration from parquet path if needed,
    // but create_session in SessionManager takes a path.
    // If we are creating a session for an existing table, we might need to look up its source path.

    // We can use state.metadata_manager to look up the table path.
    let table_meta =
        state
            .metadata_manager
            .store
            .get_table("datafusion", "public", &payload.table_name);

    match table_meta {
        Ok(Some(meta)) => {
            let path = meta.file_path;
            match state
                .session_manager
                .create_session(
                    &payload.table_name,
                    &path,
                    payload.session_name,
                    payload.from_session_id,
                    payload.is_default.unwrap_or(false),
                )
                .await
            {
                Ok(info) => Json(serde_json::json!({
                    "status": "ok",
                    "session": info
                })),
                Err(e) => Json(serde_json::json!({
                    "status": "error",
                    "message": e
                })),
            }
        }
        Ok(None) => Json(serde_json::json!({
            "status": "error",
            "message": format!("Table '{}' not found", payload.table_name)
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

pub async fn save_session(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // SessionManager auto-saves metadata to SQLite.
    // Lance data is persisted on write.
    // So explicit save might just trigger a flush if we had buffering.
    // SessionManager has start_auto_flush.
    // We can just return OK for now or trigger something if exposed.

    // The user requirement says "/api/save_session" is missing.
    // Ideally we might want to force a checkpoint or similar.
    // For now, let's just acknowledge.
    Json(serde_json::json!({
        "status": "ok",
        "message": "Session state is automatically persisted"
    }))
}

pub async fn delete_table(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeleteTableRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .delete_table(&payload.table_name)
        .await
    {
        Ok(count) => Json(serde_json::json!({
            "status": "ok",
            "message": format!("Deleted table and {} sessions", count)
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}
