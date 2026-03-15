use axum::extract::Multipart;
use std::sync::Arc;

use crate::datasources::{
    csv::CsvDataSource, excel::ExcelDataSource, parquet::ParquetDataSource, DataSource,
};
// **[2026-02-25]** 变更原因：register_table 改为参数结构体。
// **[2026-02-25]** 变更目的：上传注册流程复用统一参数类型。
// **[2026-02-25]** 变更说明：仅新增类型导入。
// **[2026-02-25]** 变更说明：不影响现有数据源逻辑。
// **[2026-02-25]** 变更说明：保持调用方式一致。
// **[2026-02-25]** 变更说明：避免重复结构体定义。
use crate::metadata_manager::RegisterTableParams;
use crate::{add_log, AppState};

pub(crate) async fn handle_upload(
    state: &Arc<AppState>,
    header_rows: usize,
    header_mode: String,
    mut multipart: Multipart,
) -> serde_json::Value {
    loop {
        let field_res = multipart.next_field().await;

        match field_res {
            Ok(Some(field)) => {
                let file_name = match field.file_name() {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                let data = match field.bytes().await {
                    Ok(d) => d,
                    Err(e) => {
                        return serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to read bytes: {}", e)
                        });
                    }
                };

                let data_dir = if std::path::Path::new("federated_query_engine/data").exists() {
                    std::path::Path::new("federated_query_engine/data")
                } else {
                    std::path::Path::new("data")
                };

                if let Err(e) = std::fs::create_dir_all(data_dir) {
                    return serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to create data dir: {}", e)
                    });
                }

                let path = data_dir.join(&file_name);

                if let Err(e) = std::fs::write(&path, data) {
                    return serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to write file: {}", e)
                    });
                }
                let path_str = path.to_str().unwrap_or_default().to_string();

                add_log(&state.logs, format!("File uploaded: {}", file_name));

                let raw_name = std::path::Path::new(&file_name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("unknown");
                let table_name = raw_name.replace(['.', '-', ' ', '(', ')'], "_");

                let extension = std::path::Path::new(&file_name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                // let cache_manager = state.cache_context.cache_manager();

                let mut single_sheet_name = None;

                if extension == "xlsx" {
                    if let Ok(sheets) = ExcelDataSource::get_sheet_names(&path_str) {
                        if sheets.len() > 1 {
                            return serde_json::json!({
                                "status": "select_sheet",
                                "message": "Multiple sheets detected. Please select one.",
                                "file_path": path_str,
                                "sanitized_name": table_name,
                                "sheets": sheets,
                                "header_rows": header_rows,
                                "header_mode": header_mode
                            });
                        } else if sheets.len() == 1 {
                            single_sheet_name = Some(sheets[0].clone());
                        } else {
                            return serde_json::json!({
                                "status": "error",
                                "message": "Excel file contains no sheets"
                            });
                        }
                    }
                }

                let (source, final_path, final_source_type): (
                    Option<Box<dyn DataSource>>,
                    String,
                    String,
                ) = if extension == "csv" {
                    match crate::cache_manager::CacheManager::ensure_parquet_cache(
                        &path_str, "csv", None,
                    )
                    .await
                    {
                        Ok(p) => (
                            Some(Box::new(ParquetDataSource::new(
                                table_name.clone(),
                                p.clone(),
                            ))),
                            p,
                            "parquet".to_string(),
                        ),
                        Err(e) => {
                            eprintln!(
                                "Transcoding failed for {}, falling back to CSV: {}",
                                file_name, e
                            );
                            (
                                Some(Box::new(CsvDataSource::new(
                                    table_name.clone(),
                                    path_str.clone(),
                                ))),
                                path_str.clone(),
                                "csv".to_string(),
                            )
                        }
                    }
                } else if extension == "xlsx" {
                    let sheet = single_sheet_name
                        .clone()
                        .unwrap_or_else(|| "Sheet1".to_string());

                    match crate::cache_manager::CacheManager::ensure_parquet_cache(
                        &path_str,
                        "excel",
                        Some(sheet.clone()),
                        // Some(header_rows),
                        // Some(header_mode.clone()),
                    )
                    .await
                    {
                        Ok(p) => (
                            Some(Box::new(ParquetDataSource::new(
                                table_name.clone(),
                                p.clone(),
                            ))),
                            p,
                            "parquet".to_string(),
                        ),
                        Err(e) => {
                            eprintln!(
                                "Transcoding failed for {}, falling back to Excel: {}",
                                file_name, e
                            );
                            (
                                Some(Box::new(ExcelDataSource::new(
                                    table_name.clone(),
                                    path_str.clone(),
                                    sheet,
                                    // header_rows,
                                    // header_mode.clone(),
                                ))),
                                path_str.clone(),
                                "excel".to_string(),
                            )
                        }
                    }
                } else {
                    (None, path_str.clone(), extension.clone())
                };

                if let Some(source) = source {
                    let reg_res = source.register(&state.ctx).await;
                    if reg_res.is_ok() {
                        println!("Auto-registered uploaded data source: {}", source.name());

                        let sheet_name = if extension == "xlsx" {
                            Some(if let Some(s) = &single_sheet_name {
                                s.clone()
                            } else {
                                "Sheet1".to_string()
                            })
                        } else {
                            None
                        };

                        // **[2026-02-25]** 变更原因：register_table 改为参数结构体。
                        // **[2026-02-25]** 变更目的：上传注册统一参数传递方式。
                        // **[2026-02-25]** 变更说明：保持 sheet/header 信息不变。
                        // **[2026-02-25]** 变更说明：仅调整参数构造方式。
                        // **[2026-02-25]** 变更说明：不改变持久化逻辑。
                        // **[2026-02-25]** 变更说明：避免多参数签名触发 clippy。
                        if let Err(e) = state
                            .metadata_manager
                            .register_table(
                                &state.ctx,
                                RegisterTableParams {
                                    catalog: "datafusion",
                                    schema: "public",
                                    table: &table_name,
                                    file_path: &final_path,
                                    source_type: &final_source_type,
                                    sheet_name,
                                    header_rows: Some(header_rows),
                                    header_mode: Some(header_mode),
                                },
                            )
                            .await
                        {
                            eprintln!("Failed to persist metadata for table {}: {}", table_name, e);
                        } else {
                            println!("Persisted metadata for table: {}", table_name);
                        }

                        return serde_json::json!({
                            "status": "ok",
                            "message": format!("Uploaded and optimized {} as table '{}' (Format: {})", file_name, table_name, final_source_type),
                            "table": table_name
                        });
                    } else {
                        let err = reg_res.err().unwrap();
                        eprintln!("Registration failed for {}: {}", table_name, err);
                        return serde_json::json!({
                            "status": "error",
                            "message": format!("Uploaded {} but failed to register: {}", file_name, err)
                        });
                    }
                }

                return serde_json::json!({
                    "status": "ok",
                    "message": format!("Uploaded {} (not registered, unsupported type)", file_name)
                });
            }
            Ok(None) => break,
            Err(e) => {
                return serde_json::json!({
                    "status": "error",
                    "message": format!("Multipart error: {}", e)
                });
            }
        }
    }

    serde_json::json!({ "status": "error", "message": "No valid file found in request" })
}
