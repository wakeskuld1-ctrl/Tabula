use axum::extract::{Query, State};
use axum::Json;
use std::sync::Arc;

use crate::services::grid_service::{fetch_grid_data, GridDataRequest, GridDataResult};
use crate::AppState;

fn build_grid_data_response(result: GridDataResult) -> serde_json::Value {
    serde_json::json!({
        "status": "ok",
        "data": result.rows,
        "columns": result.columns,
        "column_types": result.column_types,
        "total_rows": result.total_rows,
        "metadata": result.metadata,
        "formula_columns": result.formula_columns
    })
}

/// 分页拉取表格数据，并可携带过滤与排序条件。
pub(crate) async fn get_grid_data(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GridDataRequest>,
) -> Json<serde_json::Value> {
    match fetch_grid_data(&state, &params).await {
        Ok(result) => Json(build_grid_data_response(result)),
        Err(message) => Json(serde_json::json!({ "status": "error", "message": message })),
    }
}

pub(crate) async fn list_tables(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // List from Metadata Manager to show rich metadata
    if let Ok(tables) = state.metadata_manager.list_tables() {
        let json_tables: Vec<serde_json::Value> = tables
            .iter()
            .map(|t| {
                serde_json::json!({
                    "table_name": t.table_name,
                    "file_path": t.file_path,
                    "source_type": t.source_type,
                    "sheet_name": t.sheet_name,
                    "schema_json": t.schema_json,
                    "indexes_json": t.indexes_json
                })
            })
            .collect();
        Json(serde_json::json!({ "status": "ok", "tables": json_tables }))
    } else {
        Json(serde_json::json!({ "status": "error", "message": "Failed to list tables" }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::grid_service::FormulaColumnMeta;

    #[tokio::test]
    async fn test_grid_data_includes_formula_columns() {
        let result = GridDataResult {
            rows: vec![],
            columns: vec!["col_a".to_string()],
            column_types: vec!["utf8".to_string()],
            total_rows: 0,
            metadata: None,
            formula_columns: Some(vec![FormulaColumnMeta {
                index: 0,
                name: "col_a".to_string(),
                raw_expression: "A+B".to_string(),
                sql_expression: "\"col_a\" + \"col_b\"".to_string(),
            }]),
        };

        let payload = build_grid_data_response(result);
        let has_formula_columns = payload
            .get("formula_columns")
            .and_then(|v| v.as_array())
            .is_some();

        assert!(has_formula_columns);
    }
}
