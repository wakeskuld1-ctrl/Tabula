use arrow::array::{Array, ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use rusqlite::{params, Connection as SqliteConnection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(thiserror::Error, Debug)]
pub enum MetadataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Lock error")]
    LockError,
}

type Result<T> = std::result::Result<T, MetadataError>;
const CHANGE_NOTES: &[&str] = &[
    "变更备注 2026-02-28: 准备引入SQLite后端用于并发一致性存储的阶段1，原因是多进程/多节点写入需要事务与约束保障",
    "变更备注 2026-02-28: 统一SQLite连接与事务入口，原因是修复可变借用错误并集中写入路径",
    "变更备注 2026-02-28: 调整SQLite错误构造方式，原因是清理clippy警告以满足lint要求",
    "变更备注 2026-02-28: 增加测试运行记录校验用例，原因是引入CI式测试日志机制的TDD红阶段",
    "变更备注 2026-02-28: 增加三层模型迁移失败测试，原因是阶段2功能采用TDD先行验证",
    "变更备注 2026-02-28: 初始化三层模型表并迁移旧表数据，原因是第二阶段需要支持表实例与版本演进",
    "变更备注 2026-02-28: 增加Parquet到SQLite迁移入口，原因是历史数据需要自动迁入三层模型",
    "变更备注 2026-02-28: 调整布尔转整型写法，原因是消除clippy可读性警告",
    "变更备注 2026-02-28: 增加版本演进与回滚测试，原因是第三阶段采用TDD覆盖指针回滚逻辑",
    "变更备注 2026-02-28: 增加版本指针与回滚接口，原因是第三阶段需要支持版本演进与回退",
    "变更备注 2026-02-28: 改造SQLite保存逻辑以追加版本并维护指针，原因是实现版本演进与回滚一致性",
];

// Table Metadata Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub catalog_name: String,
    pub schema_name: String,
    pub table_name: String,
    pub file_path: String,
    pub source_type: String, // "csv", "excel", "sqlite", "oracle"
    pub sheet_name: Option<String>,
    pub schema_json: Option<String>,  // Serialized Arrow Schema
    pub stats_json: Option<String>,   // Serialized Statistics
    pub indexes_json: Option<String>, // Serialized Index Info
    pub stats_updated_at: Option<i64>,
    pub stats_stale: Option<bool>,
    pub stats_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    pub id: String,
    pub name: String,
    pub source_type: String, // "oracle", "yashandb"
    pub config: String,      // JSON string of connection params
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersionRecord {
    pub version: i64,
    pub schema_json: Option<String>,
    pub stats_json: Option<String>,
    pub indexes_json: Option<String>,
    pub stats_updated_at: i64,
    pub stats_stale: bool,
    pub stats_source: String,
}

#[derive(Debug, Clone)]
struct SchemaVersionRow {
    id: i64,
    version: i64,
    schema_json: Option<String>,
    stats_json: Option<String>,
    indexes_json: Option<String>,
    stats_updated_at: i64,
    stats_stale: bool,
    stats_source: String,
}

enum Backend {
    Parquet { dir_path: PathBuf },
    Sqlite { db_path: PathBuf },
}

pub struct MetadataStore {
    backend: Backend,
    tables: Mutex<Vec<TableMetadata>>,
    connections: Mutex<Vec<ConnectionMetadata>>,
}

impl MetadataStore {
    /// 初始化元数据存储
    ///
    /// **实现方案**:
    /// 1. 检查 `base_path` 后缀，决定使用 SQLite 后端 (.db) 还是 Parquet 后端 (目录)。
    /// 2. 如果是 SQLite，确保存储目录存在并初始化数据库 Schema (`init_sqlite`)。
    /// 3. 如果是 Parquet，确保存储目录存在。
    /// 4. 创建 `MetadataStore` 实例并调用 `reload` 加载初始数据。
    ///
    /// **调用链路**:
    /// - 被 `MetadataManager::new` 调用。
    ///
    /// **关键问题点**:
    /// - 自动检测后端类型，支持平滑迁移。
    /// - 初始化时会自动执行数据迁移逻辑。
    pub fn new(base_path: &str) -> Result<Self> {
        let _ = CHANGE_NOTES;
        let path = Path::new(base_path);
        let is_sqlite = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("db"))
            .unwrap_or(false);
        let backend = if is_sqlite {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            Self::init_sqlite(path)?;
            Backend::Sqlite {
                db_path: path.to_path_buf(),
            }
        } else {
            let dir_path = if path.extension().is_some() {
                path.parent()
                    .unwrap_or(Path::new("."))
                    .join("metadata_data")
            } else {
                path.to_path_buf()
            };
            if !dir_path.exists() {
                fs::create_dir_all(&dir_path)?;
            }
            Backend::Parquet { dir_path }
        };

        let store = Self {
            backend,
            tables: Mutex::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
        };

        // Load initial data
        store.reload()?;

