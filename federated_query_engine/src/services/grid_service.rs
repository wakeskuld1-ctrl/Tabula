use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::quote_ident;
use crate::session_manager::SheetMetadata;
use crate::AppState;

#[derive(Deserialize)]
pub(crate) struct GridDataRequest {
    pub(crate) session_id: Option<String>,
    pub(crate) table_name: String,
    pub(crate) page: usize,
    pub(crate) page_size: usize,
    #[serde(default)]
    pub(crate) filters: Option<String>,
    #[serde(default)]
    pub(crate) sort: Option<String>,
}

pub(crate) struct GridDataResult {
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) columns: Vec<String>,
    pub(crate) column_types: Vec<String>,
    pub(crate) total_rows: usize,
    pub(crate) metadata: Option<SheetMetadata>,
    pub(crate) formula_columns: Option<Vec<FormulaColumnMeta>>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormulaColumnMeta {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) raw_expression: String,
    pub(crate) sql_expression: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FormulaMarker {
    kind: String,
    raw: String,
    sql: String,
}

fn parse_formula_marker(raw: &str) -> Option<FormulaMarker> {
    let marker = serde_json::from_str::<FormulaMarker>(raw).ok()?;
    if marker.kind != "formula" || marker.sql.trim().is_empty() {
        return None;
    }
    Some(marker)
}

// **[2026-02-16]** 变更原因：需要识别 Excel 列标。
// **[2026-02-16]** 变更目的：把 A/B/AA 转换为列索引。
fn column_label_to_index(label: &str) -> Option<usize> {
    if label.is_empty() {
        return None;
    }
    let mut index = 0usize;
    for ch in label.chars() {
        if !ch.is_ascii_uppercase() {
            return None;
        }
        let value = (ch as u8 - b'A' + 1) as usize;
        index = index * 26 + value;
    }
    Some(index.saturating_sub(1))
}

// **[2026-02-16]** 变更原因：避免 Utf8 与数值混算报错。
// **[2026-02-16]** 变更目的：统一包裹 TRY_CAST + NULLIF。
// **[2026-02-17]** 变更原因：NULLIF 直接比较数值列与空字符串会触发 Arrow Cast error。
// **[2026-02-17]** 变更目的：先 CAST 到 VARCHAR，再做 NULLIF，避免类型冲突。
fn build_safe_cast_expr(column_name: &str) -> String {
    format!(
        "TRY_CAST(NULLIF(CAST({} AS VARCHAR), '') AS DOUBLE)",
        quote_ident(column_name)
    )
}

// **[2026-02-16]** 变更原因：需要重建公式列 SQL。
// **[2026-02-16]** 变更目的：为安全转换提供统一入口。
fn rebuild_formula_sql(raw: &str, column_names: &[String]) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut output = String::new();
    let mut token = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_uppercase() {
            token.push(ch);
            continue;
        }
        if ch.is_ascii_alphabetic() {
            return None;
        }
        if !token.is_empty() {
            let idx = column_label_to_index(&token)?;
            let column_name = column_names.get(idx)?;
            output.push_str(&build_safe_cast_expr(column_name));
            token.clear();
        }
        output.push(ch);
    }
    if !token.is_empty() {
        let idx = column_label_to_index(&token)?;
        let column_name = column_names.get(idx)?;
        output.push_str(&build_safe_cast_expr(column_name));
    }
    Some(output)
}

fn load_formula_markers(state: &Arc<AppState>, table_name: &str) -> Vec<Option<FormulaMarker>> {
    let meta = state
        .metadata_manager
        .store
        .get_table("datafusion", "public", table_name)
        .ok()
        .flatten();

    let raw = meta.and_then(|m| m.column_default_formulas_json);
    let list = raw
        .as_ref()
        .and_then(|payload| serde_json::from_str::<Vec<Option<String>>>(payload).ok())
        .unwrap_or_default();

    list.into_iter()
        .map(|entry: Option<String>| entry.and_then(|value| parse_formula_marker(&value)))
        .collect()
}

