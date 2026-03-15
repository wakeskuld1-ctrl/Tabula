use axum::extract::{Json, State};
use axum::Json as AxumJson;
use serde::Deserialize;
use std::sync::Arc;

use arrow::datatypes::{DataType, TimeUnit};

use crate::AppState;

#[derive(Deserialize)]
pub struct EnsureColumnsRequest {
    pub table_name: String,
    // **[2026-03-15]** Reason: frontend may send null for empty session_id.
    // **[2026-03-15]** Purpose: accept optional session_id and normalize server-side.
    pub session_id: Option<String>,
    pub columns: Vec<EnsureColumnItem>,
}

#[derive(Deserialize)]
pub struct EnsureColumnItem {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
}

// **[2026-03-15]** 变更原因：前端传入字符串类型需要映射到 Arrow DataType。
// **[2026-03-15]** 变更目的：兼容常见类型并在无法识别时给出明确错误。
fn parse_data_type(raw: &str) -> Result<DataType, String> {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return Err("data_type is empty".to_string());
    }

    // **[2026-03-15]** 变更原因：支持 decimal 类型 precision/scale。
    // **[2026-03-15]** 变更目的：允许 pivot 输出定义 decimal 列。
    for (prefix, is_256) in [
        ("decimal128(", false),
        ("decimal256(", true),
        ("decimal(", false),
    ] {
        if let Some(inner) = normalized.strip_prefix(prefix) {
            let inner = inner.trim_end_matches(')').trim();
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            if parts.len() != 2 {
                return Err(format!("invalid decimal format '{}'", raw));
            }
            let precision: u8 = parts[0]
                .parse()
                .map_err(|_| format!("invalid decimal precision '{}'", parts[0]))?;
            let scale: i8 = parts[1]
                .parse()
                .map_err(|_| format!("invalid decimal scale '{}'", parts[1]))?;
            return Ok(if is_256 {
                DataType::Decimal256(precision, scale)
            } else {
                DataType::Decimal128(precision, scale)
            });
        }
    }

    // **[2026-03-15]** 变更原因：支持 fixed_size_binary 格式。
    // **[2026-03-15]** 变更目的：允许显式指定二进制列宽度。
    if let Some(inner) = normalized.strip_prefix("fixed_size_binary(") {
        let inner = inner.trim_end_matches(')').trim();
        let size: i32 = inner
            .parse()
            .map_err(|_| format!("invalid fixed_size_binary size '{}'", inner))?;
        return Ok(DataType::FixedSizeBinary(size));
    }

    let dt = match normalized.as_str() {
        "null" => DataType::Null,
        "utf8" | "string" | "text" => DataType::Utf8,
        "largeutf8" | "large_string" | "largetext" => DataType::LargeUtf8,
        "binary" => DataType::Binary,
        "largebinary" | "large_binary" => DataType::LargeBinary,
        "bool" | "boolean" => DataType::Boolean,
        "int8" | "i8" => DataType::Int8,
        "int16" | "i16" => DataType::Int16,
        "int32" | "i32" => DataType::Int32,
        "int64" | "i64" => DataType::Int64,
        "uint8" | "u8" => DataType::UInt8,
        "uint16" | "u16" => DataType::UInt16,
        "uint32" | "u32" => DataType::UInt32,
        "uint64" | "u64" => DataType::UInt64,
        "float16" | "f16" => DataType::Float16,
        "float32" | "f32" => DataType::Float32,
        "float64" | "f64" | "double" => DataType::Float64,
        "date32" | "date" => DataType::Date32,
        "date64" => DataType::Date64,
        "timestamp" | "timestamp_us" | "timestamp_micros" => {
            DataType::Timestamp(TimeUnit::Microsecond, None)
        }
        "timestamp_ms" | "timestamp_millis" => DataType::Timestamp(TimeUnit::Millisecond, None),
        "timestamp_ns" | "timestamp_nanos" => DataType::Timestamp(TimeUnit::Nanosecond, None),
        "timestamp_s" | "timestamp_sec" | "timestamp_second" => {
            DataType::Timestamp(TimeUnit::Second, None)
        }
        _ => {
            return Err(format!(
                "unsupported data_type '{}', expected arrow-like type name",
                raw
            ))
        }
    };

    Ok(dt)
}

// **[2026-03-15]** 变更原因：补齐 ensure_columns HTTP 入口。
// **[2026-03-15]** 变更目的：支持 session 级扩列以便 batch_update_cells 写入。
pub async fn ensure_columns(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EnsureColumnsRequest>,
) -> AxumJson<serde_json::Value> {
    if payload.table_name.trim().is_empty() {
        return AxumJson(serde_json::json!({
            "status": "error",
            "message": "table_name required",
            // **[2026-03-15]** 变更原因：前端期望错误响应包含 error 字段。
            // **[2026-03-15]** 变更目的：保持 message 与 error 同步便于展示。
            "error": "table_name required"
        }));
    }

    // **[2026-03-15]** Reason: normalize null/empty session_id to None.
    // **[2026-03-15]** Purpose: align with frontend rule (null -> active session).
    let normalized_session_id = match payload.session_id.as_deref() {
        Some("null") | Some("") => None,
        Some(value) => Some(value),
        None => None,
    };

    let mut parsed_columns = Vec::with_capacity(payload.columns.len());
    for item in payload.columns {
        let dt = match parse_data_type(&item.data_type) {
            Ok(v) => v,
            Err(e) => {
                return AxumJson(serde_json::json!({
                    "status": "error",
                    "message": e,
                    // **[2026-03-15]** 变更原因：字段解析失败需要显式 error 字段。
                    // **[2026-03-15]** 变更目的：与前端可选 error 字段对齐。
                    "error": e
                }))
            }
        };
        parsed_columns.push((item.name, dt));
    }

    match state
        .session_manager
        .ensure_columns(&payload.table_name, normalized_session_id, parsed_columns)
        .await
    {
        Ok((effective_session_id, columns)) => AxumJson(serde_json::json!({
            "status": "ok",
            "session_id": effective_session_id,
            "columns": columns,
            // **[2026-03-15]** 变更原因：统一成功响应的 message/error 结构。
            // **[2026-03-15]** 变更目的：便于前端复用 toast/提示逻辑。
            "message": "columns ensured",
            "error": serde_json::Value::Null
        })),
        Err(e) => AxumJson(serde_json::json!({
            "status": "error",
            "message": e.message(),
            "code": e.code(),
            "details": e.details(),
            // **[2026-03-15]** 变更原因：会话扩列失败需带 error 字段。
            // **[2026-03-15]** 变更目的：前端统一错误解析逻辑。
            "error": e.message()
        })),
    }
}
