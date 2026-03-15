use serde::Deserialize;
use std::sync::Arc;

use crate::datasources::{
    csv::CsvDataSource, excel::ExcelDataSource, parquet::ParquetDataSource,
    sqlite::SqliteDataSource, DataSource,
};
// **[2026-02-25]** 变更原因：register_table 改为参数结构体。
// **[2026-02-25]** 变更目的：引入参数类型以适配新签名。
// **[2026-02-25]** 变更说明：仅增加类型引用。
// **[2026-02-25]** 变更说明：不影响其他模块依赖。
// **[2026-02-25]** 变更说明：保持原有使用方式。
// **[2026-02-25]** 变更说明：避免重复定义结构体。
use crate::metadata_manager::RegisterTableParams;
use crate::{add_log, AppState};

#[derive(Deserialize)]
pub(crate) struct RegisterTableRequest {
    pub(crate) file_path: String,
    pub(crate) table_name: String,
    pub(crate) sheet_name: Option<String>,
    pub(crate) source_type: String,
    #[serde(default)]
    pub(crate) header_rows: Option<usize>,
    #[serde(default)]
    pub(crate) header_mode: Option<String>,
}

pub(crate) async fn register_table(
    state: &Arc<AppState>,
    payload: RegisterTableRequest,
) -> serde_json::Value {
    // let cache_manager = state.cache_context.cache_manager();
    let header_rows = payload.header_rows.unwrap_or(0);
    let header_mode = payload
        .header_mode
        .clone()
        .unwrap_or_else(|| "none".to_string());

    // **[2026-02-25]** 变更原因：clippy 提示 double_ended_iterator_last。
    // **[2026-02-25]** 变更目的：使用 next_back 提升迭代语义。
    // **[2026-02-25]** 变更说明：表名清洗逻辑保持一致。
    // **[2026-02-25]** 变更说明：仍以最后一段为最终表名。
    // **[2026-02-25]** 变更说明：避免反向迭代多余消耗。
    // **[2026-02-25]** 变更说明：与 clippy 建议一致。
    let table_name_clean = payload
        .table_name
        .split('.')
        .next_back()
        .unwrap_or(&payload.table_name)
        .to_string();

    if table_name_clean != payload.table_name {
        println!(
            "Sanitized table name: '{}' -> '{}'",
            payload.table_name, table_name_clean
        );
    }

    let source: Option<Box<dyn DataSource>> = if payload.source_type == "excel" {
        let sheet = payload
            .sheet_name
            .clone()
            .unwrap_or_else(|| "Sheet1".to_string());
        match crate::cache_manager::CacheManager::ensure_parquet_cache(
            &payload.file_path,
            "excel",
            Some(sheet.clone()),
            // Some(header_rows), // Not supported
            // Some(header_mode.clone()), // Not supported
        )
        .await
        {
            Ok(p) => Some(Box::new(ParquetDataSource::new(
                table_name_clean.clone(),
                p,
            ))),
            Err(e) => {
                eprintln!(
                    "Transcoding failed for {}, using original Excel: {}",
                    table_name_clean, e
                );
                Some(Box::new(ExcelDataSource::new(
                    table_name_clean.clone(),
                    payload.file_path.clone(),
                    sheet,
                    // header_rows, // Not supported by current ExcelDataSource
                    // header_mode.clone(), // Not supported
                )))
            }
        }
    } else if payload.source_type == "csv" {
        match crate::cache_manager::CacheManager::ensure_parquet_cache(
            &payload.file_path,
            "csv",
            None,
        )
        .await
        {
            Ok(p) => Some(Box::new(ParquetDataSource::new(
                table_name_clean.clone(),
                p,
            ))),
            Err(e) => {
                eprintln!(
                    "Transcoding failed for {}, using original CSV: {}",
                    table_name_clean, e
                );
                Some(Box::new(CsvDataSource::new(
                    table_name_clean.clone(),
                    payload.file_path.clone(),
                )))
            }
        }
    } else if payload.source_type == "sqlite" {
        let internal_name = payload
            .sheet_name
            .clone()
            .unwrap_or(table_name_clean.clone());
        Some(Box::new(SqliteDataSource::new(
            table_name_clean.clone(),
            payload.file_path.clone(),
            internal_name,
            // state.cache_context.clone(), // Not supported
        )))
    } else {
        None
    };

    if let Some(source) = source {
        if source.register(&state.ctx).await.is_ok() {
            crate::add_log(
                &state.logs,
                format!("Manually registered data source: {}", source.name()),
            );

            if let Err(e) = state
                .metadata_manager
                .register_table(
                    &state.ctx,
                    crate::metadata_manager::RegisterTableParams {
                        catalog: "datafusion",
                        schema: "public",
                        table: &table_name_clean,
                        file_path: &payload.file_path,
                        source_type: &payload.source_type,
                        sheet_name: payload.sheet_name,
                        header_rows: Some(header_rows),
                        header_mode: Some(header_mode),
                    },
                )
                .await
            {
                eprintln!(
                    "Failed to persist metadata for table {}: {}",
                    table_name_clean, e
                );
            }

            return serde_json::json!({
                "status": "ok",
                "message": "Registered successfully"
            });
        }
    }

    serde_json::json!({
        "status": "error",
        "message": "Registration failed"
    })
}