fn build_grid_select_sql(
    table_name: &str,
    where_sql: &str,
    order_clause: &str,
    limit: usize,
    offset: usize,
    column_names: &[String],
    computed_columns: &HashMap<String, String>,
) -> String {
    let select_list = if column_names.is_empty() {
        "*".to_string()
    } else {
        column_names
            .iter()
            .map(|name| {
                if let Some(expr) = computed_columns.get(name) {
                    format!("{} AS {}", expr, quote_ident(name))
                } else {
                    quote_ident(name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let mut parts = vec![format!(
        "SELECT {} FROM {}",
        select_list,
        quote_ident(table_name)
    )];
    if !where_sql.is_empty() {
        parts.push(where_sql.to_string());
    }
    if !order_clause.is_empty() {
        parts.push(order_clause.to_string());
    }
    parts.push(format!("LIMIT {} OFFSET {}", limit, offset));
    parts.join(" ")
}

pub(crate) async fn prepare_grid_session_metadata(
    state: &Arc<AppState>,
    params: &GridDataRequest,
) -> Result<Option<SheetMetadata>, String> {
    if let Some(sid) = &params.session_id {
        if !sid.is_empty() {
            if let Err(e) = state
                .session_manager
                .switch_session(&params.table_name, sid)
                .await
            {
                return Err(format!("Failed to switch session: {}", e));
            }
        }
    }

    if let Err(e) = state
        .session_manager
        .register_session_to_context(&state.ctx, &params.table_name)
        .await
    {
        if !e.contains("No active session") {
            println!(
                "[get_grid_data] Warning: Failed to register session for '{}': {}. Query may use stale/original data.",
                params.table_name,
                e
            );
        }
    }

    Ok(state
        .session_manager
        .get_metadata(&params.table_name)
        .await
        .ok())
}

pub(crate) async fn fetch_grid_data(
    state: &Arc<AppState>,
    params: &GridDataRequest,
) -> Result<GridDataResult, String> {
    let metadata = prepare_grid_session_metadata(state, params).await?;

    let mut where_clauses = Vec::new();
    if let Some(filters_json) = &params.filters {
        if let Ok(filters) = serde_json::from_str::<Vec<serde_json::Value>>(filters_json) {
            for f in filters {
                if let (Some(col), Some(vals)) = (
                    f.get("col").and_then(|v| v.as_str()),
                    f.get("val").and_then(|v| v.as_array()),
                ) {
                    let val_list: Vec<String> = vals
                        .iter()
                        .map(|v| format!("'{}'", v.as_str().unwrap_or("").replace("'", "''")))
                        .collect();
                    if !val_list.is_empty() {
                        where_clauses.push(format!(
                            "{} IN ({})",
                            quote_ident(col),
                            val_list.join(", ")
                        ));
                    }
                }
            }
        }
    }

    let mut order_clause = String::new();
    if let Some(sort_json) = &params.sort {
        if let Ok(sort) = serde_json::from_str::<serde_json::Value>(sort_json) {
            if let (Some(col), Some(order)) = (
                sort.get("col").and_then(|v| v.as_str()),
                sort.get("order").and_then(|v| v.as_str()),
            ) {
                let direction = if order.to_lowercase() == "desc" {
                    "DESC"
                } else {
                    "ASC"
                };
                order_clause = format!("ORDER BY {} {}", quote_ident(col), direction);
            }
        }
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let mut count_sql = format!(
        "SELECT COUNT(*) as count FROM {} {}",
        quote_ident(&params.table_name),
        where_sql
    );

    // Attempt to rewrite query to fix case sensitivity issues
    /*
    **Modification 2026-02-14**:
    - **Reason**: Fix "table not found" error caused by case sensitivity mismatch. DataFusion treats quoted identifiers as case-sensitive, but the frontend might request a name with different casing than registered.
    - **Purpose**: Use `query_rewriter` to parse the SQL, check if the table exists, and if not, find a case-insensitive match in the catalog and rewrite the query with the correct quoted name.
    */
    if let Ok(rewritten) = crate::query_rewriter::rewrite_query(&state.ctx, &count_sql).await {
        count_sql = rewritten;
    }

    let total_rows = match state.ctx.sql(&count_sql).await {
        Ok(df) => {
            let batches = df.collect().await.unwrap_or_default();
            if !batches.is_empty() && batches[0].num_rows() > 0 {
                let col = batches[0].column(0);
                let val = col
                    .as_any()
                    .downcast_ref::<datafusion::arrow::array::Int64Array>()
                    .map(|a| a.value(0))
                    .unwrap_or(0);
                val as usize
            } else {
                0
            }
        }
        Err(_) => 0,
    };

    let offset = (params.page - 1) * params.page_size;

    let column_names = state
        .ctx
        .table_provider(&params.table_name)
        .await
        .ok()
        .map(|provider| {
            provider
                .schema()
                .fields()
                .iter()
                .map(|f| f.name().clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let formula_markers = load_formula_markers(state, &params.table_name);
    let mut computed_columns = HashMap::new();
    let mut formula_columns = Vec::new();

    for (idx, name) in column_names.iter().enumerate() {
        if let Some(marker) = formula_markers.get(idx).and_then(|m| m.clone()) {
            // **[2026-02-16]** 变更原因：公式列可能包含 Utf8。
            // **[2026-02-16]** 变更目的：重建 SQL 并忽略非数字。
            let rebuilt_sql =
                rebuild_formula_sql(&marker.raw, &column_names).unwrap_or(marker.sql.clone());
            computed_columns.insert(name.clone(), rebuilt_sql.clone());
            formula_columns.push(FormulaColumnMeta {
                index: idx,
                name: name.clone(),
                raw_expression: marker.raw,
                sql_expression: rebuilt_sql,
            });
        }
    }

    let mut sql = build_grid_select_sql(
        &params.table_name,
        &where_sql,
        &order_clause,
        params.page_size,
        offset,
        &column_names,
        &computed_columns,
    );

    println!("[GridData] Executing (Original): {}", sql);

    // Attempt to rewrite query to fix case sensitivity issues
    /*
    **Modification 2026-02-14**:
    - **Reason**: Fix "table not found" error caused by case sensitivity mismatch.
    - **Purpose**: Ensure the main data query also undergoes table name correction (case-insensitive lookup) before execution.
    */
    if let Ok(rewritten) = crate::query_rewriter::rewrite_query(&state.ctx, &sql).await {
        println!("[GridData] Rewritten SQL: {}", rewritten);
        sql = rewritten;
    }

    match state.ctx.sql(&sql).await {
        Ok(df) => {
            let batches = df.collect().await;
            match batches {
                Ok(batches) => {
                    let mut rows = Vec::new();
                    let mut columns = Vec::new();
                    let mut column_types = Vec::new();

                    if !batches.is_empty() {
                        let schema = batches[0].schema();
                        columns = schema.fields().iter().map(|f| f.name().clone()).collect();
                        column_types = schema
                            .fields()
                            .iter()
                            .map(|f| f.data_type().to_string())
                            .collect();

                        for batch in batches {
                            let num_rows = batch.num_rows();
                            let num_cols = batch.num_columns();
                            for i in 0..num_rows {
                                let mut row_data = Vec::new();
                                for j in 0..num_cols {
                                    let col = batch.column(j);
                                    let val_str =
                                        datafusion::arrow::util::display::array_value_to_string(
                                            col, i,
                                        )
                                        .unwrap_or_default();
                                    row_data.push(val_str);
                                }
                                rows.push(row_data);
                            }
                        }
                    }

                    Ok(GridDataResult {
                        rows,
                        columns,
                        column_types,
                        total_rows,
                        metadata,
                        formula_columns: if formula_columns.is_empty() {
                            None
                        } else {
                            Some(formula_columns)
                        },
                    })
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_grid_select_sql_includes_formula_columns() {
        let columns = vec!["col_a".to_string(), "calc_cf".to_string()];
        let mut computed = HashMap::new();
        computed.insert("calc_cf".to_string(), "\"col_c\" * \"col_f\"".to_string());

        let sql = build_grid_select_sql(
            "demo_table",
            "WHERE \"col_a\" = '1'",
            "ORDER BY \"col_a\" ASC",
            50,
            100,
            &columns,
            &computed,
        );

        assert_eq!(
            sql,
            "SELECT \"col_a\", \"col_c\" * \"col_f\" AS \"calc_cf\" FROM \"demo_table\" WHERE \"col_a\" = '1' ORDER BY \"col_a\" ASC LIMIT 50 OFFSET 100"
        );
    }

    #[test]
    fn test_rebuild_formula_sql_casts_columns() {
        // **[2026-02-16]** 变更原因：新增公式列安全转换用例。
        // **[2026-02-16]** 变更目的：确保 Utf8 与数值混算可被转换。
        // **[2026-02-17]** 变更原因：新增 VARCHAR 中间层避免 NULLIF 触发类型冲突。
        // **[2026-02-17]** 变更目的：更新断言与新公式 SQL 保持一致。
        let columns = vec![
            "name".to_string(),
            "amount".to_string(),
            "score".to_string(),
        ];
        let sql = rebuild_formula_sql("B+C", &columns);
        assert_eq!(
            sql,
            Some("TRY_CAST(NULLIF(CAST(\"amount\" AS VARCHAR), '') AS DOUBLE)+TRY_CAST(NULLIF(CAST(\"score\" AS VARCHAR), '') AS DOUBLE)".to_string())
        );
    }

    #[test]
    fn test_rebuild_formula_sql_rejects_out_of_range() {
        // **[2026-02-16]** 变更原因：覆盖非法列标场景。
        // **[2026-02-16]** 变更目的：避免构造错误 SQL。
        let columns = vec!["name".to_string()];
        let sql = rebuild_formula_sql("Z+1", &columns);
        assert_eq!(sql, None);
    }

    #[test]
    fn test_build_safe_cast_expr_casts_to_varchar_before_nullif() {
        // **[2026-02-17]** 变更原因：复现数值列参与 NULLIF 时的类型冲突风险。
        // **[2026-02-17]** 变更目的：确保生成表达式先 CAST 再 NULLIF。
        let sql = build_safe_cast_expr("amount");
        assert_eq!(
            sql,
            "TRY_CAST(NULLIF(CAST(\"amount\" AS VARCHAR), '') AS DOUBLE)"
        );
    }
}
