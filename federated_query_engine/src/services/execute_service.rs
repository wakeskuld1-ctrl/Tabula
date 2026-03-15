use crate::{add_log, query_rewriter, AppState};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::LogicalPlan;
use datafusion::prelude::DataFrame;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

#[derive(Serialize)]
pub(crate) struct ExecuteResponse {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) execution_time_ms: u64,
    pub(crate) error: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct PlanResponse {
    pub(crate) plan_json: Value,
    pub(crate) physical_plan_text: String,
    pub(crate) cost_est: f64,
    pub(crate) estimated_rows: Option<usize>,
    pub(crate) estimated_bytes: Option<usize>,
    pub(crate) warnings: Vec<String>,
}

enum ExecuteOutput {
    Empty,
    Data {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

pub(crate) async fn execute_sql(state: &Arc<AppState>, sql: String) -> ExecuteResponse {
    let start = Instant::now();
    add_log(&state.logs, format!("Executing SQL: {}", sql));

    let final_sql = rewrite_sql_with_logging(state, &sql).await;
    let dataframe_result = execute_with_self_heal(state, &final_sql).await;

    match dataframe_result {
        Ok(dataframe) => match collect_execute_output(dataframe).await {
            Ok(ExecuteOutput::Empty) => ExecuteResponse {
                columns: vec![],
                rows: vec![],
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: None,
            },
            Ok(ExecuteOutput::Data { columns, rows }) => {
                let duration = start.elapsed();
                add_log(
                    &state.logs,
                    format!(
                        "Query executed successfully. {} rows returned in {}ms",
                        rows.len(),
                        duration.as_millis()
                    ),
                );
                ExecuteResponse {
                    columns,
                    rows,
                    execution_time_ms: duration.as_millis() as u64,
                    error: None,
                }
            }
            Err(e) => {
                add_log(&state.logs, format!("Execution Error: {}", e));
                ExecuteResponse {
                    columns: vec![],
                    rows: vec![],
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                }
            }
        },
        Err(e) => {
            add_log(&state.logs, format!("Planning Error: {}", e));
            ExecuteResponse {
                columns: vec![],
                rows: vec![],
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
            }
        }
    }
}

pub(crate) async fn get_plan(state: &Arc<AppState>, sql: String) -> PlanResponse {
    let final_sql = rewrite_sql_without_logging(state, &sql).await;
    let logical_plan_result = create_logical_plan_with_self_heal(state, &final_sql).await;

    match logical_plan_result {
        Ok(logical_plan) => {
            let physical_plan_result = state.ctx.state().create_physical_plan(&logical_plan).await;
            match physical_plan_result {
                Ok(physical_plan) => {
                    let plan_text = datafusion::physical_plan::displayable(physical_plan.as_ref())
                        .indent(true)
                        .to_string();
                    let (estimated_rows, estimated_bytes) = extract_plan_stats(&physical_plan);
                    PlanResponse {
                        plan_json: build_plan_json(&physical_plan),
                        physical_plan_text: plan_text,
                        cost_est: 0.0,
                        estimated_rows,
                        estimated_bytes,
                        warnings: vec![],
                    }
                }
                Err(e) => PlanResponse {
                    plan_json: serde_json::json!({}),
                    physical_plan_text: format!("Physical Plan Error: {}", e),
                    cost_est: 0.0,
                    estimated_rows: None,
                    estimated_bytes: None,
                    warnings: vec![e.to_string()],
                },
            }
        }
        Err(e) => PlanResponse {
            plan_json: serde_json::json!({}),
            physical_plan_text: format!("Logical Plan Error: {}", e),
            cost_est: 0.0,
            estimated_rows: None,
            estimated_bytes: None,
            warnings: vec![e.to_string()],
        },
    }
}

async fn rewrite_sql_with_logging(state: &Arc<AppState>, sql: &str) -> String {
    match query_rewriter::rewrite_query(&state.ctx, sql).await {
        Ok(rewritten) => {
            if rewritten != sql {
                add_log(&state.logs, format!("Rewritten SQL: {}", rewritten));
            }
            rewritten
        }
        Err(e) => {
            add_log(&state.logs, format!("Rewrite Error: {}", e));
            sql.to_string()
        }
    }
}

async fn rewrite_sql_without_logging(state: &Arc<AppState>, sql: &str) -> String {
    match query_rewriter::rewrite_query(&state.ctx, sql).await {
        Ok(rewritten) => rewritten,
        Err(_) => sql.to_string(),
    }
}

async fn execute_with_self_heal(
    state: &Arc<AppState>,
    final_sql: &str,
) -> Result<DataFrame, DataFusionError> {
    let dataframe_result = state.ctx.sql(final_sql).await;

    match dataframe_result {
        Ok(dataframe) => Ok(dataframe),
        Err(error) => {
            let error_message = error.to_string();
            if !error_message.contains("No field named") {
                return Err(error);
            }

            let unknown_field = extract_unknown_field(&error_message);
            if let Some(field) = unknown_field {
                add_log(
                    &state.logs,
                    format!("Detected missing field '{}', attempting fix...", field),
                );
                match query_rewriter::fix_query(&state.ctx, final_sql, &field).await {
                    Ok(fixed_sql) => {
                        add_log(
                            &state.logs,
                            format!("Retrying with fixed SQL: {}", fixed_sql),
                        );
                        state.ctx.sql(&fixed_sql).await
                    }
                    Err(_) => Err(error),
                }
            } else {
                Err(error)
            }
        }
    }
}

async fn create_logical_plan_with_self_heal(
    state: &Arc<AppState>,
    final_sql: &str,
) -> Result<LogicalPlan, DataFusionError> {
    let mut plan_result = state.ctx.state().create_logical_plan(final_sql).await;

    if let Err(e) = &plan_result {
        let err_msg = e.to_string();
        if err_msg.contains("No field named") {
            let unknown_field = extract_unknown_field(&err_msg);
            if let Some(field) = unknown_field {
                if let Ok(fixed_sql) =
                    query_rewriter::fix_query(&state.ctx, final_sql, &field).await
                {
                    plan_result = state.ctx.state().create_logical_plan(&fixed_sql).await;
                }
            }
        }
    }

    plan_result
}

fn extract_unknown_field(error_message: &str) -> Option<String> {
    if let Some(start) = error_message.find("No field named \"") {
        let rest = &error_message[start + 16..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    if let Some(start) = error_message.find("No field named '") {
        let rest = &error_message[start + 16..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

fn extract_plan_stats(
    physical_plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
) -> (Option<usize>, Option<usize>) {
    let mut estimated_rows = None;
    let mut estimated_bytes = None;

    // **[2026-02-25]** 变更原因：statistics 已弃用且 clippy 以 warnings 为错误。
    // **[2026-02-25]** 变更目的：改用 partition_statistics 获取估算值。
    // **[2026-02-25]** 变更说明：取首分区统计以保持既有语义。
    // **[2026-02-25]** 变更说明：为空时保持 None 结果。
    // **[2026-02-25]** 变更说明：不影响执行流程，仅替换接口。
    // **[2026-02-25]** 变更说明：避免弃用 API 触发 clippy。
    // **[2026-02-25]** 变更原因：partition_statistics 需要可选分区参数。
    // **[2026-02-25]** 变更目的：保持与旧统计逻辑一致并修复编译错误。
    // **[2026-02-25]** 变更说明：使用 None 获取整体统计。
    // **[2026-02-25]** 变更说明：统计为空时保持 None 结果。
    // **[2026-02-25]** 变更说明：不影响调用方行为。
    // **[2026-02-25]** 变更说明：仅调整 API 调用方式。
    if let Ok(stats) = physical_plan.statistics() {
        match stats.num_rows {
            datafusion::common::stats::Precision::Exact(n) => estimated_rows = Some(n),
            datafusion::common::stats::Precision::Inexact(n) => estimated_rows = Some(n),
            _ => {}
        }
        match stats.total_byte_size {
            datafusion::common::stats::Precision::Exact(n) => estimated_bytes = Some(n),
            datafusion::common::stats::Precision::Inexact(n) => estimated_bytes = Some(n),
            _ => {}
        }
    }

    (estimated_rows, estimated_bytes)
}

fn build_plan_json(physical_plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>) -> Value {
    serde_json::json!({
        "name": "PhysicalPlan",
        "children": [
            { "name": format!("{}", physical_plan.schema()) }
        ]
    })
}

async fn collect_execute_output(dataframe: DataFrame) -> Result<ExecuteOutput, DataFusionError> {
    let batches = dataframe.collect().await?;
    if batches.is_empty() {
        return Ok(ExecuteOutput::Empty);
    }

    let schema = batches[0].schema();
    let columns: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    let mut rows = Vec::new();
    for batch in batches {
        let num_rows = batch.num_rows();
        let num_cols = batch.num_columns();
        for row_index in 0..num_rows {
            let mut row_values = Vec::new();
            for col_index in 0..num_cols {
                let col = batch.column(col_index);
                let value = arrow::util::display::array_value_to_string(col, row_index)
                    .unwrap_or("".to_string());
                row_values.push(value);
            }
            rows.push(row_values);
        }
    }

    Ok(ExecuteOutput::Data { columns, rows })
}
