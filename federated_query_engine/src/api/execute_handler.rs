// **[2026-03-15]** 变更原因：该 handler 目前未挂路由导致 dead_code 告警。
// **[2026-03-15]** 变更目的：保留兼容入口并消除编译 warnings。
// **[2026-03-15]** 变更说明：仅影响编译期告警，不改变运行行为。
// **[2026-03-15]** 变更备注：路由接入后请移除此 allow。
#![allow(dead_code)]

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::execute_service::{execute_sql as execute_sql_service, ExecuteResponse};
use crate::AppState;

#[derive(Deserialize)]
pub(crate) struct ExecuteRequest {
    sql: String,
}

// **[2026-03-15]** 变更原因：前端要求 /api/execute 统一 status 字段。
// **[2026-03-15]** 变更目的：在不破坏原有字段的前提下补齐响应格式。
// **[2026-03-15]** 变更说明：使用 flatten 保留原 ExecuteResponse 字段。
#[derive(Serialize)]
pub(crate) struct ExecuteResponseWithStatus {
    status: String,
    #[serde(flatten)]
    data: ExecuteResponse,
}

pub(crate) async fn execute_sql(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExecuteRequest>,
) -> Json<ExecuteResponseWithStatus> {
    // **[2026-03-15]** 变更原因：根据 error 字段派生 ok/error 状态。
    // **[2026-03-15]** 变更目的：让前端无需解析 error 即可判断成功与否。
    let response = execute_sql_service(&state, payload.sql).await;
    // **[2026-03-15]** 变更原因：保持错误与成功路径统一结构。
    // **[2026-03-15]** 变更目的：确保 status 字段恒定出现。
    let status = if response.error.is_some() { "error" } else { "ok" };
    Json(ExecuteResponseWithStatus {
        status: status.to_string(),
        data: response,
    })
}
