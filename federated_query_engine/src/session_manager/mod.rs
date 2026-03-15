use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, Date64Array, Float64Array, Int64Array, StringArray,
    TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
    TimestampSecondArray,
};
use arrow::datatypes::{
    DataType, Date32Type, Date64Type, Field, Float64Type, Int64Type, Schema, TimeUnit,
    TimestampMicrosecondType, TimestampMillisecondType, TimestampNanosecondType,
    TimestampSecondType,
};
use arrow::record_batch::{RecordBatch, RecordBatchIterator};
use chrono::{NaiveDate, NaiveDateTime};
use datafusion::prelude::*;
use futures::TryStreamExt;
use lance::dataset::{Dataset, WriteParams};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{self, Duration};
use uuid::Uuid;

use crate::metadata_manager::MetadataManager;
use chrono::Datelike;
use metadata_store::{Session as DbSession, SheetAttribute, TableMetadata};

use serde::{Deserialize, Serialize};

// --- Style & Metadata Structures ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellStyle {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub align: Option<String>, // "left", "center", "right"
    pub color: Option<String>, // Hex code e.g. "#FF0000"
    pub bg_color: Option<String>,
    // **[2026-02-16]** 变更原因：新增单元格格式字段。
    // **[2026-02-16]** 变更目的：支持数值/百分比/货币/日期显示。
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRange {
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SheetMetadata {
    // Key: "row,col" string. E.g. "0,1" for Row 0, Col 1 (B1)
    pub styles: HashMap<String, CellStyle>,
    pub merges: Vec<MergeRange>,
}

// -----------------------------------

// Session Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub table_name: String,
    pub name: String, // Friendly name e.g. "Draft 1"
    pub lance_path: String,
    pub created_at: u64,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub from_session_id: Option<String>, // Parent Session ID (if forked)
    #[serde(skip)]
    pub current_data: Option<Arc<RwLock<Vec<RecordBatch>>>>, // In-Memory Editing State
    #[serde(default)]
    pub metadata: SheetMetadata, // Metadata (Styles, Merges) - Persisted
    #[serde(skip)]
    pub dirty: bool,
    #[serde(skip)]
    pub pending_writes: usize,
    #[serde(skip)]
    pub last_modified_at: u64,
    #[serde(skip)]
    pub last_persisted_at: u64,
    #[serde(skip)]
    pub is_flushing: bool,
}

#[derive(Debug, Clone)]
pub struct AutoFlushConfig {
    pub interval_ms: u64,
    pub max_pending_writes: usize,
    pub max_dirty_ms: u64,
}

