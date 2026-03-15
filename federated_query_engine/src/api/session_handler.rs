use axum::extract::{Json, Query, State};
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

// - **2026-03-14**: Query payload for sessions list endpoint.
// - **Reason**: Keep GET parameters explicit and typed.
// - **Purpose**: Validate table_name presence before hitting SessionManager.
#[derive(Deserialize)]
pub struct ListSessionsQuery {
    pub table_name: String,
}

// - **2026-03-14**: Request payload for session switch endpoint.
// - **Reason**: Switching needs both table_name and target session_id.
// - **Purpose**: Ensure JSON contract matches frontend expectations.
#[derive(Deserialize)]
pub struct SwitchSessionRequest {
    pub table_name: String,
    pub session_id: String,
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

// - **2026-03-14**: List sessions for a given table with active session id.
// - **Reason**: Frontend sheet tabs need the list + active selection in one call.
// - **Purpose**: Reduce round trips and keep UI state consistent.
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListSessionsQuery>,
) -> Json<serde_json::Value> {
    let sessions = state.session_manager.list_sessions(&query.table_name).await;
    let active_session_id = state
        .session_manager
        .get_active_session_id(&query.table_name)
        .await;

    Json(serde_json::json!({
        "status": "ok",
        "sessions": sessions,
        "active_session_id": active_session_id
    }))
}

// - **2026-03-14**: Switch the active session for a table.
// - **Reason**: Users must move between sandboxes explicitly.
// - **Purpose**: Centralize active session updates on the server.
pub async fn switch_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SwitchSessionRequest>,
) -> Json<serde_json::Value> {
    match state
        .session_manager
        .switch_session(&payload.table_name, &payload.session_id)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "status": "ok",
            "message": "Switched session"
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
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