        Ok(store)
    }

    /// 重新加载元数据到内存
    ///
    /// **实现方案**:
    /// 1. 获取内存锁 (`tables`, `connections`)。
    /// 2. 根据后端类型调用相应的加载函数 (`load_tables_parquet` 或 `load_tables_sqlite`)。
    /// 3. 如果从 Parquet 加载时检测到旧数据格式 (`migration_needed`)，则自动回写迁移。
    /// 4. 更新内存中的 `tables` 和 `connections` 列表。
    ///
    /// **调用链路**:
    /// - 被 `new` 调用。
    /// - 可被外部调用以强制刷新缓存。
    ///
    /// **关键问题点**:
    /// - 线程安全：使用 Mutex 保护内存数据。
    /// - 自动迁移：Parquet 模式下处理旧数据字段的默认值填充。
    fn reload(&self) -> Result<()> {
        let mut tables = self.tables.lock().map_err(|_| MetadataError::LockError)?;
        let mut conns = self
            .connections
            .lock()
            .map_err(|_| MetadataError::LockError)?;

        let (loaded_tables, migration_needed) = match &self.backend {
            Backend::Parquet { .. } => self.load_tables_parquet()?,
            Backend::Sqlite { .. } => (self.load_tables_sqlite()?, false),
        };
        *tables = loaded_tables;
        if migration_needed {
            self.save_tables_to_disk(&tables)?;
        }
        *conns = match &self.backend {
            Backend::Parquet { .. } => self.load_connections_parquet()?,
            Backend::Sqlite { .. } => self.load_connections_sqlite()?,
        };

        Ok(())
    }

    fn get_connections_path(&self) -> PathBuf {
        match &self.backend {
            Backend::Parquet { dir_path } => dir_path.join("connections.parquet"),
            Backend::Sqlite { db_path } => db_path.clone(),
        }
    }

    fn get_tables_path(&self) -> PathBuf {
        match &self.backend {
            Backend::Parquet { dir_path } => dir_path.join("tables.parquet"),
            Backend::Sqlite { db_path } => db_path.clone(),
        }
    }

    // --- Persistence Logic ---

    /// 初始化 SQLite 数据库 Schema
    ///
    /// **实现方案**:
    /// 1. 创建基础表 `connections` 和 `tables` (旧版兼容)。
    /// 2. 初始化三层模型 Schema (`table_instances`, `schema_versions`, `schema_migrations`)。
    /// 3. 执行数据迁移：
    ///    - 将旧版 `tables` 数据迁移到三层模型 (`migrate_legacy_tables_to_three_layer`)。
    ///    - 如果存在 Parquet 目录，将其数据导入 SQLite (`migrate_parquet_dir_to_three_layer`)。
    ///    - 初始化版本指针 (`migrate_current_schema_version_pointer`)。
    ///
    /// **调用链路**:
    /// - 被 `new` 调用 (当使用 SQLite 后端时)。
    ///
    /// **关键问题点**:
    /// - 幂等性：所有 CREATE 语句使用 `IF NOT EXISTS`，迁移逻辑检查版本号。
    /// - 兼容性：同时保留旧表结构以支持未迁移的代码。
    fn init_sqlite(db_path: &Path) -> Result<()> {
        let mut conn = SqliteConnection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS connections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL,
                config TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tables (
                catalog_name TEXT NOT NULL,
                schema_name TEXT NOT NULL,
                table_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                source_type TEXT NOT NULL,
                sheet_name TEXT,
                schema_json TEXT,
                stats_json TEXT,
                indexes_json TEXT,
                stats_updated_at INTEGER NOT NULL DEFAULT 0,
                stats_stale INTEGER NOT NULL DEFAULT 1,
                stats_source TEXT NOT NULL DEFAULT 'unknown',
                UNIQUE(catalog_name, schema_name, table_name, source_type)
            );",
        )?;
        Self::init_three_layer_schema(&conn)?;
        Self::migrate_legacy_tables_to_three_layer(&mut conn)?;
        Self::migrate_parquet_dir_to_three_layer(&mut conn, db_path)?;
        Self::migrate_current_schema_version_pointer(&mut conn)?;
        Ok(())
    }

    /// 初始化三层模型表结构
    ///
    /// **实现方案**:
    /// 创建 `table_instances` (表实例), `schema_versions` (版本历史), `schema_migrations` (迁移记录)。
    /// 建立外键约束维护完整性。
    fn init_three_layer_schema(conn: &SqliteConnection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS table_instances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                catalog_name TEXT NOT NULL,
                schema_name TEXT NOT NULL,
                table_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                source_type TEXT NOT NULL,
                sheet_name TEXT,
                current_schema_version_id INTEGER,
                UNIQUE(catalog_name, schema_name, table_name, source_type),
                FOREIGN KEY(current_schema_version_id) REFERENCES schema_versions(id)
            );
            CREATE TABLE IF NOT EXISTS schema_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_instance_id INTEGER NOT NULL,
                version INTEGER NOT NULL,
                schema_json TEXT,
                stats_json TEXT,
                indexes_json TEXT,
                stats_updated_at INTEGER NOT NULL DEFAULT 0,
                stats_stale INTEGER NOT NULL DEFAULT 1,
                stats_source TEXT NOT NULL DEFAULT 'unknown',
                FOREIGN KEY(table_instance_id) REFERENCES table_instances(id)
            );
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    /// 将旧版 Tables 表数据迁移到三层模型
    ///
    /// **实现方案**:
    /// 1. 检查 `schema_migrations` 版本号，避免重复迁移。
    /// 2. 开启事务。
    /// 3. 遍历 `tables` 表的所有行。
    /// 4. 为每行数据创建 `table_instances` 记录。
    /// 5. 创建初始版本 `schema_versions` (Version 1)。
    /// 6. 更新实例的 `current_schema_version_id` 指针。
    /// 7. 记录迁移版本号并提交事务。
    ///
    /// **关键问题点**:
    /// - 默认值处理：`stats_updated_at` 默认为 0，`stats_stale` 默认为 true。
    fn migrate_legacy_tables_to_three_layer(conn: &mut SqliteConnection) -> Result<()> {
        let already_applied: Option<i64> = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1 LIMIT 1",
                params![2],
                |row| row.get(0),
            )
            .optional()?;
        if already_applied.is_some() {
            return Ok(());
        }

        let tx = conn.transaction()?;
        let has_pointer = Self::table_instances_has_column_tx(&tx, "current_schema_version_id")?;
        {
            let mut stmt = tx.prepare(
                "SELECT catalog_name, schema_name, table_name, file_path, source_type, sheet_name,
                        schema_json, stats_json, indexes_json, stats_updated_at, stats_stale, stats_source
                 FROM tables",
            )?;
            let rows = stmt.query_map([], |row| {
                let stats_stale: Option<i64> = row.get(10)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<i64>>(9)?,
                    stats_stale.map(|v| v != 0),
                    row.get::<_, Option<String>>(11)?,
                ))
            })?;

            for row in rows {
                let (
                    catalog_name,
                    schema_name,
                    table_name,
                    file_path,
                    source_type,
                    sheet_name,
                    schema_json,
                    stats_json,
                    indexes_json,
                    stats_updated_at,
                    stats_stale,
                    stats_source,
                ) = row?;

                tx.execute(
                    "INSERT OR IGNORE INTO table_instances (
                        catalog_name, schema_name, table_name, file_path, source_type, sheet_name
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        catalog_name,
                        schema_name,
                        table_name,
                        file_path,
                        source_type,
                        sheet_name
                    ],
                )?;

                let table_instance_id: i64 = tx.query_row(
                    "SELECT id FROM table_instances
                     WHERE catalog_name = ?1 AND schema_name = ?2 AND table_name = ?3 AND source_type = ?4",
                    params![catalog_name, schema_name, table_name, source_type],
                    |r| r.get(0),
                )?;

                let existing_version: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM schema_versions
                         WHERE table_instance_id = ?1 AND version = 1 LIMIT 1",
                        params![table_instance_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let version_id = if let Some(version_id) = existing_version {
                    version_id
                } else {
                    tx.execute(
                        "INSERT INTO schema_versions (
                            table_instance_id, version, schema_json, stats_json, indexes_json,
                            stats_updated_at, stats_stale, stats_source
                         ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            table_instance_id,
                            schema_json,
                            stats_json,
                            indexes_json,
                            stats_updated_at.unwrap_or(0),
                            if stats_stale.unwrap_or(true) { 1 } else { 0 },
                            stats_source.unwrap_or_else(|| "unknown".to_string())
                        ],
                    )?;
                    tx.last_insert_rowid()
                };
                if has_pointer {
                    tx.execute(
                        "UPDATE table_instances SET current_schema_version_id = ?1 WHERE id = ?2",
                        params![version_id, table_instance_id],
                    )?;
                }
            }
        }

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![2, now_timestamp()],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn migrate_parquet_dir_to_three_layer(
        conn: &mut SqliteConnection,
        db_path: &Path,
    ) -> Result<()> {
        let already_applied: Option<i64> = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1 LIMIT 1",
                params![3],
                |row| row.get(0),
            )
            .optional()?;
        if already_applied.is_some() {
            return Ok(());
        }

        let parent_dir = db_path.parent().unwrap_or_else(|| Path::new("."));
        let parquet_dir = parent_dir.join("metadata_data");
        let connections_path = parquet_dir.join("connections.parquet");
        let tables_path = parquet_dir.join("tables.parquet");
        if !connections_path.exists() && !tables_path.exists() {
            return Ok(());
        }

        let connections = Self::load_connections_parquet_from_dir(&parquet_dir)?;
        let (tables, _) = Self::load_tables_parquet_from_dir(&parquet_dir)?;

        let tx = conn.transaction()?;
        for conn_meta in connections {
            tx.execute(
                "INSERT OR REPLACE INTO connections (id, name, source_type, config)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    conn_meta.id,
                    conn_meta.name,
                    conn_meta.source_type,
                    conn_meta.config
                ],
            )?;
        }

        for table_meta in tables {
            Self::insert_table_instance_and_version(&tx, &table_meta)?;
            Self::insert_legacy_table_row(&tx, &table_meta)?;
        }

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![3, now_timestamp()],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn migrate_current_schema_version_pointer(conn: &mut SqliteConnection) -> Result<()> {
        let already_applied: Option<i64> = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1 LIMIT 1",
                params![4],
                |row| row.get(0),
            )
            .optional()?;
        if already_applied.is_some() {
            return Ok(());
        }

        let has_pointer = Self::table_instances_has_column(conn, "current_schema_version_id")?;
        if !has_pointer {
            conn.execute_batch(
                "ALTER TABLE table_instances ADD COLUMN current_schema_version_id INTEGER;",
            )?;
        }

        conn.execute_batch(
            "UPDATE table_instances
             SET current_schema_version_id = (
                 SELECT id FROM schema_versions
                 WHERE table_instance_id = table_instances.id
                 ORDER BY version DESC
                 LIMIT 1
             )
             WHERE current_schema_version_id IS NULL;",
        )?;
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![4, now_timestamp()],
        )?;
        Ok(())
    }

    fn table_instances_has_column(conn: &SqliteConnection, column: &str) -> Result<bool> {
        let mut stmt = conn.prepare("PRAGMA table_info(table_instances)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn table_instances_has_column_tx(tx: &rusqlite::Transaction, column: &str) -> Result<bool> {
        let mut stmt = tx.prepare("PRAGMA table_info(table_instances)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn load_connections_parquet_from_dir(dir_path: &Path) -> Result<Vec<ConnectionMetadata>> {
        let path = dir_path.join("connections.parquet");
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut result = Vec::new();
        for batch_result in reader {
            let batch = batch_result?;
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let names = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let types = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let configs = batch
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            for i in 0..batch.num_rows() {
                result.push(ConnectionMetadata {
                    id: ids.value(i).to_string(),
                    name: names.value(i).to_string(),
                    source_type: types.value(i).to_string(),
                    config: configs.value(i).to_string(),
                });
            }
        }
        Ok(result)
    }

    /// 从 Parquet 目录加载表元数据
    ///
    /// **实现方案**:
    /// 1. 使用 `ParquetRecordBatchReader` 读取 `tables.parquet`。
    /// 2. 遍历 RecordBatch，逐列提取数据。
    /// 3. 将列数据重组为 `TableMetadata` 对象。
    /// 4. 检查是否缺少新字段（如 `stats_source`），如果是，则标记 `migration_needed` 为 true。
    ///
    /// **调用链路**:
    /// - 被 `reload` 调用 (Parquet 模式)。
    ///
    /// **关键问题点**:
    /// - 性能：批量读取，但逐行反序列化，可能有优化空间。
    /// - 兼容性：处理列缺失或 null 值，提供默认值。
    fn load_tables_parquet_from_dir(dir_path: &Path) -> Result<(Vec<TableMetadata>, bool)> {
        let path = dir_path.join("tables.parquet");
        if !path.exists() {
            return Ok((Vec::new(), false));
        }

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut result = Vec::new();
        let mut migration_needed = false;
        for batch_result in reader {
            let batch = batch_result?;
            let catalogs = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let schemas = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let tables = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let paths = batch
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let types = batch
                .column(4)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let sheets = batch
                .column(5)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let schema_jsons = batch
                .column(6)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let stats_jsons = batch
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let indexes_jsons = batch
                .column(8)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let stats_updated_ats = if batch.num_columns() > 9 {
                Some(
                    batch
                        .column(9)
                        .as_any()
                        .downcast_ref::<Int64Array>()
                        .unwrap(),
                )
            } else {
                None
            };
            let stats_stales = if batch.num_columns() > 10 {
                Some(
                    batch
                        .column(10)
                        .as_any()
                        .downcast_ref::<BooleanArray>()
                        .unwrap(),
                )
            } else {
                None
            };
            let stats_sources = if batch.num_columns() > 11 {
                Some(
                    batch
                        .column(11)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .unwrap(),
                )
            } else {
                None
            };

            for i in 0..batch.num_rows() {
                let stats_updated_at = if let Some(col) = stats_updated_ats {
                    if col.is_null(i) {
                        migration_needed = true;
                        Some(0)
                    } else {
                        Some(col.value(i))
                    }
                } else {
                    migration_needed = true;
                    Some(0)
                };
                let stats_stale = if let Some(col) = stats_stales {
                    if col.is_null(i) {
                        migration_needed = true;
                        Some(true)
                    } else {
                        Some(col.value(i))
                    }
                } else {
                    migration_needed = true;
                    Some(true)
                };
                let stats_source = if let Some(col) = stats_sources {
                    if col.is_null(i) {
                        migration_needed = true;
                        Some("unknown".to_string())
                    } else {
                        Some(col.value(i).to_string())
                    }
                } else {
                    migration_needed = true;
                    Some("unknown".to_string())
                };
                result.push(TableMetadata {
                    catalog_name: catalogs.value(i).to_string(),
                    schema_name: schemas.value(i).to_string(),
                    table_name: tables.value(i).to_string(),
                    file_path: paths.value(i).to_string(),
                    source_type: types.value(i).to_string(),
                    sheet_name: if sheets.is_null(i) {
                        None
                    } else {
                        Some(sheets.value(i).to_string())
                    },
                    schema_json: if schema_jsons.is_null(i) {
                        None
                    } else {
                        Some(schema_jsons.value(i).to_string())
                    },
                    stats_json: if stats_jsons.is_null(i) {
                        None
                    } else {
                        Some(stats_jsons.value(i).to_string())
                    },
                    indexes_json: if indexes_jsons.is_null(i) {
                        None
                    } else {
                        Some(indexes_jsons.value(i).to_string())
                    },
                    stats_updated_at,
                    stats_stale,
                    stats_source,
                });
            }
        }
        Ok((result, migration_needed))
    }

    fn insert_table_instance_and_version(
        tx: &rusqlite::Transaction,
        table_meta: &TableMetadata,
    ) -> Result<()> {
        let has_pointer = Self::table_instances_has_column_tx(tx, "current_schema_version_id")?;
        tx.execute(
            "INSERT OR IGNORE INTO table_instances (
                catalog_name, schema_name, table_name, file_path, source_type, sheet_name
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                table_meta.catalog_name,
                table_meta.schema_name,
                table_meta.table_name,
                table_meta.file_path,
                table_meta.source_type,
                table_meta.sheet_name
            ],
        )?;

        let table_instance_id: i64 = tx.query_row(
            "SELECT id FROM table_instances
             WHERE catalog_name = ?1 AND schema_name = ?2 AND table_name = ?3 AND source_type = ?4",
            params![
                table_meta.catalog_name,
                table_meta.schema_name,
                table_meta.table_name,
                table_meta.source_type
            ],
            |r| r.get(0),
        )?;

        let existing_version: Option<i64> = tx
            .query_row(
                "SELECT id FROM schema_versions
                 WHERE table_instance_id = ?1 AND version = 1 LIMIT 1",
                params![table_instance_id],
                |r| r.get(0),
            )
            .optional()?;
        let version_id = if let Some(version_id) = existing_version {
            version_id
        } else {
            tx.execute(
                "INSERT INTO schema_versions (
                    table_instance_id, version, schema_json, stats_json, indexes_json,
                    stats_updated_at, stats_stale, stats_source
                 ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    table_instance_id,
                    table_meta.schema_json,
                    table_meta.stats_json,
                    table_meta.indexes_json,
                    table_meta.stats_updated_at.unwrap_or(0),
                    if table_meta.stats_stale.unwrap_or(true) {
                        1
                    } else {
                        0
                    },
                    table_meta
                        .stats_source
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string())
                ],
            )?;
            tx.last_insert_rowid()
        };
        if has_pointer {
            tx.execute(
                "UPDATE table_instances SET current_schema_version_id = ?1 WHERE id = ?2",
                params![version_id, table_instance_id],
            )?;
        }
        Ok(())
    }

    fn insert_legacy_table_row(
        tx: &rusqlite::Transaction,
        table_meta: &TableMetadata,
    ) -> Result<()> {
        let stats_stale = table_meta.stats_stale.map(|v| if v { 1 } else { 0 });
        tx.execute(
            "INSERT OR REPLACE INTO tables (
                catalog_name, schema_name, table_name, file_path, source_type,
                sheet_name, schema_json, stats_json, indexes_json,
                stats_updated_at, stats_stale, stats_source
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                table_meta.catalog_name,
                table_meta.schema_name,
                table_meta.table_name,
                table_meta.file_path,
                table_meta.source_type,
                table_meta.sheet_name,
                table_meta.schema_json,
                table_meta.stats_json,
                table_meta.indexes_json,
                table_meta.stats_updated_at,
                stats_stale,
                table_meta.stats_source
            ],
        )?;
        Ok(())
    }

    fn with_sqlite_conn<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&SqliteConnection) -> Result<T>,
    {
        let db_path = match &self.backend {
            Backend::Sqlite { db_path } => db_path,
            Backend::Parquet { .. } => {
                return Err(MetadataError::Io(std::io::Error::other(
                    "sqlite backend not enabled",
                )))
            }
        };
        let conn = SqliteConnection::open(db_path)?;
        f(&conn)
    }

    fn with_sqlite_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T>,
    {
        let db_path = match &self.backend {
            Backend::Sqlite { db_path } => db_path,
            Backend::Parquet { .. } => {
                return Err(MetadataError::Io(std::io::Error::other(
                    "sqlite backend not enabled",
                )))
            }
        };
        let mut conn = SqliteConnection::open(db_path)?;
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    fn load_connections_parquet(&self) -> Result<Vec<ConnectionMetadata>> {
        let path = self.get_connections_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut result = Vec::new();

        for batch_result in reader {
            let batch = batch_result?;
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let names = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let types = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let configs = batch
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            for i in 0..batch.num_rows() {
                result.push(ConnectionMetadata {
                    id: ids.value(i).to_string(),
                    name: names.value(i).to_string(),
                    source_type: types.value(i).to_string(),
                    config: configs.value(i).to_string(),
                });
            }
        }
        Ok(result)
    }

    fn save_connections_parquet(&self, conns: &[ConnectionMetadata]) -> Result<()> {
        let path = self.get_connections_path();
        let file = File::create(path)?;

        let ids = StringArray::from(conns.iter().map(|c| c.id.clone()).collect::<Vec<_>>());
        let names = StringArray::from(conns.iter().map(|c| c.name.clone()).collect::<Vec<_>>());
        let types = StringArray::from(
            conns
                .iter()
                .map(|c| c.source_type.clone())
                .collect::<Vec<_>>(),
        );
        let configs = StringArray::from(conns.iter().map(|c| c.config.clone()).collect::<Vec<_>>());

        let batch = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(ids) as ArrayRef),
            ("name", Arc::new(names) as ArrayRef),
            ("source_type", Arc::new(types) as ArrayRef),
            ("config", Arc::new(configs) as ArrayRef),
        ])?;

        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }

    fn load_tables_parquet(&self) -> Result<(Vec<TableMetadata>, bool)> {
        let path = self.get_tables_path();
        if !path.exists() {
            return Ok((Vec::new(), false));
        }

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut result = Vec::new();
        let mut migration_needed = false;

        for batch_result in reader {
            let batch = batch_result?;
            let catalogs = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let schemas = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let tables = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let paths = batch
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let types = batch
                .column(4)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let sheets = batch
                .column(5)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let schema_jsons = batch
                .column(6)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let stats_jsons = batch
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let indexes_jsons = batch
                .column(8)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let stats_updated_ats = if batch.num_columns() > 9 {
                Some(
                    batch
                        .column(9)
                        .as_any()
                        .downcast_ref::<Int64Array>()
                        .unwrap(),
                )
            } else {
                None
            };
            let stats_stales = if batch.num_columns() > 10 {
                Some(
                    batch
                        .column(10)
                        .as_any()
                        .downcast_ref::<BooleanArray>()
                        .unwrap(),
                )
            } else {
                None
            };
            let stats_sources = if batch.num_columns() > 11 {
                Some(
                    batch
                        .column(11)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .unwrap(),
                )
            } else {
                None
            };

            for i in 0..batch.num_rows() {
                let stats_updated_at = if let Some(col) = stats_updated_ats {
                    if col.is_null(i) {
                        migration_needed = true;
                        Some(0)
                    } else {
                        Some(col.value(i))
                    }
                } else {
                    migration_needed = true;
                    Some(0)
                };
                let stats_stale = if let Some(col) = stats_stales {
                    if col.is_null(i) {
                        migration_needed = true;
                        Some(true)
                    } else {
                        Some(col.value(i))
                    }
                } else {
                    migration_needed = true;
                    Some(true)
                };
                let stats_source = if let Some(col) = stats_sources {
                    if col.is_null(i) {
                        migration_needed = true;
                        Some("unknown".to_string())
                    } else {
                        Some(col.value(i).to_string())
                    }
                } else {
                    migration_needed = true;
                    Some("unknown".to_string())
                };
                result.push(TableMetadata {
                    catalog_name: catalogs.value(i).to_string(),
                    schema_name: schemas.value(i).to_string(),
                    table_name: tables.value(i).to_string(),
                    file_path: paths.value(i).to_string(),
                    source_type: types.value(i).to_string(),
                    sheet_name: if sheets.is_null(i) {
                        None
                    } else {
                        Some(sheets.value(i).to_string())
                    },
                    schema_json: if schema_jsons.is_null(i) {
                        None
                    } else {
                        Some(schema_jsons.value(i).to_string())
                    },
                    stats_json: if stats_jsons.is_null(i) {
                        None
                    } else {
                        Some(stats_jsons.value(i).to_string())
                    },
                    indexes_json: if indexes_jsons.is_null(i) {
                        None
                    } else {
                        Some(indexes_jsons.value(i).to_string())
                    },
                    stats_updated_at,
                    stats_stale,
                    stats_source,
                });
            }
        }
        Ok((result, migration_needed))
    }

    fn save_tables_parquet(&self, tables: &[TableMetadata]) -> Result<()> {
        let path = self.get_tables_path();
        let file = File::create(path)?;

        let catalogs = StringArray::from(
            tables
                .iter()
                .map(|t| t.catalog_name.clone())
                .collect::<Vec<_>>(),
        );
        let schemas = StringArray::from(
            tables
                .iter()
                .map(|t| t.schema_name.clone())
                .collect::<Vec<_>>(),
        );
        let table_names = StringArray::from(
            tables
                .iter()
                .map(|t| t.table_name.clone())
                .collect::<Vec<_>>(),
        );
        let paths = StringArray::from(
            tables
                .iter()
                .map(|t| t.file_path.clone())
                .collect::<Vec<_>>(),
        );
        let types = StringArray::from(
            tables
                .iter()
                .map(|t| t.source_type.clone())
                .collect::<Vec<_>>(),
        );
        let sheets = StringArray::from(
            tables
                .iter()
                .map(|t| t.sheet_name.clone())
                .collect::<Vec<_>>(),
        );
        let schema_jsons = StringArray::from(
            tables
                .iter()
                .map(|t| t.schema_json.clone())
                .collect::<Vec<_>>(),
        );
        let stats_jsons = StringArray::from(
            tables
                .iter()
                .map(|t| t.stats_json.clone())
                .collect::<Vec<_>>(),
        );
        let indexes_jsons = StringArray::from(
            tables
                .iter()
                .map(|t| t.indexes_json.clone())
                .collect::<Vec<_>>(),
        );
        let stats_updated_ats = Int64Array::from(
            tables
                .iter()
                .map(|t| t.stats_updated_at)
                .collect::<Vec<_>>(),
        );
        let stats_stales =
            BooleanArray::from(tables.iter().map(|t| t.stats_stale).collect::<Vec<_>>());
        let stats_sources = StringArray::from(
            tables
                .iter()
                .map(|t| t.stats_source.clone())
                .collect::<Vec<_>>(),
        );

        let batch = RecordBatch::try_from_iter(vec![
            ("catalog_name", Arc::new(catalogs) as ArrayRef),
            ("schema_name", Arc::new(schemas) as ArrayRef),
            ("table_name", Arc::new(table_names) as ArrayRef),
            ("file_path", Arc::new(paths) as ArrayRef),
            ("source_type", Arc::new(types) as ArrayRef),
            ("sheet_name", Arc::new(sheets) as ArrayRef),
            ("schema_json", Arc::new(schema_jsons) as ArrayRef),
            ("stats_json", Arc::new(stats_jsons) as ArrayRef),
            ("indexes_json", Arc::new(indexes_jsons) as ArrayRef),
            ("stats_updated_at", Arc::new(stats_updated_ats) as ArrayRef),
            ("stats_stale", Arc::new(stats_stales) as ArrayRef),
            ("stats_source", Arc::new(stats_sources) as ArrayRef),
        ])?;

        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }

    fn load_connections_sqlite(&self) -> Result<Vec<ConnectionMetadata>> {
        self.with_sqlite_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, source_type, config FROM connections")?;
            let rows = stmt.query_map([], |row| {
                Ok(ConnectionMetadata {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_type: row.get(2)?,
                    config: row.get(3)?,
                })
            })?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row?);
            }
            Ok(result)
        })
    }

    fn save_connections_sqlite(&self, conns: &[ConnectionMetadata]) -> Result<()> {
        self.with_sqlite_transaction(|tx| {
            tx.execute("DELETE FROM connections", [])?;
            let mut stmt = tx.prepare(
                "INSERT INTO connections (id, name, source_type, config) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for c in conns {
                stmt.execute(params![c.id, c.name, c.source_type, c.config])?;
            }
            Ok(())
        })
    }

    fn load_tables_sqlite(&self) -> Result<Vec<TableMetadata>> {
        self.with_sqlite_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT catalog_name, schema_name, table_name, file_path, source_type, sheet_name,
                        schema_json, stats_json, indexes_json, stats_updated_at, stats_stale, stats_source
                 FROM tables",
            )?;
            let rows = stmt.query_map([], |row| {
                let stats_stale: Option<i64> = row.get(10)?;
                Ok(TableMetadata {
                    catalog_name: row.get(0)?,
                    schema_name: row.get(1)?,
                    table_name: row.get(2)?,
                    file_path: row.get(3)?,
                    source_type: row.get(4)?,
                    sheet_name: row.get(5)?,
                    schema_json: row.get(6)?,
                    stats_json: row.get(7)?,
                    indexes_json: row.get(8)?,
                    stats_updated_at: row.get(9)?,
                    stats_stale: stats_stale.map(|v| v != 0),
                    stats_source: row.get(11)?,
                })
            })?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row?);
            }
            Ok(result)
        })
    }

    /// 保存表元数据到 SQLite
    ///
    /// **实现方案**:
    /// 1. 开启事务。
    /// 2. 清空旧版 `tables` 表（用于保持兼容性）。
    /// 3. 遍历 `tables` 列表：
    ///    - 调用 `upsert_schema_version` 更新三层模型（表实例 + 版本）。
    ///    - 插入旧版 `tables` 表。
    /// 4. 提交事务。
    ///
    /// **调用链路**:
    /// - 被 `save_table` 调用。
    ///
    /// **关键问题点**:
    /// - 双写机制：同时维护新旧两套表结构，确保回滚兼容性。
    fn save_tables_sqlite(&self, tables: &[TableMetadata]) -> Result<()> {
        self.with_sqlite_transaction(|tx| {
            let has_pointer = Self::table_instances_has_column_tx(tx, "current_schema_version_id")?;
            tx.execute("DELETE FROM tables", [])?;
            let mut stmt = tx.prepare(
                "INSERT INTO tables (
                    catalog_name, schema_name, table_name, file_path, source_type,
                    sheet_name, schema_json, stats_json, indexes_json,
                    stats_updated_at, stats_stale, stats_source
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for t in tables {
                Self::upsert_schema_version(tx, t, has_pointer)?;
                let stats_stale = t.stats_stale.map(|v| if v { 1 } else { 0 });
                stmt.execute(params![
                    t.catalog_name,
                    t.schema_name,
                    t.table_name,
                    t.file_path,
                    t.source_type,
                    t.sheet_name,
                    t.schema_json,
                    t.stats_json,
                    t.indexes_json,
                    t.stats_updated_at,
                    stats_stale,
                    t.stats_source
                ])?;
            }
            Ok(())
        })
    }

    /// 更新或插入 Schema 版本
    ///
    /// **实现方案**:
    /// 1. 尝试插入 `table_instances` (IGNORE if exists)。
    /// 2. 获取 `table_instance_id`。
    /// 3. 查询该实例的最新版本。
    /// 4. 比较元数据（Schema, Stats, Index）是否发生变化。
    /// 5. 如果变化，插入新版本 (`version + 1`) 到 `schema_versions`，并更新实例指针。
    ///
    /// **调用链路**:
    /// - 被 `save_tables_sqlite` 调用。
    ///
    /// **关键问题点**:
    /// - 版本控制：仅当内容变更时才生成新版本，节省空间。
    /// - 并发安全：依赖 SQLite 事务隔离。
    fn upsert_schema_version(
        tx: &rusqlite::Transaction,
        table_meta: &TableMetadata,
        has_pointer: bool,
    ) -> Result<i64> {
        tx.execute(
            "INSERT OR IGNORE INTO table_instances (
                catalog_name, schema_name, table_name, file_path, source_type, sheet_name
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                table_meta.catalog_name,
                table_meta.schema_name,
                table_meta.table_name,
                table_meta.file_path,
                table_meta.source_type,
                table_meta.sheet_name
            ],
        )?;

        let table_instance_id: i64 = tx.query_row(
            "SELECT id FROM table_instances
             WHERE catalog_name = ?1 AND schema_name = ?2 AND table_name = ?3 AND source_type = ?4",
            params![
                table_meta.catalog_name,
                table_meta.schema_name,
                table_meta.table_name,
                table_meta.source_type
            ],
            |r| r.get(0),
        )?;

        let latest: Option<SchemaVersionRow> = tx
            .query_row(
                "SELECT id, version, schema_json, stats_json, indexes_json,
                        stats_updated_at, stats_stale, stats_source
                 FROM schema_versions
                 WHERE table_instance_id = ?1
                 ORDER BY version DESC
                 LIMIT 1",
                params![table_instance_id],
                |row| {
                    let stats_stale: i64 = row.get(6)?;
                    Ok(SchemaVersionRow {
                        id: row.get(0)?,
                        version: row.get(1)?,
                        schema_json: row.get(2)?,
                        stats_json: row.get(3)?,
                        indexes_json: row.get(4)?,
                        stats_updated_at: row.get(5)?,
                        stats_stale: stats_stale != 0,
                        stats_source: row.get(7)?,
                    })
                },
            )
            .optional()?;

        let target_stats_updated_at = table_meta.stats_updated_at.unwrap_or(0);
        let target_stats_stale = table_meta.stats_stale.unwrap_or(true);
        let target_stats_source = table_meta
            .stats_source
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let version_id = if let Some(latest) = &latest {
            if latest.schema_json == table_meta.schema_json
                && latest.stats_json == table_meta.stats_json
                && latest.indexes_json == table_meta.indexes_json
                && latest.stats_updated_at == target_stats_updated_at
                && latest.stats_stale == target_stats_stale
                && latest.stats_source == target_stats_source
            {
                latest.id
            } else {
                let next_version = latest.version + 1;
                tx.execute(
                    "INSERT INTO schema_versions (
                        table_instance_id, version, schema_json, stats_json, indexes_json,
                        stats_updated_at, stats_stale, stats_source
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        table_instance_id,
                        next_version,
                        table_meta.schema_json,
                        table_meta.stats_json,
                        table_meta.indexes_json,
                        target_stats_updated_at,
                        if target_stats_stale { 1 } else { 0 },
                        target_stats_source
                    ],
                )?;
                tx.last_insert_rowid()
            }
        } else {
            tx.execute(
                "INSERT INTO schema_versions (
                    table_instance_id, version, schema_json, stats_json, indexes_json,
                    stats_updated_at, stats_stale, stats_source
                 ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    table_instance_id,
                    table_meta.schema_json,
                    table_meta.stats_json,
                    table_meta.indexes_json,
                    target_stats_updated_at,
                    if target_stats_stale { 1 } else { 0 },
                    target_stats_source
                ],
            )?;
            tx.last_insert_rowid()
        };

        if has_pointer {
            tx.execute(
                "UPDATE table_instances SET current_schema_version_id = ?1 WHERE id = ?2",
                params![version_id, table_instance_id],
            )?;
        }
        Ok(version_id)
    }

    fn save_connections_to_disk(&self, conns: &[ConnectionMetadata]) -> Result<()> {
        match &self.backend {
            Backend::Parquet { .. } => self.save_connections_parquet(conns),
            Backend::Sqlite { .. } => self.save_connections_sqlite(conns),
        }
    }

    fn save_tables_to_disk(&self, tables: &[TableMetadata]) -> Result<()> {
        match &self.backend {
            Backend::Parquet { .. } => self.save_tables_parquet(tables),
            Backend::Sqlite { .. } => self.save_tables_sqlite(tables),
        }
    }

    // --- Connections API ---

    pub fn save_connection(&self, meta: &ConnectionMetadata) -> Result<()> {
        let mut conns = self
            .connections
            .lock()
            .map_err(|_| MetadataError::LockError)?;

        // Upsert
        if let Some(idx) = conns.iter().position(|c| c.id == meta.id) {
            conns[idx] = meta.clone();
        } else {
            conns.push(meta.clone());
        }

        self.save_connections_to_disk(&conns)
    }

    pub fn list_connections(&self) -> Result<Vec<ConnectionMetadata>> {
        let conns = self
            .connections
            .lock()
            .map_err(|_| MetadataError::LockError)?;
        Ok(conns.clone())
    }

    pub fn get_connection(&self, id: &str) -> Result<Option<ConnectionMetadata>> {
        let conns = self
            .connections
            .lock()
            .map_err(|_| MetadataError::LockError)?;
        Ok(conns.iter().find(|c| c.id == id).cloned())
    }

    pub fn delete_connection(&self, id: &str) -> Result<usize> {
        let mut conns = self
            .connections
            .lock()
            .map_err(|_| MetadataError::LockError)?;
        let initial_len = conns.len();
        conns.retain(|c| c.id != id);
        let deleted = initial_len - conns.len();

        if deleted > 0 {
            self.save_connections_to_disk(&conns)?;
        }
        Ok(deleted)
    }

    // --- Tables API ---

    pub fn save_table(&self, meta: &TableMetadata) -> Result<()> {
        let mut tables = self.tables.lock().map_err(|_| MetadataError::LockError)?;

        // Upsert based on unique constraint (catalog, schema, table, source_type)
        if let Some(idx) = tables.iter().position(|t| {
            t.catalog_name == meta.catalog_name
                && t.schema_name == meta.schema_name
                && t.table_name == meta.table_name
                && t.source_type == meta.source_type
        }) {
            tables[idx] = meta.clone();
        } else {
            tables.push(meta.clone());
        }

        self.save_tables_to_disk(&tables)
    }

    pub fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        let tables = self.tables.lock().map_err(|_| MetadataError::LockError)?;
        Ok(tables.clone())
    }

    pub fn list_schema_versions(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        source_type: &str,
    ) -> Result<Vec<SchemaVersionRecord>> {
        match &self.backend {
            Backend::Parquet { .. } => Ok(Vec::new()),
            Backend::Sqlite { .. } => self.with_sqlite_conn(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT sv.version, sv.schema_json, sv.stats_json, sv.indexes_json,
                            sv.stats_updated_at, sv.stats_stale, sv.stats_source
                     FROM schema_versions sv
                     JOIN table_instances ti ON ti.id = sv.table_instance_id
                     WHERE ti.catalog_name = ?1 AND ti.schema_name = ?2
                       AND ti.table_name = ?3 AND ti.source_type = ?4
                     ORDER BY sv.version",
                )?;
                let rows = stmt.query_map(params![catalog, schema, table, source_type], |row| {
                    let stats_stale: i64 = row.get(5)?;
                    Ok(SchemaVersionRecord {
                        version: row.get(0)?,
                        schema_json: row.get(1)?,
                        stats_json: row.get(2)?,
                        indexes_json: row.get(3)?,
                        stats_updated_at: row.get(4)?,
                        stats_stale: stats_stale != 0,
                        stats_source: row.get(6)?,
                    })
                })?;
                let mut result = Vec::new();
                for row in rows {
                    result.push(row?);
                }
                Ok(result)
            }),
        }
    }

    pub fn rollback_schema_version(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        source_type: &str,
        version: i64,
    ) -> Result<bool> {
        let mut rollback_target: Option<SchemaVersionRow> = None;
        let mut rollback_keys: Option<(String, String, String, String)> = None;

        match &self.backend {
            Backend::Parquet { .. } => return Ok(false),
            Backend::Sqlite { .. } => {}
        }

        self.with_sqlite_transaction(|tx| {
            let table_instance_id: Option<i64> = tx
                .query_row(
                    "SELECT id FROM table_instances
                     WHERE catalog_name = ?1 AND schema_name = ?2
                       AND table_name = ?3 AND source_type = ?4",
                    params![catalog, schema, table, source_type],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(table_instance_id) = table_instance_id else {
                return Ok(());
            };

            let target: Option<SchemaVersionRow> = tx
                .query_row(
                    "SELECT id, version, schema_json, stats_json, indexes_json,
                            stats_updated_at, stats_stale, stats_source
                     FROM schema_versions
                     WHERE table_instance_id = ?1 AND version = ?2
                     LIMIT 1",
                    params![table_instance_id, version],
                    |row| {
                        let stats_stale: i64 = row.get(6)?;
                        Ok(SchemaVersionRow {
                            id: row.get(0)?,
                            version: row.get(1)?,
                            schema_json: row.get(2)?,
                            stats_json: row.get(3)?,
                            indexes_json: row.get(4)?,
                            stats_updated_at: row.get(5)?,
                            stats_stale: stats_stale != 0,
                            stats_source: row.get(7)?,
                        })
                    },
                )
                .optional()?;
            let Some(target) = target else {
                return Ok(());
            };

            tx.execute(
                "UPDATE table_instances
                 SET current_schema_version_id = ?1
                 WHERE id = ?2",
                params![target.id, table_instance_id],
            )?;
            tx.execute(
                "UPDATE tables
                 SET schema_json = ?1,
                     stats_json = ?2,
                     indexes_json = ?3,
                     stats_updated_at = ?4,
                     stats_stale = ?5,
                     stats_source = ?6
                 WHERE catalog_name = ?7 AND schema_name = ?8
                   AND table_name = ?9 AND source_type = ?10",
                params![
                    target.schema_json,
                    target.stats_json,
                    target.indexes_json,
                    target.stats_updated_at,
                    if target.stats_stale { 1 } else { 0 },
                    target.stats_source,
                    catalog,
                    schema,
                    table,
                    source_type
                ],
            )?;

            rollback_target = Some(target);
            rollback_keys = Some((
                catalog.to_string(),
                schema.to_string(),
                table.to_string(),
                source_type.to_string(),
            ));
            Ok(())
        })?;

        if let (Some(target), Some((catalog, schema, table, source_type))) =
            (rollback_target, rollback_keys)
        {
            let mut tables = self.tables.lock().map_err(|_| MetadataError::LockError)?;
            if let Some(table_meta) = tables.iter_mut().find(|t| {
                t.catalog_name == catalog
                    && t.schema_name == schema
                    && t.table_name == table
                    && t.source_type == source_type
            }) {
                table_meta.schema_json = target.schema_json;
                table_meta.stats_json = target.stats_json;
                table_meta.indexes_json = target.indexes_json;
                table_meta.stats_updated_at = Some(target.stats_updated_at);
                table_meta.stats_stale = Some(target.stats_stale);
                table_meta.stats_source = Some(target.stats_source);
            }
            return Ok(true);
        }
        Ok(false)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_table(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        file_path: &str,
        source_type: &str,
        sheet_name: Option<String>,
        cost: Option<u64>,
    ) -> Result<()> {
        let stats_json = cost.map(|c| format!("{{\"num_rows\": {}}}", c));
        self.save_table(&TableMetadata {
            catalog_name: catalog.to_string(),
            schema_name: schema.to_string(),
            table_name: table.to_string(),
            file_path: file_path.to_string(),
            source_type: source_type.to_string(),
            sheet_name,
            schema_json: None,
            stats_json,
            indexes_json: None,
            stats_updated_at: Some(now_timestamp()),
            stats_stale: Some(false),
            stats_source: Some("manual".to_string()),
        })
    }

    pub fn delete_table(&self, catalog: &str, schema: &str, table: &str) -> Result<usize> {
        let mut tables = self.tables.lock().map_err(|_| MetadataError::LockError)?;
        let initial_len = tables.len();
        tables.retain(|t| {
            !(t.catalog_name == catalog && t.schema_name == schema && t.table_name == table)
        });
        let deleted = initial_len - tables.len();

        if deleted > 0 {
            self.save_tables_to_disk(&tables)?;
        }
        Ok(deleted)
    }

    pub fn get_table(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
    ) -> Result<Option<TableMetadata>> {
        let tables = self.tables.lock().map_err(|_| MetadataError::LockError)?;
        Ok(tables
            .iter()
            .find(|t| t.catalog_name == catalog && t.schema_name == schema && t.table_name == table)
            .cloned())
    }
}

fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection as SqliteConnection;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn sqlite_backend_creates_db_and_persists() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("metadata_store_test_{}", nanos));
        fs::create_dir_all(&temp_root).unwrap();
        let db_path = temp_root.join("metadata_test.db");

        let store = MetadataStore::new(db_path.to_string_lossy().as_ref()).unwrap();
        store
            .save_connection(&ConnectionMetadata {
                id: "c1".to_string(),
                name: "conn1".to_string(),
                source_type: "oracle".to_string(),
                config: "{\"host\":\"a\"}".to_string(),
            })
            .unwrap();
        store
            .save_table(&TableMetadata {
                catalog_name: "cat".to_string(),
                schema_name: "sch".to_string(),
                table_name: "tab".to_string(),
                file_path: "fp".to_string(),
                source_type: "oracle".to_string(),
                sheet_name: None,
                schema_json: Some("[{\"name\":\"id\",\"type\":\"int\"}]".to_string()),
                stats_json: None,
                indexes_json: None,
                stats_updated_at: Some(1),
                stats_stale: Some(false),
                stats_source: Some("manual".to_string()),
            })
            .unwrap();
        drop(store);

        assert!(db_path.exists());
        let metadata_data_dir = temp_root.join("metadata_data");
        assert!(!metadata_data_dir.exists());

        let store2 = MetadataStore::new(db_path.to_string_lossy().as_ref()).unwrap();
        let conns = store2.list_connections().unwrap();
        let tables = store2.list_tables().unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(tables.len(), 1);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn run_ci_script_creates_log_entry() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let log_path = repo_root.join("test").join("test_runs.jsonl");
        let _ = fs::remove_file(&log_path);

        let script_path = repo_root.join("test").join("run_ci.ps1");
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(script_path)
            .arg("-Commands")
            .arg("powershell -Command \"exit 0\"")
            .status()
            .unwrap();

        assert!(status.success());
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test_runs.jsonl") == false);
        assert!(content.contains("powershell -Command"));
    }

    #[test]
    fn sqlite_migrates_old_tables_to_three_layer() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("metadata_store_migrate_{}", nanos));
        fs::create_dir_all(&temp_root).unwrap();
        let db_path = temp_root.join("metadata_legacy.db");

        {
            let conn = SqliteConnection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE connections (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    config TEXT NOT NULL
                );
                CREATE TABLE tables (
                    catalog_name TEXT NOT NULL,
                    schema_name TEXT NOT NULL,
                    table_name TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    sheet_name TEXT,
                    schema_json TEXT,
                    stats_json TEXT,
                    indexes_json TEXT,
                    stats_updated_at INTEGER NOT NULL DEFAULT 0,
                    stats_stale INTEGER NOT NULL DEFAULT 1,
                    stats_source TEXT NOT NULL DEFAULT 'unknown',
                    UNIQUE(catalog_name, schema_name, table_name, source_type)
                );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO connections (id, name, source_type, config) VALUES (?1, ?2, ?3, ?4)",
                params!["c1", "conn1", "oracle", "{\"host\":\"a\"}"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO tables (
                    catalog_name, schema_name, table_name, file_path, source_type,
                    sheet_name, schema_json, stats_json, indexes_json,
                    stats_updated_at, stats_stale, stats_source
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    "cat",
                    "sch",
                    "tab",
                    "fp",
                    "oracle",
                    Option::<String>::None,
                    Some("[{\"name\":\"id\",\"type\":\"int\"}]"),
                    Option::<String>::None,
                    Option::<String>::None,
                    1,
                    0,
                    "manual"
                ],
            )
            .unwrap();
        }

        let _ = MetadataStore::new(db_path.to_string_lossy().as_ref()).unwrap();
        let conn = SqliteConnection::open(&db_path).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master
                 WHERE type='table' AND name IN ('table_instances','schema_versions')",
            )
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(names.len(), 2);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn parquet_migrates_into_sqlite_three_layer() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("metadata_store_parquet_{}", nanos));
        fs::create_dir_all(&temp_root).unwrap();
        let parquet_root = temp_root.join("metadata_data");
        let sqlite_path = temp_root.join("metadata_new.db");

        {
            let parquet_store =
                MetadataStore::new(parquet_root.to_string_lossy().as_ref()).unwrap();
            parquet_store
                .save_connection(&ConnectionMetadata {
                    id: "c1".to_string(),
                    name: "conn1".to_string(),
                    source_type: "oracle".to_string(),
                    config: "{\"host\":\"a\"}".to_string(),
                })
                .unwrap();
            parquet_store
                .save_table(&TableMetadata {
                    catalog_name: "cat".to_string(),
                    schema_name: "sch".to_string(),
                    table_name: "tab".to_string(),
                    file_path: "fp".to_string(),
                    source_type: "oracle".to_string(),
                    sheet_name: None,
                    schema_json: Some("[{\"name\":\"id\",\"type\":\"int\"}]".to_string()),
                    stats_json: None,
                    indexes_json: None,
                    stats_updated_at: Some(1),
                    stats_stale: Some(false),
                    stats_source: Some("manual".to_string()),
                })
                .unwrap();
        }

        let sqlite_store = MetadataStore::new(sqlite_path.to_string_lossy().as_ref()).unwrap();
        let tables = sqlite_store.list_tables().unwrap();
        assert_eq!(tables.len(), 1);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn schema_versions_append_and_rollback_pointer() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("metadata_store_version_{}", nanos));
        fs::create_dir_all(&temp_root).unwrap();
        let db_path = temp_root.join("metadata_version.db");

        let store = MetadataStore::new(db_path.to_string_lossy().as_ref()).unwrap();
        store
            .save_table(&TableMetadata {
                catalog_name: "cat".to_string(),
                schema_name: "sch".to_string(),
                table_name: "tab".to_string(),
                file_path: "fp".to_string(),
                source_type: "oracle".to_string(),
                sheet_name: None,
                schema_json: Some("v1".to_string()),
                stats_json: None,
                indexes_json: None,
                stats_updated_at: Some(1),
                stats_stale: Some(false),
                stats_source: Some("manual".to_string()),
            })
            .unwrap();
        store
            .save_table(&TableMetadata {
                catalog_name: "cat".to_string(),
                schema_name: "sch".to_string(),
                table_name: "tab".to_string(),
                file_path: "fp".to_string(),
                source_type: "oracle".to_string(),
                sheet_name: None,
                schema_json: Some("v2".to_string()),
                stats_json: None,
                indexes_json: None,
                stats_updated_at: Some(2),
                stats_stale: Some(false),
                stats_source: Some("manual".to_string()),
            })
            .unwrap();

        let versions = store
            .list_schema_versions("cat", "sch", "tab", "oracle")
            .unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].schema_json.as_deref(), Some("v1"));
        assert_eq!(versions[1].schema_json.as_deref(), Some("v2"));

        let rolled_back = store
            .rollback_schema_version("cat", "sch", "tab", "oracle", 1)
            .unwrap();
        assert!(rolled_back);

        let tables = store.list_tables().unwrap();
        assert_eq!(tables[0].schema_json.as_deref(), Some("v1"));
        let versions = store
            .list_schema_versions("cat", "sch", "tab", "oracle")
            .unwrap();
        assert_eq!(versions.len(), 2);

        let _ = fs::remove_dir_all(&temp_root);
    }
}