impl Default for AutoFlushConfig {
    /// 提供自动落盘的默认阈值配置。
    fn default() -> Self {
        Self {
            interval_ms: 5000,
            max_pending_writes: 20,
            max_dirty_ms: 15000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCellError {
    NoActiveSession { table_name: String },
    SessionNotFound { session_id: String },
    ColumnNotFound { column: String },
    SessionDataNotLoaded,
    EmptyDataset,
    LoadLanceFailed { source: String, reason: String },
    CastFailed { reason: String },
    UnsupportedType { data_type: String },
    Internal { reason: String },
}

impl UpdateCellError {
    /// 返回错误码标识。
    pub fn code(&self) -> &'static str {
        match self {
            UpdateCellError::NoActiveSession { .. } => "NO_ACTIVE_SESSION",
            UpdateCellError::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            UpdateCellError::ColumnNotFound { .. } => "COLUMN_NOT_FOUND",
            UpdateCellError::SessionDataNotLoaded => "SESSION_DATA_NOT_LOADED",
            UpdateCellError::EmptyDataset => "EMPTY_DATASET",
            UpdateCellError::LoadLanceFailed { .. } => "LANCE_LOAD_FAILED",
            UpdateCellError::CastFailed { .. } => "TYPE_CAST_FAILED",
            UpdateCellError::UnsupportedType { .. } => "UNSUPPORTED_TYPE",
            UpdateCellError::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    /// 返回错误类型分类。
    pub fn error_type(&self) -> &'static str {
        match self {
            UpdateCellError::NoActiveSession { .. } => "session_missing",
            UpdateCellError::SessionNotFound { .. } => "session_missing",
            UpdateCellError::ColumnNotFound { .. } => "schema_error",
            UpdateCellError::SessionDataNotLoaded => "memory_missing",
            UpdateCellError::EmptyDataset => "data_empty",
            UpdateCellError::LoadLanceFailed { .. } => "storage_error",
            UpdateCellError::CastFailed { .. } => "type_error",
            UpdateCellError::UnsupportedType { .. } => "type_error",
            UpdateCellError::Internal { .. } => "internal_error",
        }
    }

    /// 返回面向用户的错误提示。
    pub fn message(&self) -> String {
        match self {
            UpdateCellError::NoActiveSession { table_name } => {
                format!("未找到活动会话，表：{}", table_name)
            }
            UpdateCellError::SessionNotFound { session_id } => {
                format!("会话不存在：{}", session_id)
            }
            UpdateCellError::ColumnNotFound { column } => format!("列不存在：{}", column),
            UpdateCellError::SessionDataNotLoaded => "会话内存数据未加载".to_string(),
            UpdateCellError::EmptyDataset => "数据为空，无法更新".to_string(),
            UpdateCellError::LoadLanceFailed { .. } => "加载存储数据失败".to_string(),
            UpdateCellError::CastFailed { .. } => "类型转换失败".to_string(),
            UpdateCellError::UnsupportedType { .. } => "不支持的字段类型".to_string(),
            UpdateCellError::Internal { .. } => "内部错误".to_string(),
        }
    }

    /// 返回用于排查的错误细节。
    pub fn details(&self) -> String {
        match self {
            UpdateCellError::NoActiveSession { table_name } => format!("table_name={}", table_name),
            UpdateCellError::SessionNotFound { session_id } => format!("session_id={}", session_id),
            UpdateCellError::ColumnNotFound { column } => format!("column={}", column),
            UpdateCellError::SessionDataNotLoaded => "current_data=None".to_string(),
            UpdateCellError::EmptyDataset => "batches为空或缺少schema".to_string(),
            UpdateCellError::LoadLanceFailed { source, reason } => {
                format!("source={}, reason={}", source, reason)
            }
            UpdateCellError::CastFailed { reason } => format!("reason={}", reason),
            UpdateCellError::UnsupportedType { data_type } => format!("data_type={}", data_type),
            UpdateCellError::Internal { reason } => format!("reason={}", reason),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellUpdate {
    pub row: usize,
    pub col: String,
    pub val: String,
}

// Session State
// Tracks active Lance datasets for each table (per user in future, currently global for demo)
pub struct SessionManager {
    base_path: PathBuf,
    metadata_manager: Arc<MetadataManager>,
    // session_id -> SessionInfo
    sessions: Mutex<HashMap<String, SessionInfo>>,
    // table_name -> active session_id (which session is currently being viewed/edited)
    active_table_sessions: Mutex<HashMap<String, String>>,
    auto_flush: AutoFlushConfig,
}

/// 获取当前时间的毫秒级时间戳。
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// **[2026-02-15]** 变更原因：插入行时需要按列默认公式构造新行数据。
#[derive(Debug, Deserialize)]
struct FormulaMarker {
    kind: String,
    raw: String,
    sql: String,
}

fn is_formula_marker(raw: &str) -> bool {
    // **[2026-02-25]** 变更原因：clippy 提示 FormulaMarker.raw 未使用。
    // **[2026-02-25]** 变更目的：在解析校验中纳入 raw 字段。
    // **[2026-02-25]** 变更说明：逻辑仍以 kind 与 sql 判断为主。
    // **[2026-02-25]** 变更说明：避免空 raw 造成误识别。
    // **[2026-02-25]** 变更说明：仅增加校验条件。
    // **[2026-02-25]** 变更说明：保持原有错误处理方式。
    serde_json::from_str::<FormulaMarker>(raw)
        .map(|marker| {
            marker.kind == "formula"
                && !marker.raw.trim().is_empty()
                && !marker.sql.trim().is_empty()
        })
        .unwrap_or(false)
}

fn build_row_with_defaults(
    schema: &Schema,
    default_formulas: &[Option<String>],
) -> Vec<Arc<dyn Array>> {
    schema
        .fields()
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            if field.data_type() == &DataType::Utf8 {
                if let Some(Some(formula)) = default_formulas.get(idx) {
                    if is_formula_marker(formula) {
                        arrow::array::new_null_array(field.data_type(), 1)
                    } else {
                        Arc::new(StringArray::from(vec![formula.clone()])) as ArrayRef
                    }
                } else {
                    arrow::array::new_null_array(field.data_type(), 1)
                }
            } else {
                arrow::array::new_null_array(field.data_type(), 1)
            }
        })
        .collect()
}

// **[2026-02-25]** 变更原因：create_session_internal 参数过多触发 clippy。
// **[2026-02-25]** 变更目的：将会话创建参数聚合为结构体。
// **[2026-02-25]** 变更说明：仅用于内部调用，减少签名噪音。
// **[2026-02-25]** 变更说明：字段语义与原参数一一对应。
// **[2026-02-25]** 变更说明：不改变调用与逻辑顺序。
// **[2026-02-25]** 变更说明：便于扩展与维护。
struct CreateSessionParams<'a> {
    sessions: &'a mut HashMap<String, SessionInfo>,
    active_map: &'a mut HashMap<String, String>,
    table_name: &'a str,
    parquet_path: &'a str,
    session_name: Option<String>,
    from_session_id: Option<String>,
    is_default: bool,
}

impl SessionManager {
    // **[2026-02-15]** 变更原因：插入行/列时需要查询列默认公式元数据。
    fn load_column_default_formulas(&self, table_name: &str) -> Vec<Option<String>> {
        // **[2026-02-15]** 变更目的：读取持久化元数据，失败时回退为空列表避免阻断编辑。
        let result = self
            .metadata_manager
            .store
            .get_table("datafusion", "public", table_name);
        if let Ok(Some(meta)) = result {
            if let Some(raw) = meta.column_default_formulas_json {
                if let Ok(list) = serde_json::from_str::<Vec<Option<String>>>(&raw) {
                    return list;
                }
            }
        }
        Vec::new()
    }

    // **[2026-02-15]** 变更原因：插入列需要同步更新列默认公式元数据。
    fn update_column_default_formulas(
        &self,
        table_name: &str,
        col_idx: usize,
        default_formula: Option<String>,
    ) -> Result<(), UpdateCellError> {
        // **[2026-02-15]** 变更目的：获取元数据并插入新列默认公式（允许为空）。
        let table = self
            .metadata_manager
            .store
            .get_table("datafusion", "public", table_name)
            .map_err(|e| UpdateCellError::Internal {
                reason: format!("Failed to read table metadata: {}", e),
            })?;

        if let Some(mut meta) = table {
            let mut formulas = if let Some(raw) = &meta.column_default_formulas_json {
                serde_json::from_str::<Vec<Option<String>>>(raw).unwrap_or_default()
            } else {
                Vec::new()
            };
            if col_idx <= formulas.len() {
                formulas.insert(col_idx, default_formula);
            } else {
                while formulas.len() < col_idx {
                    formulas.push(None);
                }
                formulas.push(default_formula);
            }
            meta.column_default_formulas_json =
                Some(serde_json::to_string(&formulas).unwrap_or_default());
            self.metadata_manager.store.save_table(&meta).map_err(|e| {
                UpdateCellError::Internal {
                    reason: format!("Failed to save table metadata: {}", e),
                }
            })?;
        }
        Ok(())
    }

    pub fn update_column_default_formula_at(
        &self,
        table_name: &str,
        col_idx: usize,
        default_formula: Option<String>,
    ) -> Result<(), UpdateCellError> {
        let table = self
            .metadata_manager
            .store
            .get_table("datafusion", "public", table_name)
            .map_err(|e| UpdateCellError::Internal {
                reason: format!("Failed to read table metadata: {}", e),
            })?;

        if let Some(mut meta) = table {
            let mut formulas = if let Some(raw) = &meta.column_default_formulas_json {
                serde_json::from_str::<Vec<Option<String>>>(raw).unwrap_or_default()
            } else {
                Vec::new()
            };
            if col_idx >= formulas.len() {
                while formulas.len() <= col_idx {
                    formulas.push(None);
                }
            }
            formulas[col_idx] = default_formula;
            meta.column_default_formulas_json =
                Some(serde_json::to_string(&formulas).unwrap_or_default());
            self.metadata_manager.store.save_table(&meta).map_err(|e| {
                UpdateCellError::Internal {
                    reason: format!("Failed to save table metadata: {}", e),
                }
            })?;
        }
        Ok(())
    }

    fn remove_column_default_formula_at(
        &self,
        table_name: &str,
        col_idx: usize,
    ) -> Result<(), UpdateCellError> {
        let table = self
            .metadata_manager
            .store
            .get_table("datafusion", "public", table_name)
            .map_err(|e| UpdateCellError::Internal {
                reason: format!("Failed to read table metadata: {}", e),
            })?;

        if let Some(mut meta) = table {
            let mut formulas = if let Some(raw) = &meta.column_default_formulas_json {
                serde_json::from_str::<Vec<Option<String>>>(raw).unwrap_or_default()
            } else {
                Vec::new()
            };
            if col_idx < formulas.len() {
                formulas.remove(col_idx);
            }
            meta.column_default_formulas_json =
                Some(serde_json::to_string(&formulas).unwrap_or_default());
            self.metadata_manager.store.save_table(&meta).map_err(|e| {
                UpdateCellError::Internal {
                    reason: format!("Failed to save table metadata: {}", e),
                }
            })?;
        }
        Ok(())
    }

    pub fn is_formula_column(&self, table_name: &str, col_idx: usize) -> bool {
        let formulas = self.load_column_default_formulas(table_name);
        if let Some(Some(raw)) = formulas.get(col_idx) {
            is_formula_marker(raw)
        } else {
            false
        }
    }
    /// 创建会话管理器并使用默认自动落盘配置。
    pub fn new(base_path: &str, metadata_manager: Arc<MetadataManager>) -> Self {
        Self::new_with_config(base_path, metadata_manager, AutoFlushConfig::default())
    }

    /// 创建会话管理器并指定自动落盘配置。
    pub fn new_with_config(
        base_path: &str,
        metadata_manager: Arc<MetadataManager>,
        auto_flush: AutoFlushConfig,
    ) -> Self {
        let path = PathBuf::from(base_path).join("sessions");
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
        }

        let sessions_file = path.join("sessions.json");
        let mut sessions_map = HashMap::new();

        // 1. Migration: Check if sessions.json exists
        if sessions_file.exists() {
            println!("[SessionManager] Found legacy sessions.json. Starting migration...");
            if let Ok(content) = std::fs::read_to_string(&sessions_file) {
                if let Ok(legacy_sessions) =
                    serde_json::from_str::<HashMap<String, SessionInfo>>(&content)
                {
                    for (id, info) in legacy_sessions {
                        // Insert into SQLite
                        let db_session = DbSession {
                            session_id: info.session_id.clone(),
                            table_name: info.table_name.clone(),
                            friendly_name: Some(info.name.clone()),
                            lance_path: info.lance_path.clone(),
                            created_at: info.created_at as i64,
                            is_default: info.is_default,
                            parent_session_id: None,
                            last_accessed_at: info.created_at as i64,
                        };

                        if let Err(e) = metadata_manager.store.create_session(&db_session) {
                            println!("[SessionManager] Failed to migrate session {}: {}", id, e);
                        } else {
                            // Migrate Attributes (Styles)
                            for (cell_key, style) in &info.metadata.styles {
                                if let Ok(val) = serde_json::to_string(style) {
                                    let attr = SheetAttribute {
                                        session_id: id.clone(),
                                        cell_key: cell_key.clone(),
                                        attr_type: "style".to_string(),
                                        attr_value: val,
                                    };
                                    let _ = metadata_manager.store.set_sheet_attribute(&attr);
                                }
                            }
                            // Migrate Merges
                            if !info.metadata.merges.is_empty() {
                                if let Ok(val) = serde_json::to_string(&info.metadata.merges) {
                                    let attr = SheetAttribute {
                                        session_id: id.clone(),
                                        cell_key: "SHEET".to_string(),
                                        attr_type: "merges".to_string(),
                                        attr_value: val,
                                    };
                                    let _ = metadata_manager.store.set_sheet_attribute(&attr);
                                }
                            }
                        }

                        // Keep in memory map for compatibility
                        sessions_map.insert(id, info);
                    }
                }
            }
            // Rename legacy file
            let backup_path = path.join("sessions.json.bak");
            if let Err(e) = std::fs::rename(&sessions_file, &backup_path) {
                println!("[SessionManager] Failed to rename sessions.json: {}", e);
            } else {
                println!("[SessionManager] Migration complete. sessions.json renamed to .bak");
            }
        }

        // 2. Load ALL sessions from SQLite (Source of Truth)
        sessions_map.clear();
        let mut active_map = HashMap::new();

        let now = now_millis();

        if let Ok(sessions) = metadata_manager.store.list_all_sessions() {
            for s in sessions {
                // Check if session data exists on disk
                if !PathBuf::from(&s.lance_path).exists() {
                    println!(
                        "[SessionManager] Warning: Session {} data not found at {}. Skipping.",
                        s.session_id, s.lance_path
                    );
                    continue;
                }

                // Load Attributes
                let attrs = metadata_manager
                    .store
                    .get_sheet_attributes(&s.session_id)
                    .unwrap_or_default();

                let mut metadata = SheetMetadata::default();
                for attr in attrs {
                    if attr.attr_type == "style" {
                        // Cell Key format: "row,col"
                        let parts: Vec<&str> = attr.cell_key.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(r), Ok(c)) =
                                (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                            {
                                if let Ok(style) =
                                    serde_json::from_str::<CellStyle>(&attr.attr_value)
                                {
                                    metadata.styles.insert(format!("{},{}", r, c), style);
                                }
                            }
                        }
                    } else if attr.attr_type == "merges" {
                        if let Ok(merges) =
                            serde_json::from_str::<Vec<MergeRange>>(&attr.attr_value)
                        {
                            metadata.merges = merges;
                        }
                    }
                }

                let info = SessionInfo {
                    session_id: s.session_id.clone(),
                    table_name: s.table_name.clone(),
                    name: s.friendly_name.unwrap_or_else(|| "Untitled".to_string()),
                    lance_path: s.lance_path.clone(),
                    created_at: s.created_at as u64,
                    is_default: s.is_default,
                    current_data: None,
                    metadata,
                    from_session_id: s.parent_session_id,
                    dirty: false,
                    pending_writes: 0,
                    last_modified_at: now,
                    last_persisted_at: now,
                    is_flushing: false,
                };

                sessions_map.insert(s.session_id.clone(), info);

                if s.is_default {
                    active_map.insert(s.table_name.clone(), s.session_id.clone());
                }
            }
        }

        println!(
            "[SessionManager] Loaded {} sessions from SQLite",
            sessions_map.len()
        );

        Self {
            base_path: path,
            metadata_manager,
            sessions: Mutex::new(sessions_map),
            active_table_sessions: Mutex::new(active_map),
            auto_flush,
        }
    }

    /// 启动后台自动落盘任务。
    pub fn start_auto_flush(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            manager.auto_flush_loop().await;
        });
    }

    // List Sessions for a Table
    /// 列出指定表的所有会话。
    pub async fn list_sessions(&self, table_name: &str) -> Vec<SessionInfo> {
        let sessions = self.sessions.lock().await;
        let mut result = Vec::new();
        for s in sessions.values() {
            if s.table_name == table_name {
                result.push(s.clone());
            }
        }
        // Sort by created_at desc
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result
    }


    // - **2026-03-14**: Expose active session id for sessions list API.
    // - **Reason**: Frontend needs to highlight the active sandbox tab deterministically.
    // - **Purpose**: Keep API response small and avoid leaking full session payloads.
    pub async fn get_active_session_id(&self, table_name: &str) -> Option<String> {
        let active = self.active_table_sessions.lock().await;
        active.get(table_name).cloned()
    }

    /// 删除指定表的所有会话与持久化数据。
    pub async fn delete_table(&self, table_name: &str) -> Result<usize, String> {
        let mut sessions = self.sessions.lock().await;
        let mut active_map = self.active_table_sessions.lock().await;

        active_map.remove(table_name);

        let session_ids: Vec<String> = sessions
            .values()
            .filter(|s| s.table_name == table_name)
            .map(|s| s.session_id.clone())
            .collect();

        let mut deleted = 0;
        for sid in session_ids {
            if let Some(info) = sessions.remove(&sid) {
                deleted += 1;
                let _ = self
                    .metadata_manager
                    .store
                    .delete_sheet_attributes_by_session(&sid);
                let _ = self.metadata_manager.store.delete_session(&sid);

                if let Ok(meta) = tokio::fs::metadata(&info.lance_path).await {
                    if meta.is_dir() {
                        let _ = tokio::fs::remove_dir_all(&info.lance_path).await;
                    } else {
                        let _ = tokio::fs::remove_file(&info.lance_path).await;
                    }
                }
            }
        }

        Ok(deleted)
    }

    // Create New Table (Atomic Transaction)
    /// 创建新表并生成默认会话。
    pub async fn create_new_table(&self, table_name: &str) -> Result<SessionInfo, String> {
        // 1. Define Paths
        let lance_uri = self.base_path.join(format!("{}.lance", table_name));
        let lance_uri_str = lance_uri.to_str().unwrap().to_string();

        // 2. Create Empty Lance Dataset (Schema: id (int), col1 (utf8))
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("col1", DataType::Utf8, true),
        ]));

        // Create initial batch with 1 row (id=1, col1="") to allow immediate editing
        let id_array = arrow::array::Int64Array::from(vec![1]);
        let col1_array = arrow::array::StringArray::from(vec![""]);
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(id_array), Arc::new(col1_array)],
        )
        .unwrap();

        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let write_params = WriteParams {
            mode: lance::dataset::WriteMode::Create,
            ..Default::default()
        };
        Dataset::write(reader, &lance_uri_str, Some(write_params))
            .await
            .map_err(|e| e.to_string())?;

        // 3. Prepare Metadata
        let session_id = Uuid::new_v4().to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let table_meta = TableMetadata {
            catalog_name: "datafusion".to_string(),
            schema_name: "public".to_string(),
            table_name: table_name.to_string(),
            file_path: lance_uri_str.clone(),
            source_type: "parquet".to_string(), // Lance acts as Parquet for now
            sheet_name: None,
            header_rows: None,
            header_mode: None,
            schema_json: None,
            stats_json: None,
            indexes_json: None,
            // **[2026-02-15]** 变更原因：新表默认无列默认公式。
            column_default_formulas_json: None,
        };

        let session_meta = DbSession {
            session_id: session_id.clone(),
            table_name: table_name.to_string(),
            friendly_name: Some("Initial".to_string()),
            lance_path: lance_uri_str.clone(),
            created_at: timestamp as i64,
            is_default: true,
            parent_session_id: None,
            last_accessed_at: timestamp as i64,
        };

        // 4. Execute Transaction
        self.metadata_manager
            .store
            .create_table_transaction(&table_meta, &session_meta)
            .map_err(|e| format!("Transaction failed: {}", e))?;

        // 5. Update Memory State
        let now_ms = now_millis();
        let info = SessionInfo {
            session_id: session_id.clone(),
            table_name: table_name.to_string(),
            name: "Initial".to_string(),
            lance_path: lance_uri_str,
            created_at: timestamp,
            is_default: true,
            from_session_id: None,
            current_data: None,
            metadata: SheetMetadata::default(),
            dirty: false,
            pending_writes: 0,
            last_modified_at: now_ms,
            last_persisted_at: now_ms,
            is_flushing: false,
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), info.clone());

        let mut active = self.active_table_sessions.lock().await;
        active.insert(table_name.to_string(), session_id.clone());

        println!(
            "[SessionManager] Created new table '{}' with session {}",
            table_name, session_id
        );

        Ok(info)
    }

    // Create New Session (Branch) - Public Wrapper
    /// 创建新会话并按需从指定来源派生。
    pub async fn create_session(
        &self,
        table_name: &str,
        parquet_path: &str,
        session_name: Option<String>,
        from_session_id: Option<String>,
        is_default: bool,
    ) -> Result<SessionInfo, String> {
        // Acquire locks in order to prevent deadlocks (sessions -> active)
        let mut sessions = self.sessions.lock().await;
        let mut active = self.active_table_sessions.lock().await;

        // **[2026-02-25]** 变更原因：create_session_internal 参数过多。
        // **[2026-02-25]** 变更目的：改为结构体参数以满足 clippy。
        // **[2026-02-25]** 变更说明：仅封装参数，不改变逻辑。
        // **[2026-02-25]** 变更说明：保持锁持有顺序不变。
        // **[2026-02-25]** 变更说明：调用点语义一致。
        // **[2026-02-25]** 变更说明：便于后续扩展。
        self.create_session_internal(CreateSessionParams {
            sessions: &mut sessions,
            active_map: &mut active,
            table_name,
            parquet_path,
            session_name,
            from_session_id,
            is_default,
        })
        .await
    }

    // Internal helper that assumes locks are already held
    /// 在持有锁的前提下创建会话并完成数据初始化。
    async fn create_session_internal(
        &self,
        params: CreateSessionParams<'_>,
    ) -> Result<SessionInfo, String> {
        // **[2026-02-25]** 变更原因：结构体参数需要解构以使用原变量名。
        // **[2026-02-25]** 变更目的：保持原有逻辑可读性。
        // **[2026-02-25]** 变更说明：字段与原参数一一对应。
        // **[2026-02-25]** 变更说明：不改变借用与生命周期。
        // **[2026-02-25]** 变更说明：仅替换签名与解构方式。
        // **[2026-02-25]** 变更说明：方便后续扩展字段。
        let CreateSessionParams {
            sessions,
            active_map,
            table_name,
            parquet_path,
            session_name,
            from_session_id,
            is_default,
        } = params;
        let session_id = Uuid::new_v4().to_string();
        let name = session_name.unwrap_or_else(|| {
            format!("Session {}", session_id.chars().take(8).collect::<String>())
        });

        let lance_path = self
            .base_path
            .join(format!("{}_{}", table_name, session_id));
        let lance_uri = lance_path.to_string_lossy().to_string();

        // Determine source path
        let mut source_uri = parquet_path.to_string();
        let mut is_lance_source = false;
        let mut in_memory_batches: Option<Vec<RecordBatch>> = None;

        if let Some(ref parent_id) = from_session_id {
            if let Some(parent_session) = sessions.get(parent_id) {
                // 1. Try to fork from Memory
                if let Some(data_lock) = &parent_session.current_data {
                    let data = data_lock.read().await;
                    if !data.is_empty() {
                        in_memory_batches = Some(data.clone());
                        println!("[SessionManager] Forking Session '{}' from IN-MEMORY data of '{}' ({})", name, parent_session.name, parent_id);
                    }
                }

                // 2. Fallback to Disk (Lance)
                if in_memory_batches.is_none() {
                    source_uri = parent_session.lance_path.clone();
                    is_lance_source = true;
                    println!(
                        "[SessionManager] Forking Session '{}' from DISK (Lance) of '{}' ({})",
                        name, parent_session.name, parent_id
                    );
                }
            } else {
                return Err(format!("Source session {} not found", parent_id));
            }
        } else {
            println!(
                "[SessionManager] Creating Session '{}' ({}) for table '{}' from source {}",
                name, session_id, table_name, lance_uri
            );
        }

        // 1. Hydrate / Copy Data
        let ctx = SessionContext::new();

        let df = if let Some(batches) = in_memory_batches {
            // Read from Memory
            if batches.is_empty() {
                // Should not happen due to check above, but safe guard
                return Err("Source session has empty data".to_string());
            }
            let schema = batches[0].schema();
            let provider = datafusion::datasource::MemTable::try_new(schema, vec![batches])
                .map_err(|e| e.to_string())?;
            ctx.read_table(Arc::new(provider))
                .map_err(|e| e.to_string())?
        } else if is_lance_source {
            // Read from existing Lance dataset
            let ds = Dataset::open(&source_uri)
                .await
                .map_err(|e| e.to_string())?;
            let scanner = ds.scan();
            let batches = scanner
                .try_into_stream()
                .await
                .map_err(|e| e.to_string())?
                .try_collect::<Vec<RecordBatch>>()
                .await
                .map_err(|e| e.to_string())?;

            let schema = ds.schema().into();
            let provider =
                datafusion::datasource::MemTable::try_new(Arc::new(schema), vec![batches])
                    .map_err(|e| e.to_string())?;
            ctx.read_table(Arc::new(provider))
                .map_err(|e| e.to_string())?
        } else {
            // Read from Parquet or CSV
            if source_uri.ends_with(".csv") {
                ctx.read_csv(&source_uri, CsvReadOptions::default())
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                ctx.read_parquet(&source_uri, ParquetReadOptions::default())
                    .await
                    .map_err(|e| e.to_string())?
            }
        };

        // Lance 1.0.4 doesn't support Utf8View yet, so we must cast to Utf8
        let mut exprs = Vec::new();
        for field in df.schema().fields() {
            let col_expr = col(field.name());
            if matches!(field.data_type(), arrow::datatypes::DataType::Utf8View) {
                exprs.push(
                    datafusion::logical_expr::Expr::Cast(datafusion::logical_expr::Cast {
                        expr: Box::new(col_expr),
                        data_type: arrow::datatypes::DataType::Utf8,
                    })
                    .alias(field.name()),
                );
            } else {
                exprs.push(col_expr);
            }
        }
        let df = df.select(exprs).map_err(|e| e.to_string())?;

        let schema = Arc::new(df.schema().as_arrow().clone());
        let batches = df.collect().await.map_err(|e| e.to_string())?;

        // Clone batches for in-memory state
        let current_data = Arc::new(RwLock::new(batches.clone()));
        // Initialize empty metadata
        let metadata = SheetMetadata::default();

        // Write initial data to Lance (Disk Persistence for recovery/initial state)
        let write_params = WriteParams::default();
        let reader = RecordBatchIterator::new(batches.into_iter().map(Ok), schema);

        Dataset::write(reader, lance_uri.as_str(), Some(write_params))
            .await
            .map_err(|e| e.to_string())?;

        // 2. Register Session
        let now_ms = now_millis();
        let info = SessionInfo {
            session_id: session_id.clone(),
            table_name: table_name.to_string(),
            name: name.clone(),
            lance_path: lance_uri.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_default,
            current_data: Some(current_data),
            metadata,
            from_session_id: from_session_id.clone(),
            dirty: false,
            pending_writes: 0,
            last_modified_at: now_ms,
            last_persisted_at: now_ms,
            is_flushing: false,
        };

        sessions.insert(session_id.clone(), info.clone());

        // Persist to SQLite
        let db_session = DbSession {
            session_id: session_id.clone(),
            table_name: table_name.to_string(),
            friendly_name: Some(name),
            lance_path: lance_uri,
            created_at: info.created_at as i64,
            is_default,
            parent_session_id: from_session_id.clone(),
            last_accessed_at: info.created_at as i64,
        };
        if let Err(e) = self.metadata_manager.store.create_session(&db_session) {
            println!(
                "[SessionManager] Failed to persist session to SQLite: {}",
                e
            );
        }

        // Auto-activate
        active_map.insert(table_name.to_string(), session_id.clone());
        println!(
            "[SessionManager] Activated session {} for table {}. Active map size: {}",
            session_id,
            table_name,
            active_map.len()
        );

        Ok(info)
    }

    // Switch Active Session
    /// 切换指定表的活动会话。
    pub async fn switch_session(&self, table_name: &str, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(session_id) {
            // Load data into memory if missing
            if session.current_data.is_none() {
                println!(
                    "[SessionManager] Loading data for session {} from disk...",
                    session_id
                );
                let uri = session.lance_path.clone();

                // Load from Lance
                match Dataset::open(&uri).await {
                    Ok(ds) => {
                        let scanner = ds.scan();
                        match scanner.try_into_stream().await {
                            Ok(stream) => {
                                match stream.try_collect::<Vec<RecordBatch>>().await {
                                    Ok(batches) => {
                                        session.current_data = Some(Arc::new(RwLock::new(batches)));
                                        // Metadata is already loaded from sessions.json when SessionManager starts
                                        // or when create_session is called.
                                        // If we wanted to load per-session separate metadata file, we would do it here.
                                        if session.metadata.styles.is_empty() {
                                            // Placeholder for future lazy loading if needed
                                        }
                                        println!(
                                            "[SessionManager] Data loaded for session {}",
                                            session_id
                                        );
                                    }
                                    Err(e) => println!(
                                        "[SessionManager] Failed to collect batches: {}",
                                        e
                                    ),
                                }
                            }
                            Err(e) => println!("[SessionManager] Failed to scan: {}", e),
                        }
                    }
                    Err(e) => println!("[SessionManager] Failed to open dataset: {}", e),
                }
            }
        } else {
            return Err("Session not found".to_string());
        }

        let mut active = self.active_table_sessions.lock().await;
        active.insert(table_name.to_string(), session_id.to_string());
        println!(
            "[SessionManager] Switched table '{}' to session '{}'",
            table_name, session_id
        );
        Ok(())
    }

    // Reset to Default Session (Oldest one)
    /// 重置为最早创建的默认会话。
    pub async fn reset_to_default_session(&self, table_name: &str) -> Result<String, String> {
        let sessions = self.sessions.lock().await;
        let mut table_sessions: Vec<&SessionInfo> = sessions
            .values()
            .filter(|s| s.table_name == table_name)
            .collect();

        // Sort by created_at ASC (Oldest first)
        table_sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        if let Some(default_session) = table_sessions.first() {
            let id = default_session.session_id.clone();
            drop(sessions); // Release lock

            self.switch_session(table_name, &id).await?;
            println!(
                "[SessionManager] Reset table '{}' to default session '{}'",
                table_name, id
            );
            Ok(id)
        } else {
            Err("No sessions found for table".to_string())
        }
    }

    /// 更新单元格样式并持久化。
    pub async fn update_style(
        &self,
        table_name: &str,
        row: u32,
        col: u32,
        style: CellStyle,
    ) -> Result<String, String> {
        let session_id = {
            let active = self.active_table_sessions.lock().await;
            active.get(table_name).cloned().ok_or("No active session")?
        };

        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or("Session not found")?;

        let key = format!("{},{}", row, col);

        // Merge with existing style
        let mut new_style = session
            .metadata
            .styles
            .get(&key)
            .cloned()
            .unwrap_or_default();
        if let Some(v) = style.bold {
            new_style.bold = Some(v);
        }
        if let Some(v) = style.italic {
            new_style.italic = Some(v);
        }
        if let Some(v) = style.underline {
            new_style.underline = Some(v);
        }
        if let Some(v) = style.align {
            new_style.align = Some(v);
        }
        if let Some(v) = style.color {
            new_style.color = Some(v);
        }
        if let Some(v) = style.bg_color {
            new_style.bg_color = Some(v);
        }
        // **[2026-02-16]** 变更原因：补齐 format 字段合并。
        // **[2026-02-16]** 变更目的：保证样式更新覆盖格式配置。
        if let Some(v) = style.format {
            new_style.format = Some(v);
        }

        session
            .metadata
            .styles
            .insert(key.clone(), new_style.clone());

        // Persist
        // self.persist_sessions(&sessions);
        if let Ok(val) = serde_json::to_string(&new_style) {
            let attr = SheetAttribute {
                session_id: session_id.clone(),
                cell_key: key,
                attr_type: "style".to_string(),
                attr_value: val,
            };
            if let Err(e) = self.metadata_manager.store.set_sheet_attribute(&attr) {
                println!("[SessionManager] Failed to persist style: {}", e);
            }
        }

        Ok("Style updated".to_string())
    }

    /// 批量更新范围内的单元格样式。
    pub async fn update_style_range(
        &self,
        table_name: &str,
        range: MergeRange,
        style: CellStyle,
    ) -> Result<String, String> {
        let session_id = {
            let active = self.active_table_sessions.lock().await;
            active.get(table_name).cloned().ok_or("No active session")?
        };

        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or("Session not found")?;

        for r in range.start_row..=range.end_row {
            for c in range.start_col..=range.end_col {
                let key = format!("{},{}", r, c);
                let mut new_style = session
                    .metadata
                    .styles
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();

                if let Some(v) = style.bold {
                    new_style.bold = Some(v);
                }
                if let Some(v) = style.italic {
                    new_style.italic = Some(v);
                }
                if let Some(v) = style.underline {
                    new_style.underline = Some(v);
                }
                if let Some(ref v) = style.align {
                    new_style.align = Some(v.clone());
                }
                if let Some(ref v) = style.color {
                    new_style.color = Some(v.clone());
                }
                if let Some(ref v) = style.bg_color {
                    new_style.bg_color = Some(v.clone());
                }
                // **[2026-02-16]** 变更原因：补齐 format 字段合并。
                // **[2026-02-16]** 变更目的：范围更新保持格式一致。
                if let Some(ref v) = style.format {
                    new_style.format = Some(v.clone());
                }

                session
                    .metadata
                    .styles
                    .insert(key.clone(), new_style.clone());

                // Persist to SQLite
                if let Ok(val) = serde_json::to_string(&new_style) {
                    let attr = SheetAttribute {
                        session_id: session_id.clone(),
                        cell_key: key,
                        attr_type: "style".to_string(),
                        attr_value: val,
                    };
                    if let Err(e) = self.metadata_manager.store.set_sheet_attribute(&attr) {
                        println!("[SessionManager] Failed to persist style range: {}", e);
                    }
                }
            }
        }

        Ok("Style range updated".to_string())
    }

    /// 更新合并范围并持久化合并列表。
    pub async fn update_merge(
        &self,
        table_name: &str,
        range: MergeRange,
    ) -> Result<String, String> {
        let session_id = {
            let active = self.active_table_sessions.lock().await;
            active.get(table_name).cloned().ok_or("No active session")?
        };

        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or("Session not found")?;

        // Check for exact match to TOGGLE (Unmerge)
        if let Some(idx) = session.metadata.merges.iter().position(|m| {
            m.start_row == range.start_row
                && m.start_col == range.start_col
                && m.end_row == range.end_row
                && m.end_col == range.end_col
        }) {
            session.metadata.merges.remove(idx);
            // Persist full merges list to SQLite
            if let Ok(val) = serde_json::to_string(&session.metadata.merges) {
                let attr = SheetAttribute {
                    session_id: session_id.clone(),
                    cell_key: "SHEET".to_string(),
                    attr_type: "merges".to_string(),
                    attr_value: val,
                };
                let _ = self.metadata_manager.store.set_sheet_attribute(&attr);
            }
            return Ok("Unmerged".to_string());
        }

        // Remove overlapping merges
        session.metadata.merges.retain(|m| {
            // Keep if NO overlap
            // Overlap logic: !(r1.end < r2.start || r1.start > r2.end || c1.end < c2.start || c1.start > c2.end)
            let overlap = !(m.end_row < range.start_row
                || m.start_row > range.end_row
                || m.end_col < range.start_col
                || m.start_col > range.end_col);
            !overlap
        });

        // Add new merge
        session.metadata.merges.push(range);

        // Persist full merges list to SQLite
        if let Ok(val) = serde_json::to_string(&session.metadata.merges) {
            let attr = SheetAttribute {
                session_id: session_id.clone(),
                cell_key: "SHEET".to_string(),
                attr_type: "merges".to_string(),
                attr_value: val,
            };
            let _ = self.metadata_manager.store.set_sheet_attribute(&attr);
        }

        Ok("Merged".to_string())
    }

    /// 获取当前活动会话的元数据。
    pub async fn get_metadata(&self, table_name: &str) -> Result<SheetMetadata, String> {
        let session_id = {
            let active = self.active_table_sessions.lock().await;
            active.get(table_name).cloned().ok_or("No active session")?
        };

        let sessions = self.sessions.lock().await;
        let session = sessions.get(&session_id).ok_or("Session not found")?;

        Ok(session.metadata.clone())
    }

    // Get Active Lance URI
    /// 获取指定表当前活动会话的 Lance 路径。
    pub async fn get_active_session_uri(&self, table_name: &str) -> Option<String> {
        let (sess_id, keys) = {
            let active = self.active_table_sessions.lock().await;
            (
                active.get(table_name).cloned(),
                active.keys().cloned().collect::<Vec<_>>(),
            )
        };

        if let Some(sess_id) = sess_id {
            let sessions = self.sessions.lock().await;
            if let Some(info) = sessions.get(&sess_id) {
                return Some(info.lance_path.clone());
            } else {
                println!("[SessionManager] Warning: Active session {} for table {} not found in session list", sess_id, table_name);
            }
        } else {
            println!(
                "[SessionManager] No active session for table {}. Available: {:?}",
                table_name, keys
            );
        }
        None
    }

    // Get Dataset Versions (Time Machine)
    /// 获取当前活动会话的数据版本列表。
    pub async fn get_versions(&self, table_name: &str) -> Result<Vec<serde_json::Value>, String> {
        let uri = self
            .get_active_session_uri(table_name)
            .await
            .ok_or("No active session found")?;

        let ds = Dataset::open(&uri).await.map_err(|e| e.to_string())?;
        let versions = ds.versions().await.map_err(|e| e.to_string())?;

        let mut result = Vec::new();
        for v in versions {
            result.push(serde_json::json!({
                "version": v.version,
                "timestamp": v.timestamp.timestamp(), // Unix timestamp
                "metadata": v.metadata
            }));
        }

        // Sort by version desc
        result.sort_by(|a, b| b["version"].as_u64().cmp(&a["version"].as_u64()));

        Ok(result)
    }

    // Checkout/Restore Version
    // This loads the specific version's data into memory.
    // Any subsequent write will create a NEW version (effectively a restore).
    /// 切换到指定版本并加载到内存。
    pub async fn checkout_version(&self, table_name: &str, version: u64) -> Result<(), String> {
        let session_id = {
            let active = self.active_table_sessions.lock().await;
            active.get(table_name).cloned().ok_or("No active session")?
        };

        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or("Session not found")?;
        let uri = session.lance_path.clone();

        println!(
            "[SessionManager] Checking out version {} for table '{}' (Session {})",
            version, table_name, session_id
        );

        // Open Dataset at specific version
        let ds = Dataset::open(&uri).await.map_err(|e| e.to_string())?;
        let ds = ds
            .checkout_version(version)
            .await
            .map_err(|e| e.to_string())?;

        // Load data into memory
        let scanner = ds.scan();
        let batches = scanner
            .try_into_stream()
            .await
            .map_err(|e| e.to_string())?
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|e| e.to_string())?;

        session.current_data = Some(Arc::new(RwLock::new(batches)));

        println!("[SessionManager] Version {} loaded into memory.", version);
        Ok(())
    }

    // Hydrate (Legacy/Compat): Ensures at least one session exists and activates it
    // Returns the path to the Lance dataset
    /// 确保指定表存在可用会话并返回 Lance 路径。
    pub async fn hydrate(&self, table_name: &str, parquet_path: &str) -> Result<String, String> {
        // Check if there is an active session
        if let Some(uri) = self.get_active_session_uri(table_name).await {
            // Ensure data is loaded even if "active" (e.g. after restart if we persisted active map?)
            // Actually active map is not persisted. So we are good.
            if std::path::Path::new(&uri).exists() {
                return Ok(uri);
            }
        }

        // Check if ANY session exists for this table (maybe persisted but not active in memory map yet)
        // If persisted sessions exist, pick the latest one
        {
            let sessions = self.sessions.lock().await;
            let mut table_sessions: Vec<&SessionInfo> = sessions
                .values()
                .filter(|s| s.table_name == table_name)
                .collect();
            table_sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            if let Some(latest) = table_sessions.first() {
                let uri = latest.lance_path.clone();
                let session_id = latest.session_id.clone();
                if std::path::Path::new(&uri).exists() {
                    drop(sessions); // Release lock before switching
                    let _ = self.switch_session(table_name, &session_id).await;
                    return Ok(uri);
                }
            }
        }

        // Create Default Session
        let info = self
            .create_session(
                table_name,
                parquet_path,
                Some("Default".to_string()),
                None,
                true,
            )
            .await?;
        Ok(info.lance_path)
    }

    // Update Cell: Modify a single value in the In-Memory Session
    // Returns (LanceURI, SessionID)
    // OPTIMIZED: Uses coarse-grained locking to prevent race conditions (Forking) and Vectorized Splicing for performance.
    /// 更新单元格内容，必要时自动分叉默认会话。
    pub async fn update_cell(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        row_idx: usize,
        col_name: &str,
        new_value: &str,
    ) -> Result<(String, String), UpdateCellError> {
        // 1. Lock Everything (Coarse-grained lock for Atomicity of Check-Fork-Switch)
        let mut sessions = self.sessions.lock().await;
        let mut active_map = self.active_table_sessions.lock().await;

        // 2. Identify Target Session
        let target_session_id = if let Some(id) = session_id {
            if sessions.contains_key(id) {
                id.to_string()
            } else {
                return Err(UpdateCellError::SessionNotFound {
                    session_id: id.to_string(),
                });
            }
        } else {
            // No session provided, find active one
            if let Some(id) = active_map.get(table_name) {
                id.to_string()
            } else {
                return Err(UpdateCellError::NoActiveSession {
                    table_name: table_name.to_string(),
                });
            }
        };

        // 3. Check for Forking (Read-only/Default session)
        // We hold the locks, so this is atomic.
        let session_info = sessions.get(&target_session_id).unwrap();
        let mut final_session_id = target_session_id.clone();
        let mut lance_uri = session_info.lance_path.clone();

        if session_info.is_default {
            println!("[SessionManager] Default/Readonly session detected. Auto-forking...");
            // Create new session from the current one (Internal call)
            let new_session = self
                // **[2026-02-25]** 变更原因：create_session_internal 改为结构体参数。
                // **[2026-02-25]** 变更目的：消除参数过多 lint。
                // **[2026-02-25]** 变更说明：封装参数不改变语义。
                // **[2026-02-25]** 变更说明：保持分支与错误处理一致。
                // **[2026-02-25]** 变更说明：不影响会话派生逻辑。
                // **[2026-02-25]** 变更说明：便于扩展字段。
                .create_session_internal(CreateSessionParams {
                    sessions: &mut sessions,
                    active_map: &mut active_map,
                    table_name,
                    parquet_path: &lance_uri,
                    session_name: None,
                    from_session_id: Some(final_session_id.clone()),
                    is_default: false,
                })
                .await
                .map_err(|e| UpdateCellError::Internal { reason: e })?;

            final_session_id = new_session.session_id;
            lance_uri = new_session.lance_path;
            // Active map is already updated by create_session_internal
        }

        println!(
            "[SessionManager] Updating '{}' (Session: {}) Row {} Col '{}' -> '{}'",
            table_name, final_session_id, row_idx, col_name, new_value
        );

        // 4. Modify In-Memory Data
        let needs_load = {
            let session =
                sessions
                    .get(&final_session_id)
                    .ok_or(UpdateCellError::SessionNotFound {
                        session_id: final_session_id.clone(),
                    })?;
            session.current_data.is_none()
        };
        if needs_load {
            let uri = {
                let session =
                    sessions
                        .get(&final_session_id)
                        .ok_or(UpdateCellError::SessionNotFound {
                            session_id: final_session_id.clone(),
                        })?;
                session.lance_path.clone()
            };
            let batches = Self::load_batches_from_lance(&uri).await.map_err(|e| {
                UpdateCellError::LoadLanceFailed {
                    source: uri.clone(),
                    reason: e,
                }
            })?;
            let session =
                sessions
                    .get_mut(&final_session_id)
                    .ok_or(UpdateCellError::SessionNotFound {
                        session_id: final_session_id.clone(),
                    })?;
            session.current_data = Some(Arc::new(RwLock::new(batches)));
        }

        let session =
            sessions
                .get_mut(&final_session_id)
                .ok_or(UpdateCellError::SessionNotFound {
                    session_id: final_session_id.clone(),
                })?;

        let current_data_lock = session
            .current_data
            .as_ref()
            .ok_or(UpdateCellError::SessionDataNotLoaded)?;
        let mut batches = current_data_lock.write().await;

        // 2.1 Handle New Column (Schema Evolution)
        let mut schema = if !batches.is_empty() {
            batches[0].schema()
        } else {
            return Err(UpdateCellError::EmptyDataset);
        };

        if schema.field_with_name(col_name).is_err() {
            println!(
                "[SessionManager] Column '{}' not found. Adding new column...",
                col_name
            );
            let new_field = Field::new(col_name, DataType::Utf8, true);
            let mut new_fields = schema.fields().to_vec();
            new_fields.push(Arc::new(new_field));
            schema = Arc::new(Schema::new(new_fields));

            // Add column to all existing batches
            let mut upgraded_batches = Vec::new();
            for batch in batches.iter() {
                let num_rows = batch.num_rows();
                let new_col = arrow::array::new_null_array(&DataType::Utf8, num_rows);
                let mut columns = batch.columns().to_vec();
                columns.push(new_col);
                let new_batch = RecordBatch::try_new(schema.clone(), columns).map_err(|e| {
                    UpdateCellError::Internal {
                        reason: e.to_string(),
                    }
                })?;
                upgraded_batches.push(new_batch);
            }
            *batches = upgraded_batches;
        }

        // 2.1.5 Check for Type Promotion (Int/Float -> Utf8)
        let col_idx = schema
            .index_of(col_name)
            .map_err(|_| UpdateCellError::ColumnNotFound {
                column: col_name.to_string(),
            })?;
        let field = schema.field(col_idx);
        let mut promote_to_utf8 = false;

        match field.data_type() {
            DataType::Int64 => {
                if new_value.parse::<i64>().is_err() && new_value.parse::<f64>().is_err() {
                    promote_to_utf8 = true;
                }
            }
            DataType::Float64 => {
                if new_value.parse::<f64>().is_err() {
                    promote_to_utf8 = true;
                }
            }
            DataType::Boolean => {
                let val_lower = new_value.to_lowercase();
                let is_bool = val_lower == "true"
                    || val_lower == "false"
                    || val_lower == "1"
                    || val_lower == "0"
                    || val_lower == "yes"
                    || val_lower == "no";
                if !is_bool {
                    promote_to_utf8 = true;
                }
            }
            _ => {}
        }

        if promote_to_utf8 {
            println!(
                "[SessionManager] Promoting column '{}' from {:?} to Utf8",
                col_name,
                field.data_type()
            );

            let mut new_fields = schema.fields().to_vec();
            new_fields[col_idx] = Arc::new(Field::new(col_name, DataType::Utf8, true));
            let new_schema = Arc::new(Schema::new(new_fields));

            let mut upgraded_batches = Vec::new();
            for batch in batches.iter() {
                let mut new_columns = batch.columns().to_vec();
                let old_col = &new_columns[col_idx];
                let new_col = arrow::compute::cast(old_col, &DataType::Utf8).map_err(|e| {
                    UpdateCellError::CastFailed {
                        reason: e.to_string(),
                    }
                })?;
                new_columns[col_idx] = new_col;
                let new_batch =
                    RecordBatch::try_new(new_schema.clone(), new_columns).map_err(|e| {
                        UpdateCellError::Internal {
                            reason: e.to_string(),
                        }
                    })?;
                upgraded_batches.push(new_batch);
            }
            *batches = upgraded_batches;
            schema = new_schema;
        }

        // 2.2 Modify Data (Optimized Splicing)
        let mut new_batches = Vec::new();
        let mut current_row = 0;
        let mut row_updated = false;

        for batch in batches.iter() {
            let num_rows = batch.num_rows();

            if row_idx >= current_row && row_idx < current_row + num_rows {
                let local_idx = row_idx - current_row;

                let mut new_columns = batch.columns().to_vec();
                let old_col = &new_columns[col_idx];

                // Create single value array for the new value
                let new_val_array: ArrayRef = match old_col.data_type() {
                    DataType::Utf8 => Arc::new(StringArray::from(vec![new_value])),
                    DataType::LargeUtf8 => {
                        Arc::new(arrow::array::LargeStringArray::from(vec![new_value]))
                    }
                    DataType::Int64 => {
                        let val = new_value
                            .parse::<i64>()
                            .or_else(|_| new_value.parse::<f64>().map(|f| f as i64))
                            .unwrap_or(0);
                        Arc::new(Int64Array::from(vec![val]))
                    }
                    DataType::Float64 => {
                        let val = new_value.parse::<f64>().unwrap_or(0.0);
                        Arc::new(Float64Array::from(vec![val]))
                    }
                    DataType::Boolean => {
                        let val_lower = new_value.to_lowercase();
                        let val = val_lower == "true" || val_lower == "1" || val_lower == "yes";
                        Arc::new(BooleanArray::from(vec![val]))
                    }
                    dt => {
                        return Err(UpdateCellError::UnsupportedType {
                            data_type: format!("{:?}", dt),
                        })
                    }
                };

                // SPLICE: [0..local_idx] + [new_val] + [local_idx+1..]
                let pre_slice = old_col.slice(0, local_idx);
                let post_slice = old_col.slice(local_idx + 1, num_rows - local_idx - 1);

                let new_col = arrow::compute::concat(&[&pre_slice, &new_val_array, &post_slice])
                    .map_err(|e| UpdateCellError::Internal {
                        reason: e.to_string(),
                    })?;
                new_columns[col_idx] = new_col;

                let new_batch = RecordBatch::try_new(schema.clone(), new_columns).map_err(|e| {
                    UpdateCellError::Internal {
                        reason: e.to_string(),
                    }
                })?;
                new_batches.push(new_batch);
                row_updated = true;
            } else {
                new_batches.push(batch.clone());
            }
            current_row += num_rows;
        }

        // 3. Handle Append (New Row)
        if !row_updated && row_idx >= current_row {
            let rows_to_add = row_idx - current_row + 1;
            println!("[SessionManager] Appending {} new rows", rows_to_add);

            let mut new_columns: Vec<Arc<dyn Array>> = Vec::new();

            for field in schema.fields() {
                if field.name() == col_name {
                    // Create array: [null, null, ..., new_value]
                    let builder_col: ArrayRef = match field.data_type() {
                        DataType::Utf8 => {
                            let mut b = arrow::array::StringBuilder::with_capacity(
                                rows_to_add,
                                rows_to_add * 10,
                            );
                            for _ in 0..(rows_to_add - 1) {
                                b.append_null();
                            }
                            b.append_value(new_value);
                            Arc::new(b.finish())
                        }
                        DataType::Int64 => {
                            let mut b = arrow::array::PrimitiveBuilder::<Int64Type>::with_capacity(
                                rows_to_add,
                            );
                            for _ in 0..(rows_to_add - 1) {
                                b.append_null();
                            }
                            b.append_value(new_value.parse::<i64>().unwrap_or(0));
                            Arc::new(b.finish())
                        }
                        DataType::Float64 => {
                            let mut b =
                                arrow::array::PrimitiveBuilder::<Float64Type>::with_capacity(
                                    rows_to_add,
                                );
                            for _ in 0..(rows_to_add - 1) {
                                b.append_null();
                            }
                            b.append_value(new_value.parse::<f64>().unwrap_or(0.0));
                            Arc::new(b.finish())
                        }
                        DataType::Boolean => {
                            let mut b = arrow::array::BooleanBuilder::with_capacity(rows_to_add);
                            for _ in 0..(rows_to_add - 1) {
                                b.append_null();
                            }
                            let val = new_value.to_lowercase() == "true";
                            b.append_value(val);
                            Arc::new(b.finish())
                        }
                        _ => arrow::array::new_null_array(field.data_type(), rows_to_add),
                    };
                    new_columns.push(builder_col);
                } else {
                    new_columns.push(arrow::array::new_null_array(field.data_type(), rows_to_add));
                }
            }

            let new_batch = RecordBatch::try_new(schema.clone(), new_columns).map_err(|e| {
                UpdateCellError::Internal {
                    reason: e.to_string(),
                }
            })?;
            new_batches.push(new_batch);
        }

        *batches = new_batches;

        let now = now_millis();
        session.dirty = true;
        session.pending_writes += 1;
        session.last_modified_at = now;

        Ok((lance_uri, final_session_id))
    }

    // --- Row/Column Operations ---

    /// 在指定位置插入一行。
    pub async fn insert_row(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        row_idx: usize,
    ) -> Result<(String, String), UpdateCellError> {
        // **[2026-02-15]** 变更原因：插入行需要应用列默认公式。
        let column_default_formulas = self.load_column_default_formulas(table_name);

        self.modify_session(table_name, session_id, |batches, _schema| {
            let mut new_batches = Vec::new();
            let mut current_row = 0;
            let mut inserted = false;

            // Handle case where we insert at the very end (append)
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();

            if row_idx >= total_rows {
                // **[2026-02-15]** 变更目的：末尾插入行时按默认公式填充。
                let schema = batches[0].schema();
                let columns = build_row_with_defaults(schema.as_ref(), &column_default_formulas);
                let new_batch = RecordBatch::try_new(schema.clone(), columns).map_err(|e| {
                    UpdateCellError::Internal {
                        reason: e.to_string(),
                    }
                })?;

                new_batches = batches.clone();
                new_batches.push(new_batch);
                return Ok(new_batches);
            }

            for batch in batches.iter() {
                let num_rows = batch.num_rows();

                if !inserted && row_idx >= current_row && row_idx < current_row + num_rows {
                    let local_idx = row_idx - current_row;

                    // Split batch
                    let pre_slice = batch.slice(0, local_idx);
                    let post_slice = batch.slice(local_idx, num_rows - local_idx);

                    // **[2026-02-15]** 变更目的：插入中间行时按默认公式填充。
                    let schema = batch.schema();
                    let columns =
                        build_row_with_defaults(schema.as_ref(), &column_default_formulas);
                    let null_batch =
                        RecordBatch::try_new(schema.clone(), columns).map_err(|e| {
                            UpdateCellError::Internal {
                                reason: e.to_string(),
                            }
                        })?;

                    // If pre_slice is empty, just null + post
                    if pre_slice.num_rows() > 0 {
                        new_batches.push(pre_slice);
                    }
                    new_batches.push(null_batch);
                    if post_slice.num_rows() > 0 {
                        new_batches.push(post_slice);
                    }

                    inserted = true;
                } else {
                    new_batches.push(batch.clone());
                }
                current_row += num_rows;
            }
            Ok(new_batches)
        })
        .await
    }

    /// 删除指定行。
    pub async fn delete_row(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        row_idx: usize,
    ) -> Result<(String, String), UpdateCellError> {
        self.modify_session(table_name, session_id, |batches, _schema| {
            let mut new_batches = Vec::new();
            let mut current_row = 0;
            let mut deleted = false;

            for batch in batches.iter() {
                let num_rows = batch.num_rows();

                if !deleted && row_idx >= current_row && row_idx < current_row + num_rows {
                    let local_idx = row_idx - current_row;

                    // Split batch and skip the row
                    // [0..local_idx] + [local_idx+1..end]

                    if local_idx > 0 {
                        let pre_slice = batch.slice(0, local_idx);
                        new_batches.push(pre_slice);
                    }

                    if local_idx + 1 < num_rows {
                        let post_slice = batch.slice(local_idx + 1, num_rows - local_idx - 1);
                        new_batches.push(post_slice);
                    }

                    deleted = true;
                } else {
                    new_batches.push(batch.clone());
                }
                current_row += num_rows;
            }
            Ok(new_batches)
        })
        .await
    }

    /// 在指定位置插入列并调整数据。
    pub async fn insert_column(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        col_idx: usize,
        col_name: &str,
        data_type: Option<DataType>,
        default_formula: Option<String>,
    ) -> Result<(String, String), UpdateCellError> {
        // **[2026-02-15]** 变更原因：插入列需要写入列默认公式元数据与填充数据。
        let normalized_default_formula = default_formula
            .as_ref()
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty());

        // **[2026-02-15]** 变更目的：更新元数据中的列默认公式列表。
        self.update_column_default_formulas(
            table_name,
            col_idx,
            normalized_default_formula.clone(),
        )?;

        self.modify_session(table_name, session_id, |batches, schema| {
            // **[2026-02-15]** 变更原因：默认公式列必须使用 Utf8 类型承载公式字符串。
            let mut new_fields = schema.fields().to_vec();
            if col_idx > new_fields.len() {
                return Err(UpdateCellError::Internal {
                    reason: "Column index out of bounds".to_string(),
                });
            }
            let dt = if normalized_default_formula.is_some() {
                DataType::Utf8
            } else {
                data_type.clone().unwrap_or(DataType::Utf8)
            };
            let new_field = Arc::new(Field::new(col_name, dt.clone(), true));
            new_fields.insert(col_idx, new_field);
            let new_schema = Arc::new(Schema::new(new_fields));

            // **[2026-02-15]** 变更目的：根据默认公式填充新列值，否则保持空值。
            let mut new_batches = Vec::new();
            let is_formula_column = normalized_default_formula
                .as_ref()
                .map(|formula| is_formula_marker(formula))
                .unwrap_or(false);
            for batch in batches.iter() {
                let num_rows = batch.num_rows();
                let mut new_columns = batch.columns().to_vec();
                let new_col: ArrayRef = if let Some(formula) = &normalized_default_formula {
                    if is_formula_column {
                        arrow::array::new_null_array(&dt, num_rows)
                    } else {
                        let values = vec![formula.clone(); num_rows];
                        Arc::new(StringArray::from(values))
                    }
                } else {
                    arrow::array::new_null_array(&dt, num_rows)
                };
                new_columns.insert(col_idx, new_col);

                let new_batch =
                    RecordBatch::try_new(new_schema.clone(), new_columns).map_err(|e| {
                        UpdateCellError::Internal {
                            reason: e.to_string(),
                        }
                    })?;
                new_batches.push(new_batch);
            }

            Ok(new_batches)
        })
        .await
    }

    /// 删除指定列并调整数据。
    pub async fn delete_column(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        col_idx: usize,
    ) -> Result<(String, String), UpdateCellError> {
        let result = self
            .modify_session(table_name, session_id, |batches, schema| {
                // Update Schema
                let mut new_fields = schema.fields().to_vec();
                if col_idx >= new_fields.len() {
                    return Err(UpdateCellError::Internal {
                        reason: "Column index out of bounds".to_string(),
                    });
                }
                new_fields.remove(col_idx);
                let new_schema = Arc::new(Schema::new(new_fields));

                // Update Batches
                let mut new_batches = Vec::new();
                for batch in batches.iter() {
                    let mut new_columns = batch.columns().to_vec();
                    if col_idx < new_columns.len() {
                        new_columns.remove(col_idx);
                    }

                    let new_batch =
                        RecordBatch::try_new(new_schema.clone(), new_columns).map_err(|e| {
                            UpdateCellError::Internal {
                                reason: e.to_string(),
                            }
                        })?;
                    new_batches.push(new_batch);
                }

                Ok(new_batches)
            })
            .await;

        if result.is_ok() {
            self.remove_column_default_formula_at(table_name, col_idx)?;
        }

        result
    }

    // Helper for common session modification pattern
    /// 复用的会话修改模板，包含加载与分叉逻辑。
    async fn modify_session<F>(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        f: F,
    ) -> Result<(String, String), UpdateCellError>
    where
        F: Fn(&Vec<RecordBatch>, Arc<Schema>) -> Result<Vec<RecordBatch>, UpdateCellError>,
    {
        // 1. Lock Everything
        let mut sessions = self.sessions.lock().await;
        let mut active_map = self.active_table_sessions.lock().await;

        // 2. Identify Target Session
        let target_session_id = if let Some(id) = session_id {
            if sessions.contains_key(id) {
                id.to_string()
            } else {
                return Err(UpdateCellError::SessionNotFound {
                    session_id: id.to_string(),
                });
            }
        } else if let Some(id) = active_map.get(table_name) {
            // **[2026-02-25]** 变更原因：clippy 建议折叠 else-if。
            // **[2026-02-25]** 变更目的：消除 collapsible_else_if 警告。
            // **[2026-02-25]** 变更说明：逻辑保持不变，仅合并分支。
            // **[2026-02-25]** 变更说明：仍优先使用 active_map。
            // **[2026-02-25]** 变更说明：错误路径一致。
            // **[2026-02-25]** 变更说明：便于阅读维护。
            id.to_string()
        } else {
            return Err(UpdateCellError::NoActiveSession {
                table_name: table_name.to_string(),
            });
        };

        // 3. Check for Forking (Read-only/Default session)
        let session_info = sessions.get(&target_session_id).unwrap();
        let mut final_session_id = target_session_id.clone();
        let mut lance_uri = session_info.lance_path.clone();

        if session_info.is_default {
            println!("[SessionManager] Default/Readonly session detected. Auto-forking...");
            let new_session = self
                .create_session_internal(CreateSessionParams {
                    sessions: &mut sessions,
                    active_map: &mut active_map,
                    table_name,
                    parquet_path: &lance_uri,
                    session_name: None,
                    from_session_id: Some(final_session_id.clone()),
                    is_default: false,
                })
                .await
                .map_err(|e| UpdateCellError::Internal { reason: e })?;

            final_session_id = new_session.session_id;
            lance_uri = new_session.lance_path;
        }

        // 4. Load Data if needed
        let needs_load = {
            let session = sessions.get(&final_session_id).unwrap();
            session.current_data.is_none()
        };
        if needs_load {
            let uri = sessions.get(&final_session_id).unwrap().lance_path.clone();
            let batches = Self::load_batches_from_lance(&uri).await.map_err(|e| {
                UpdateCellError::LoadLanceFailed {
                    source: uri,
                    reason: e,
                }
            })?;
            let session = sessions.get_mut(&final_session_id).unwrap();
            session.current_data = Some(Arc::new(RwLock::new(batches)));
        }

        // 5. Apply Modification
        let session = sessions.get_mut(&final_session_id).unwrap();
        let current_data_lock = session.current_data.as_ref().unwrap();
        let mut batches = current_data_lock.write().await;

        if batches.is_empty() {
            return Err(UpdateCellError::EmptyDataset);
        }
        let schema = batches[0].schema();

        let new_batches = f(&batches, schema)?;
        *batches = new_batches;

        // 6. Mark Dirty
        let now = now_millis();
        session.dirty = true;
        session.pending_writes += 1;
        session.last_modified_at = now;

        Ok((lance_uri, final_session_id))
    }

    #[cfg(test)]
    /// 测试辅助：标记会话为脏并更新计数。
    async fn mark_dirty(&self, session_id: &str) {
        let now = now_millis();
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.dirty = true;
            session.pending_writes += 1;
            session.last_modified_at = now;
        }
    }

    /// 自动落盘循环。
    async fn auto_flush_loop(self: Arc<Self>) {
        let mut interval = time::interval(Duration::from_millis(self.auto_flush.interval_ms));
        loop {
            interval.tick().await;
            let due = self.collect_due_sessions().await;
            if !due.is_empty() {
                self.flush_sessions(due).await;
            }
        }
    }

    /// 计算需要落盘的会话列表。
    async fn collect_due_sessions(&self) -> Vec<String> {
        let now = now_millis();
        let mut sessions = self.sessions.lock().await;
        let mut due = Vec::new();
        for (id, session) in sessions.iter_mut() {
            if session.dirty
                && !session.is_flushing
                && (session.pending_writes >= self.auto_flush.max_pending_writes
                    || now.saturating_sub(session.last_modified_at) >= self.auto_flush.max_dirty_ms)
            {
                session.is_flushing = true;
                due.push(id.clone());
            }
        }
        due
    }

    /// 执行批量落盘并更新会话状态。
    async fn flush_sessions(&self, session_ids: Vec<String>) {
        for session_id in session_ids {
            let result = self.persist_session_by_id(&session_id).await;
            let now = now_millis();
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.is_flushing = false;
                if result.is_ok() {
                    session.dirty = false;
                    session.pending_writes = 0;
                    session.last_persisted_at = now;
                }
            }
        }
    }

    /// 按会话 ID 持久化当前内存数据。
    async fn persist_session_by_id(&self, session_id: &str) -> Result<String, String> {
        let (lance_uri, data_lock) = {
            let sessions = self.sessions.lock().await;
            let session = sessions.get(session_id).ok_or("Session not found")?;
            let data_lock = session
                .current_data
                .as_ref()
                .ok_or("No in-memory data to save")?;
            (session.lance_path.clone(), Arc::clone(data_lock))
        };

        let batches = data_lock.read().await;
        if batches.is_empty() {
            return Ok("Nothing to save (empty)".to_string());
        }
        let schema = batches[0].schema();
        let write_params = WriteParams {
            mode: lance::dataset::WriteMode::Overwrite,
            ..Default::default()
        };
        let reader = RecordBatchIterator::new(batches.clone().into_iter().map(Ok), schema);
        match Dataset::write(reader, lance_uri.as_str(), Some(write_params)).await {
            Ok(ds) => Ok(format!(
                "Saved to {}. Version: {}",
                lance_uri,
                ds.version().version
            )),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 从 Lance 数据集加载数据批次。
    async fn load_batches_from_lance(uri: &str) -> Result<Vec<RecordBatch>, String> {
        let ds = Dataset::open(uri).await.map_err(|e| e.to_string())?;
        let scanner = ds.scan();
        let batches = scanner
            .try_into_stream()
            .await
            .map_err(|e| e.to_string())?
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|e| e.to_string())?;
        Ok(batches)
    }

    // Register the current in-memory session data to the provided SessionContext
    /// 将当前会话数据注册到指定的 DataFusion 上下文。
    pub async fn register_session_to_context(
        &self,
        ctx: &SessionContext,
        table_name: &str,
    ) -> Result<(), String> {
        // 1. Get active session ID
        let active_id = {
            let active = self.active_table_sessions.lock().await;
            active
                .get(table_name)
                .cloned()
                .ok_or(format!("No active session for table '{}'", table_name))?
        };

        // 2. Get session data
        let batches = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions.get_mut(&active_id).ok_or("Session not found")?;

            if session.current_data.is_none() {
                println!(
                    "[SessionManager] Warning: Session data not in memory, hydrating from disk: {}",
                    session.lance_path
                );
                let ds = Dataset::open(&session.lance_path)
                    .await
                    .map_err(|e| e.to_string())?;
                let scanner = ds.scan();
                let batches = scanner
                    .try_into_stream()
                    .await
                    .map_err(|e| e.to_string())?
                    .try_collect::<Vec<RecordBatch>>()
                    .await
                    .map_err(|e| e.to_string())?;
                session.current_data = Some(Arc::new(RwLock::new(batches)));
            }

            if let Some(data_lock) = &session.current_data {
                let data = data_lock.read().await;
                data.clone()
            } else {
                return Err("Failed to hydrate session data".to_string());
            }
        };

        // 3. Update SessionContext (Deregister first to avoid "already exists" error)
        if let Err(e) = ctx.deregister_table(table_name) {
            println!(
                "[SessionManager] Warning: Failed to deregister '{}': {}",
                table_name, e
            );
        } else {
            println!("[SessionManager] Deregistered table '{}'", table_name);
        }

        if batches.is_empty() {
            println!("[SessionManager] Warning: Empty batches for {}", table_name);
        } else {
            let batch_count = batches.len();
            let schema = batches[0].schema();
            let provider = datafusion::datasource::MemTable::try_new(schema, vec![batches])
                .map_err(|e| e.to_string())?;
            ctx.register_table(table_name, Arc::new(provider))
                .map_err(|e| e.to_string())?;
            println!(
                "[SessionManager] Registered session data for '{}' ({} batches)",
                table_name, batch_count
            );
        }
        Ok(())
    }

    /// 批量更新多个单元格，支持自动列扩展。
    pub async fn batch_update_cells(
        &self,
        table_name: &str,
        session_id: Option<&str>,
        updates: Vec<CellUpdate>,
    ) -> Result<(String, String), UpdateCellError> {
        // 1. Lock Everything
        let mut sessions = self.sessions.lock().await;
        let mut active_map = self.active_table_sessions.lock().await;

        // 2. Identify Target Session
        let target_session_id = if let Some(id) = session_id {
            if sessions.contains_key(id) {
                id.to_string()
            } else {
                return Err(UpdateCellError::SessionNotFound {
                    session_id: id.to_string(),
                });
            }
        } else if let Some(id) = active_map.get(table_name) {
            // **[2026-02-25]** 变更原因：clippy 建议折叠 else-if。
            // **[2026-02-25]** 变更目的：消除 collapsible_else_if 警告。
            // **[2026-02-25]** 变更说明：逻辑保持不变，仅合并分支。
            // **[2026-02-25]** 变更说明：仍优先使用 active_map。
            // **[2026-02-25]** 变更说明：错误路径一致。
            // **[2026-02-25]** 变更说明：便于阅读维护。
            id.to_string()
        } else {
            return Err(UpdateCellError::NoActiveSession {
                table_name: table_name.to_string(),
            });
        };

        // 3. Check for Forking
        let session_info = sessions.get(&target_session_id).unwrap();
        let mut final_session_id = target_session_id.clone();
        let mut lance_uri = session_info.lance_path.clone();

        if session_info.is_default {
            println!("[SessionManager] Default/Readonly session detected (Batch). Auto-forking...");
            let new_session = self
                .create_session_internal(CreateSessionParams {
                    sessions: &mut sessions,
                    active_map: &mut active_map,
                    table_name,
                    parquet_path: &lance_uri,
                    session_name: None,
                    from_session_id: Some(final_session_id.clone()),
                    is_default: false,
                })
                .await
                .map_err(|e| UpdateCellError::Internal { reason: e })?;

            final_session_id = new_session.session_id;
            lance_uri = new_session.lance_path;
        }

        println!(
            "[SessionManager] Batch Updating '{}' (Session: {}) - {} updates",
            table_name,
            final_session_id,
            updates.len()
        );

        // 4. Modify In-Memory Data
        let needs_load = {
            let session =
                sessions
                    .get(&final_session_id)
                    .ok_or(UpdateCellError::SessionNotFound {
                        session_id: final_session_id.clone(),
                    })?;
            session.current_data.is_none()
        };
        if needs_load {
            let uri = {
                let session =
                    sessions
                        .get(&final_session_id)
                        .ok_or(UpdateCellError::SessionNotFound {
                            session_id: final_session_id.clone(),
                        })?;
                session.lance_path.clone()
            };
            let batches = Self::load_batches_from_lance(&uri).await.map_err(|e| {
                UpdateCellError::LoadLanceFailed {
                    source: uri.clone(),
                    reason: e,
                }
            })?;
            let session =
                sessions
                    .get_mut(&final_session_id)
                    .ok_or(UpdateCellError::SessionNotFound {
                        session_id: final_session_id.clone(),
                    })?;
            session.current_data = Some(Arc::new(RwLock::new(batches)));
        }

        let session =
            sessions
                .get_mut(&final_session_id)
                .ok_or(UpdateCellError::SessionNotFound {
                    session_id: final_session_id.clone(),
                })?;
        let current_data_lock = session
            .current_data
            .as_ref()
            .ok_or(UpdateCellError::SessionDataNotLoaded)?;
        let mut batches = current_data_lock.write().await;

        if batches.is_empty() {
            return Err(UpdateCellError::EmptyDataset);
        }
        let mut schema = batches[0].schema();

        let mut cols_involved = std::collections::HashSet::new();
        let mut cols_with_value = std::collections::HashSet::new();
        let mut max_row_idx_with_value: Option<usize> = None;
        for u in &updates {
            cols_involved.insert(u.col.clone());
            if !u.val.trim().is_empty() {
                cols_with_value.insert(u.col.clone());
                max_row_idx_with_value = Some(match max_row_idx_with_value {
                    Some(current) => current.max(u.row),
                    None => u.row,
                });
            }
        }

        let mut new_fields_needed = Vec::new();
        for col in &cols_involved {
            if schema.field_with_name(col).is_err() && cols_with_value.contains(col) {
                new_fields_needed.push(col.clone());
            }
        }

        if !new_fields_needed.is_empty() {
            let mut new_fields = schema.fields().to_vec();
            for col in &new_fields_needed {
                println!("[SessionManager] Batch: Adding new column '{}'", col);
                new_fields.push(Arc::new(Field::new(col, DataType::Utf8, true)));
            }
            schema = Arc::new(Schema::new(new_fields));

            let mut upgraded_batches = Vec::new();
            for batch in batches.iter() {
                let num_rows = batch.num_rows();
                let mut columns = batch.columns().to_vec();
                for _ in 0..new_fields_needed.len() {
                    columns.push(arrow::array::new_null_array(&DataType::Utf8, num_rows));
                }
                let new_batch = RecordBatch::try_new(schema.clone(), columns).map_err(|e| {
                    UpdateCellError::Internal {
                        reason: e.to_string(),
                    }
                })?;
                upgraded_batches.push(new_batch);
            }
            *batches = upgraded_batches;
        }

        // 6. Apply Updates
        let mut updates_by_col: HashMap<usize, HashMap<usize, String>> = HashMap::new();
        for u in updates {
            if let Ok(idx) = schema.index_of(&u.col) {
                updates_by_col.entry(idx).or_default().insert(u.row, u.val);
            }
        }

        let mut new_batches = Vec::new();
        let mut current_row_start = 0;

        for batch in batches.iter() {
            let num_rows = batch.num_rows();
            let current_row_end = current_row_start + num_rows;

            let mut batch_dirty = false;
            // **[2026-02-25]** 变更原因：clippy 建议遍历 values 以避免未使用 key。
            // **[2026-02-25]** 变更目的：消除 for_kv_map 警告。
            // **[2026-02-25]** 变更说明：逻辑保持不变。
            // **[2026-02-25]** 变更说明：仅调整遍历方式。
            // **[2026-02-25]** 变更说明：不影响更新判定结果。
            // **[2026-02-25]** 变更说明：代码更简洁。
            for row_map in updates_by_col.values() {
                for &r in row_map.keys() {
                    if r >= current_row_start && r < current_row_end {
                        batch_dirty = true;
                        break;
                    }
                }
                if batch_dirty {
                    break;
                }
            }

            if !batch_dirty {
                new_batches.push(batch.clone());
            } else {
                let mut new_columns = Vec::new();
                for (col_idx, old_col) in batch.columns().iter().enumerate() {
                    if let Some(row_map) = updates_by_col.get(&col_idx) {
                        let mut has_updates = false;
                        for &r in row_map.keys() {
                            if r >= current_row_start && r < current_row_end {
                                has_updates = true;
                                break;
                            }
                        }

                        if has_updates {
                            let new_col = Self::rebuild_column_with_updates(
                                old_col,
                                row_map,
                                current_row_start,
                                num_rows,
                            )?;
                            new_columns.push(new_col);
                        } else {
                            new_columns.push(old_col.clone());
                        }
                    } else {
                        new_columns.push(old_col.clone());
                    }
                }
                let new_batch = RecordBatch::try_new(schema.clone(), new_columns).map_err(|e| {
                    UpdateCellError::Internal {
                        reason: e.to_string(),
                    }
                })?;
                new_batches.push(new_batch);
            }

            current_row_start += num_rows;
        }

        if let Some(max_row_idx) = max_row_idx_with_value {
            if max_row_idx >= current_row_start {
                let rows_to_add = max_row_idx - current_row_start + 1;
                println!(
                    "[SessionManager] Batch: Appending {} new rows (Max Row: {})",
                    rows_to_add, max_row_idx
                );

                let mut new_columns: Vec<Arc<dyn Array>> = Vec::new();
                for (col_idx, field) in schema.fields().iter().enumerate() {
                    let mut builder_values = HashMap::new();
                    if let Some(row_map) = updates_by_col.get(&col_idx) {
                        for (&r, v) in row_map {
                            if r >= current_row_start && !v.trim().is_empty() {
                                builder_values.insert(r - current_row_start, v.clone());
                            }
                        }
                    }

                    let new_col =
                        Self::build_new_column(field.data_type(), rows_to_add, &builder_values)?;
                    new_columns.push(new_col);
                }
                let new_batch = RecordBatch::try_new(schema.clone(), new_columns).map_err(|e| {
                    UpdateCellError::Internal {
                        reason: e.to_string(),
                    }
                })?;
                new_batches.push(new_batch);
            }
        }

        *batches = new_batches;

        let now = now_millis();
        session.dirty = true;
        session.pending_writes += 1;
        session.last_modified_at = now;

        Ok((lance_uri, final_session_id))
    }

    /// 基于更新映射重建列数据。
    fn rebuild_column_with_updates(
        old_col: &Arc<dyn Array>,
        updates: &HashMap<usize, String>,
        start_row: usize,
        num_rows: usize,
    ) -> Result<Arc<dyn Array>, UpdateCellError> {
        let get_val =
            |local_idx: usize| -> Option<&String> { updates.get(&(start_row + local_idx)) };

        match old_col.data_type() {
            DataType::Utf8 => {
                let arr = old_col.as_any().downcast_ref::<StringArray>().unwrap();
                let mut builder =
                    arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 10);
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        builder.append_value(v);
                    // **[2026-02-25]** 变更原因：clippy 建议折叠 else-if。
                    // **[2026-02-25]** 变更目的：消除 collapsible_else_if 警告。
                    // **[2026-02-25]** 变更说明：逻辑分支保持一致。
                    // **[2026-02-25]** 变更说明：仅调整条件结构。
                    // **[2026-02-25]** 变更说明：不影响构建结果。
                    // **[2026-02-25]** 变更说明：便于后续阅读与维护。
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Int64 => {
                let arr = old_col.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Int64Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        let parsed = v.parse::<i64>().unwrap_or(0);
                        builder.append_value(parsed);
                    // **[2026-02-25]** 变更原因：clippy 建议折叠 else-if。
                    // **[2026-02-25]** 变更目的：消除 collapsible_else_if 警告。
                    // **[2026-02-25]** 变更说明：逻辑分支保持一致。
                    // **[2026-02-25]** 变更说明：仅调整条件结构。
                    // **[2026-02-25]** 变更说明：不影响构建结果。
                    // **[2026-02-25]** 变更说明：便于后续阅读与维护。
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Float64 => {
                let arr = old_col.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Float64Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        let parsed = v.parse::<f64>().unwrap_or(0.0);
                        builder.append_value(parsed);
                    // **[2026-02-25]** 变更原因：clippy 建议折叠 else-if。
                    // **[2026-02-25]** 变更目的：消除 collapsible_else_if 警告。
                    // **[2026-02-25]** 变更说明：逻辑分支保持一致。
                    // **[2026-02-25]** 变更说明：仅调整条件结构。
                    // **[2026-02-25]** 变更说明：不影响构建结果。
                    // **[2026-02-25]** 变更说明：便于后续阅读与维护。
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Boolean => {
                let arr = old_col.as_any().downcast_ref::<BooleanArray>().unwrap();
                let mut builder = arrow::array::BooleanBuilder::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        let val_lower = v.to_lowercase();
                        let val = val_lower == "true" || val_lower == "1";
                        builder.append_value(val);
                    // **[2026-02-25]** 变更原因：clippy 建议折叠 else-if。
                    // **[2026-02-25]** 变更目的：消除 collapsible_else_if 警告。
                    // **[2026-02-25]** 变更说明：逻辑分支保持一致。
                    // **[2026-02-25]** 变更说明：仅调整条件结构。
                    // **[2026-02-25]** 变更说明：不影响构建结果。
                    // **[2026-02-25]** 变更说明：便于后续阅读与维护。
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Date32 => {
                let arr = old_col.as_any().downcast_ref::<Date32Array>().unwrap();
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Date32Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        if let Ok(date) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                            builder.append_value(date.num_days_from_ce() - 719163);
                        } else {
                            builder.append_null();
                        }
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Date64 => {
                let arr = old_col.as_any().downcast_ref::<Date64Array>().unwrap();
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Date64Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        if let Ok(dt) = NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S") {
                            builder.append_value(dt.timestamp_millis());
                        } else if let Ok(d) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                            builder
                                .append_value(d.and_hms_opt(0, 0, 0).unwrap().timestamp_millis());
                        } else {
                            builder.append_null();
                        }
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Timestamp(TimeUnit::Microsecond, _) => {
                let arr = old_col
                    .as_any()
                    .downcast_ref::<TimestampMicrosecondArray>()
                    .unwrap();
                let mut builder =
                    arrow::array::PrimitiveBuilder::<TimestampMicrosecondType>::with_capacity(
                        num_rows,
                    );
                for i in 0..num_rows {
                    if let Some(v) = get_val(i) {
                        if let Ok(dt) = NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S") {
                            builder.append_value(dt.timestamp_micros());
                        } else {
                            builder.append_null();
                        }
                    } else if arr.is_null(i) {
                        builder.append_null();
                    } else {
                        builder.append_value(arr.value(i));
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            dt => Err(UpdateCellError::UnsupportedType {
                data_type: format!("{:?}", dt),
            }),
        }
    }

    /// 构造新列，用于批量插入或追加。
    fn build_new_column(
        dt: &DataType,
        num_rows: usize,
        values: &HashMap<usize, String>,
    ) -> Result<Arc<dyn Array>, UpdateCellError> {
        match dt {
            DataType::Utf8 => {
                let mut builder =
                    arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 10);
                for i in 0..num_rows {
                    if let Some(v) = values.get(&i) {
                        builder.append_value(v);
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Int64 => {
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Int64Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = values.get(&i) {
                        builder.append_value(v.parse::<i64>().unwrap_or(0));
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Float64 => {
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Float64Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = values.get(&i) {
                        builder.append_value(v.parse::<f64>().unwrap_or(0.0));
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Boolean => {
                let mut builder = arrow::array::BooleanBuilder::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = values.get(&i) {
                        let val_lower = v.to_lowercase();
                        builder.append_value(val_lower == "true" || val_lower == "1");
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Date32 => {
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Date32Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = values.get(&i) {
                        if let Ok(date) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                            builder.append_value(date.num_days_from_ce() - 719163);
                        } else {
                            builder.append_null();
                        }
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Date64 => {
                let mut builder =
                    arrow::array::PrimitiveBuilder::<Date64Type>::with_capacity(num_rows);
                for i in 0..num_rows {
                    if let Some(v) = values.get(&i) {
                        if let Ok(dt) = NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S") {
                            builder.append_value(dt.timestamp_millis());
                        } else if let Ok(d) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                            builder
                                .append_value(d.and_hms_opt(0, 0, 0).unwrap().timestamp_millis());
                        } else {
                            builder.append_null();
                        }
                    } else {
                        builder.append_null();
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            dt => Err(UpdateCellError::UnsupportedType {
                data_type: format!("{:?}", dt),
            }),
        }
    }

    // Persist Session to Disk (Save)
    /// 手动触发当前活动会话落盘。
    pub async fn save_session(&self, table_name: &str) -> Result<String, String> {
        let active_id = {
            let active = self.active_table_sessions.lock().await;
            active
                .get(table_name)
                .cloned()
                .ok_or(format!("No active session for table '{}'", table_name))?
        };

        {
            let mut sessions = self.sessions.lock().await;
            let session = sessions.get_mut(&active_id).ok_or("Session not found")?;
            if session.is_flushing {
                return Ok("Auto flush in progress".to_string());
            }
            if !session.dirty {
                return Ok("No changes to save".to_string());
            }
            session.is_flushing = true;
        }

        let result = self.persist_session_by_id(&active_id).await;
        let now = now_millis();

        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&active_id) {
            session.is_flushing = false;
            if result.is_ok() {
                session.dirty = false;
                session.pending_writes = 0;
                session.last_persisted_at = now;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::dataframe::DataFrameWriteOptions;
    use std::fs;

    #[tokio::test]
    /// 覆盖会话管理器的创建、分叉、切换与持久化。
    async fn test_session_manager_lifecycle() {
        let test_dir = "test_data_session_mgr";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // Create dummy parquet
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        // Test Create
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Test Session".to_string()),
                None,
                false,
            )
            .await
            .unwrap();
        assert_eq!(session.table_name, "test_table");
        assert_eq!(session.name, "Test Session");

        // Test Forking
        let session2 = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Forked Session".to_string()),
                Some(session.session_id.clone()),
                false,
            )
            .await
            .unwrap();
        assert_eq!(session2.name, "Forked Session");
        assert_ne!(session2.session_id, session.session_id);

        // Test List
        let sessions = sm.list_sessions("test_table").await;
        assert_eq!(sessions.len(), 2);

        // Test Switch
        sm.switch_session("test_table", &session.session_id)
            .await
            .unwrap();

        // Test Persistence (Reload)
        let sm2 = SessionManager::new(test_dir, metadata_manager.clone());
        let sessions2 = sm2.list_sessions("test_table").await;
        assert_eq!(sessions2.len(), 2);

        // Clean up
        drop(sm2);
        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证自动落盘会重置脏状态。
    async fn test_auto_flush_resets_dirty_state() {
        let test_dir = "test_data_session_mgr_flush";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let auto_flush = AutoFlushConfig {
            interval_ms: 10,
            max_pending_writes: 1,
            max_dirty_ms: 0,
        };
        let sm = SessionManager::new_with_config(test_dir, metadata_manager.clone(), auto_flush);

        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Test Session".to_string()),
                None,
                false,
            )
            .await
            .unwrap();
        sm.mark_dirty(&session.session_id).await;

        let due = sm.collect_due_sessions().await;
        sm.flush_sessions(due).await;

        let sessions = sm.sessions.lock().await;
        let info = sessions.get(&session.session_id).unwrap();
        assert!(!info.dirty);
        assert_eq!(info.pending_writes, 0);
        drop(sessions);

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证样式更新包含 format 字段。
    async fn test_update_style_includes_format() {
        // **[2026-02-16]** 变更原因：新增单元格格式测试。
        // **[2026-02-16]** 变更目的：确保 format 字段可被持久化。
        let test_dir = "test_data_update_style_format";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // **[2026-02-16]** 变更原因：构造基础数据。
        // **[2026-02-16]** 变更目的：提供最小可验证表结构。
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        // **[2026-02-16]** 变更原因：生成 Parquet。
        // **[2026-02-16]** 变更目的：复用现有创建会话逻辑。
        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：创建会话管理器。
        // **[2026-02-16]** 变更目的：触发样式更新流程。
        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        // **[2026-02-16]** 变更原因：创建活动会话。
        // **[2026-02-16]** 变更目的：确保 update_style 使用活动会话。
        let _session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：更新样式包含 format。
        // **[2026-02-16]** 变更目的：验证 format 可写入 metadata。
        let style = CellStyle {
            format: Some("percent".to_string()),
            ..Default::default()
        };
        let _ = sm.update_style("test_table", 0, 0, style).await.unwrap();

        // **[2026-02-16]** 变更原因：读取元数据验证。
        // **[2026-02-16]** 变更目的：保证格式化值被持久化。
        let metadata = sm.get_metadata("test_table").await.unwrap();
        let updated = metadata.styles.get("0,0").cloned().unwrap_or_default();
        assert_eq!(updated.format, Some("percent".to_string()));

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证范围样式更新可写入 format 字段。
    async fn test_update_style_range_applies_format() {
        // **[2026-02-16]** 变更原因：新增范围格式更新测试。
        // **[2026-02-16]** 变更目的：验证多单元格 format 写入。
        let test_dir = "test_data_update_style_range_format";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // **[2026-02-16]** 变更原因：构造基础数据。
        // **[2026-02-16]** 变更目的：提供最小可验证表结构。
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        // **[2026-02-16]** 变更原因：生成 Parquet。
        // **[2026-02-16]** 变更目的：复用现有创建会话逻辑。
        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：创建会话管理器。
        // **[2026-02-16]** 变更目的：触发样式更新流程。
        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        // **[2026-02-16]** 变更原因：创建活动会话。
        // **[2026-02-16]** 变更目的：确保 update_style_range 使用活动会话。
        let _session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：批量更新样式包含 format。
        // **[2026-02-16]** 变更目的：验证范围更新可写入 format。
        let style = CellStyle {
            format: Some("percent".to_string()),
            ..Default::default()
        };
        let range = MergeRange {
            start_row: 0,
            start_col: 0,
            end_row: 1,
            end_col: 1,
        };
        let _ = sm
            .update_style_range("test_table", range, style)
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：读取元数据验证。
        // **[2026-02-16]** 变更目的：保证范围内 format 被持久化。
        let metadata = sm.get_metadata("test_table").await.unwrap();
        for row in 0..=1 {
            for col in 0..=1 {
                let key = format!("{},{}", row, col);
                let updated = metadata.styles.get(&key).cloned().unwrap_or_default();
                assert_eq!(updated.format, Some("percent".to_string()));
            }
        }

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证非法 format 不影响原始数据值。
    async fn test_invalid_format_does_not_change_cell_value() {
        // **[2026-02-16]** 变更原因：新增非法 format 回归用例。
        // **[2026-02-16]** 变更目的：保证样式更新不污染数据。
        let test_dir = "test_data_invalid_format_raw_value";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // **[2026-02-16]** 变更原因：构造基础数据。
        // **[2026-02-16]** 变更目的：提供最小可验证表结构。
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        // **[2026-02-16]** 变更原因：生成 Parquet。
        // **[2026-02-16]** 变更目的：复用现有创建会话逻辑。
        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：创建会话管理器。
        // **[2026-02-16]** 变更目的：触发样式更新流程。
        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        // **[2026-02-16]** 变更原因：创建活动会话。
        // **[2026-02-16]** 变更目的：确保 update_style 使用活动会话。
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：设置原始值。
        // **[2026-02-16]** 变更目的：构造可验证的原值。
        let _ = sm
            .update_cell("test_table", Some(&session.session_id), 0, "name", "100")
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：写入非法 format。
        // **[2026-02-16]** 变更目的：验证格式化不影响原值。
        let style = CellStyle {
            format: Some("unknown_format".to_string()),
            ..Default::default()
        };
        let _ = sm.update_style("test_table", 0, 1, style).await.unwrap();

        // **[2026-02-16]** 变更原因：注册会话到查询上下文。
        // **[2026-02-16]** 变更目的：查询数据验证原值未变。
        sm.register_session_to_context(&ctx, "test_table")
            .await
            .unwrap();
        let df = ctx
            .sql("SELECT name FROM test_table WHERE id = 1")
            .await
            .unwrap();
        let batches = df.collect().await.unwrap();
        let val = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::StringArray>()
            .unwrap()
            .value(0)
            .to_string();
        assert_eq!(val, "100");

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证 format 更新不改变批量数据内容。
    async fn test_format_does_not_mutate_data_batches() {
        // **[2026-02-16]** 变更原因：新增 display/原值分离回归用例。
        // **[2026-02-16]** 变更目的：确保格式更新不修改数据批次。
        let test_dir = "test_data_format_no_data_mutation";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // **[2026-02-16]** 变更原因：构造基础数据。
        // **[2026-02-16]** 变更目的：提供最小可验证表结构。
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        // **[2026-02-16]** 变更原因：生成 Parquet。
        // **[2026-02-16]** 变更目的：复用现有创建会话逻辑。
        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：创建会话管理器。
        // **[2026-02-16]** 变更目的：触发样式更新流程。
        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        // **[2026-02-16]** 变更原因：创建活动会话。
        // **[2026-02-16]** 变更目的：确保 update_style_range 使用活动会话。
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：写入原始值。
        // **[2026-02-16]** 变更目的：建立可验证数据。
        let _ = sm
            .update_cell("test_table", Some(&session.session_id), 0, "name", "Alpha")
            .await
            .unwrap();
        let _ = sm
            .update_cell("test_table", Some(&session.session_id), 1, "name", "Beta")
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：范围更新 format。
        // **[2026-02-16]** 变更目的：验证样式更新不影响数据批次。
        let style = CellStyle {
            format: Some("percent".to_string()),
            ..Default::default()
        };
        let range = MergeRange {
            start_row: 0,
            start_col: 1,
            end_row: 1,
            end_col: 1,
        };
        let _ = sm
            .update_style_range("test_table", range, style)
            .await
            .unwrap();

        // **[2026-02-16]** 变更原因：注册会话到查询上下文。
        // **[2026-02-16]** 变更目的：查询数据验证原值未变。
        sm.register_session_to_context(&ctx, "test_table")
            .await
            .unwrap();
        let df = ctx
            .sql("SELECT name FROM test_table WHERE id IN (1, 2) ORDER BY id")
            .await
            .unwrap();
        let batches = df.collect().await.unwrap();
        let arr = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::StringArray>()
            .unwrap();
        assert_eq!(arr.value(0), "Alpha");
        assert_eq!(arr.value(1), "Beta");

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证默认会话在更新时触发分叉。
    async fn test_update_cell_forks_default_session() {
        let test_dir = "test_data_update_fork";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Default".to_string()),
                None,
                true,
            )
            .await
            .unwrap();
        let default_id = session.session_id.clone();

        let (_, new_id) = sm
            .update_cell("test_table", Some(&default_id), 0, "name", "Zed")
            .await
            .unwrap();
        assert_ne!(new_id, default_id);

        let (default_data, new_data) = {
            let sessions = sm.sessions.lock().await;
            let default_data = sessions
                .get(&default_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone();
            let new_data = sessions
                .get(&new_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone();
            (default_data, new_data)
        };

        let default_batches = default_data.read().await;
        let new_batches = new_data.read().await;

        let default_batch = &default_batches[0];
        let new_batch = &new_batches[0];
        let col_idx = default_batch.schema().index_of("name").unwrap();
        let default_col = default_batch
            .column(col_idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let new_col = new_batch
            .column(col_idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        assert_eq!(default_col.value(0), "Alice");
        assert_eq!(new_col.value(0), "Zed");

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证无活动会话时返回错误。
    async fn test_update_cell_no_active_session_error() {
        let test_dir = "test_data_update_no_active";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        let err = sm
            .update_cell("test_table", None, 0, "name", "Zed")
            .await
            .unwrap_err();
        assert!(matches!(err, UpdateCellError::NoActiveSession { .. }));

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证缺失会话时返回错误。
    async fn test_update_cell_session_not_found_error() {
        let test_dir = "test_data_update_missing_session";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());

        let err = sm
            .update_cell("test_table", Some("missing"), 0, "name", "Zed")
            .await
            .unwrap_err();
        assert!(matches!(err, UpdateCellError::SessionNotFound { .. }));

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    async fn test_batch_update_cells_ignores_trailing_empty_rows() {
        let test_dir = "test_data_batch_update_trailing_empty";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        let updates = vec![CellUpdate {
            row: 2,
            col: "name".to_string(),
            val: "".to_string(),
        }];

        let _ = sm
            .batch_update_cells("test_table", Some(&session.session_id), updates)
            .await
            .unwrap();

        let data_lock = {
            let sessions = sm.sessions.lock().await;
            sessions
                .get(&session.session_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone()
        };
        let batches = data_lock.read().await;
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2);

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    async fn test_insert_column_with_numeric_type_supports_sum() {
        let test_dir = "test_data_insert_column_numeric";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        sm.insert_column(
            "test_table",
            Some(&session.session_id),
            2,
            "amount",
            Some(DataType::Int64),
            // **[2026-02-15]** 变更原因：接口新增默认公式参数，测试场景无默认公式。
            None,
        )
        .await
        .unwrap();

        let updates = vec![
            CellUpdate {
                row: 0,
                col: "amount".to_string(),
                val: "10".to_string(),
            },
            CellUpdate {
                row: 1,
                col: "amount".to_string(),
                val: "20".to_string(),
            },
        ];
        sm.batch_update_cells("test_table", Some(&session.session_id), updates)
            .await
            .unwrap();

        sm.register_session_to_context(&ctx, "test_table")
            .await
            .unwrap();
        let df = ctx.sql("SELECT SUM(amount) FROM test_table").await.unwrap();
        let batches = df.collect().await.unwrap();
        let sum = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap()
            .value(0);
        assert_eq!(sum, 30);

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    async fn test_insert_column_with_float64_supports_sum() {
        let test_dir = "test_data_insert_column_float64";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        sm.insert_column(
            "test_table",
            Some(&session.session_id),
            2,
            "amount",
            Some(DataType::Float64),
            // **[2026-02-15]** 变更原因：接口新增默认公式参数，测试场景无默认公式。
            None,
        )
        .await
        .unwrap();

        let updates = vec![
            CellUpdate {
                row: 0,
                col: "amount".to_string(),
                val: "1.2".to_string(),
            },
            CellUpdate {
                row: 1,
                col: "amount".to_string(),
                val: "3.4".to_string(),
            },
        ];
        sm.batch_update_cells("test_table", Some(&session.session_id), updates)
            .await
            .unwrap();

        sm.register_session_to_context(&ctx, "test_table")
            .await
            .unwrap();
        let df = ctx.sql("SELECT SUM(amount) FROM test_table").await.unwrap();
        let batches = df.collect().await.unwrap();
        let sum = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Float64Array>()
            .unwrap()
            .value(0);
        assert!((sum - 4.6).abs() < 1e-9);

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证插入列时默认公式会填充所有行。
    async fn test_insert_column_applies_default_formula() {
        // **[2026-02-15]** 变更原因：新增“插入列默认公式”能力需要单测护航，先定义期望行为。
        let test_dir = "test_data_insert_column_default_formula";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // **[2026-02-15]** 变更目的：构造最小数据集，验证默认公式会覆盖新列所有行。
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        // **[2026-02-15]** 变更目的：创建会话管理器，使用默认公式插入列。
        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        // **[2026-02-15]** 变更原因：新增 default_formula 参数用于插入列时填充。
        let default_formula = "=A1+1";
        sm.insert_column(
            "test_table",
            Some(&session.session_id),
            2,
            "calc",
            Some(DataType::Utf8),
            Some(default_formula.to_string()),
        )
        .await
        .unwrap();

        // **[2026-02-15]** 变更目的：验证新列所有行都写入默认公式字符串。
        let data_lock = {
            let sessions = sm.sessions.lock().await;
            sessions
                .get(&session.session_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone()
        };
        let batches = data_lock.read().await;
        let batch = &batches[0];
        let col_idx = batch.schema().index_of("calc").unwrap();
        let col = batch
            .column(col_idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col.value(0), default_formula);
        assert_eq!(col.value(1), default_formula);

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    async fn test_insert_column_with_formula_marker_uses_nulls() {
        let test_dir = "test_data_insert_column_formula_marker";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        let formula_marker = serde_json::json!({
            "kind": "formula",
            "raw": "C*F",
            "sql": "\"col_c\" * \"col_f\""
        })
        .to_string();

        sm.insert_column(
            "test_table",
            Some(&session.session_id),
            2,
            "calc",
            Some(DataType::Utf8),
            Some(formula_marker),
        )
        .await
        .unwrap();

        let data_lock = {
            let sessions = sm.sessions.lock().await;
            sessions
                .get(&session.session_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone()
        };
        let batches = data_lock.read().await;
        let batch = &batches[0];
        let col_idx = batch.schema().index_of("calc").unwrap();
        let col = batch.column(col_idx);
        assert!(col.is_null(0));
        assert!(col.is_null(1));

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证插入行时列默认公式会应用到新行。
    async fn test_insert_row_applies_column_default_formulas() {
        // **[2026-02-15]** 变更原因：新增“插入行默认公式继承”行为需要单测保护。
        let test_dir = "test_data_insert_row_default_formula";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        // **[2026-02-15]** 变更目的：准备基础表数据与元数据默认公式。
        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        // **[2026-02-15]** 变更原因：通过元数据设置列默认公式，插入行时应自动填充。
        let default_formulas = serde_json::json!([null, "=ROW()+1"]);
        metadata_manager
            .store
            .save_table(&TableMetadata {
                catalog_name: "datafusion".to_string(),
                schema_name: "public".to_string(),
                table_name: "test_table".to_string(),
                file_path: parquet_path.clone(),
                source_type: "csv".to_string(),
                sheet_name: None,
                header_rows: None,
                header_mode: None,
                schema_json: None,
                stats_json: None,
                indexes_json: None,
                column_default_formulas_json: Some(default_formulas.to_string()),
            })
            .unwrap();

        // **[2026-02-15]** 变更目的：插入新行后验证“name”列采用默认公式。
        sm.insert_row("test_table", Some(&session.session_id), 2)
            .await
            .unwrap();

        let data_lock = {
            let sessions = sm.sessions.lock().await;
            sessions
                .get(&session.session_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone()
        };
        let batches = data_lock.read().await;
        // **[2026-02-15]** 变更原因：插入行可能分布在新的批次，需要按全局行号定位。
        let mut current_row = 0;
        let mut target_value: Option<String> = None;
        for batch in batches.iter() {
            let num_rows = batch.num_rows();
            if 2 >= current_row && 2 < current_row + num_rows {
                let local_idx = 2 - current_row;
                let col_idx = batch.schema().index_of("name").unwrap();
                let col = batch
                    .column(col_idx)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap();
                target_value = Some(col.value(local_idx).to_string());
                break;
            }
            current_row += num_rows;
        }
        assert_eq!(target_value.as_deref(), Some("=ROW()+1"));

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    async fn test_sum_ignores_nulls_for_numeric_column() {
        let test_dir = "test_data_sum_ignores_nulls";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob\n3,Carol";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();

        sm.insert_column(
            "test_table",
            Some(&session.session_id),
            2,
            "amount",
            Some(DataType::Int64),
            // **[2026-02-15]** 变更原因：接口新增默认公式参数，测试场景无默认公式。
            None,
        )
        .await
        .unwrap();

        let updates = vec![
            CellUpdate {
                row: 0,
                col: "amount".to_string(),
                val: "10".to_string(),
            },
            CellUpdate {
                row: 2,
                col: "amount".to_string(),
                val: "30".to_string(),
            },
        ];
        sm.batch_update_cells("test_table", Some(&session.session_id), updates)
            .await
            .unwrap();

        sm.register_session_to_context(&ctx, "test_table")
            .await
            .unwrap();
        let df = ctx.sql("SELECT SUM(amount) FROM test_table").await.unwrap();
        let batches = df.collect().await.unwrap();
        let sum = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::Int64Array>()
            .unwrap()
            .value(0);
        assert_eq!(sum, 40);

        drop(sm);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[tokio::test]
    /// 验证在缺失内存数据时会加载数据。
    async fn test_update_cell_loads_missing_session_data() {
        let test_dir = "test_data_update_load";
        if fs::metadata(test_dir).is_ok() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();

        let ctx = SessionContext::new();
        let csv_content = "id,name\n1,Alice\n2,Bob";
        let csv_path = format!("{}/test.csv", test_dir);
        fs::write(&csv_path, csv_content).unwrap();

        let df = ctx
            .read_csv(&csv_path, CsvReadOptions::default())
            .await
            .unwrap();
        let parquet_path = format!("{}/test.parquet", test_dir);
        df.write_parquet(&parquet_path, DataFrameWriteOptions::default(), None)
            .await
            .unwrap();

        let db_path = format!("{}/metadata.db", test_dir);
        let metadata_manager = Arc::new(MetadataManager::new(&db_path).unwrap());
        let sm = SessionManager::new(test_dir, metadata_manager.clone());
        let session = sm
            .create_session(
                "test_table",
                &parquet_path,
                Some("Editable".to_string()),
                None,
                false,
            )
            .await
            .unwrap();
        let session_id = session.session_id.clone();

        drop(sm);

        let sm2 = SessionManager::new(test_dir, metadata_manager.clone());
        let (_, updated_id) = sm2
            .update_cell("test_table", Some(&session_id), 1, "name", "Carol")
            .await
            .unwrap();
        assert_eq!(updated_id, session_id);

        let data_lock = {
            let sessions = sm2.sessions.lock().await;
            sessions
                .get(&session_id)
                .unwrap()
                .current_data
                .as_ref()
                .unwrap()
                .clone()
        };
        let batches = data_lock.read().await;
        let batch = &batches[0];
        let col_idx = batch.schema().index_of("name").unwrap();
        let col = batch
            .column(col_idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col.value(1), "Carol");

        drop(sm2);
        let _ = fs::remove_dir_all(test_dir);
    }
}
