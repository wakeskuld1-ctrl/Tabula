# RFC 001: 统一元数据与写优化引擎架构演进
# Architecture Evolution RFC: Unified Metadata & Write-Optimized Engine

**Status:** DRAFT
**Date:** 2026-02-01
**Author:** System Architect
**Context:** [ARCHITECTURAL_RISK_ANALYSIS.md](./ARCHITECTURAL_RISK_ANALYSIS.md)

## 1. 问题陈述 (Problem Statement)

经过对 Tabula 代码库的深度审计，确认当前架构存在阻碍 "Create Table" 和 "Cross-Sheet Formulas" 功能落地的严重风险。

### 1.1 "脑裂" 风险 (Split-Brain Risk)
目前元数据分散在三个独立的存储中，缺乏事务一致性保障：
*   **SQLite (`metadata_manager.rs`)**: 存储表的注册信息 (`register_table`)。
*   **JSON (`session_manager/mod.rs`)**: 在 `sessions.json` 中存储 Session 列表、样式 (Styles) 和合并单元格 (Merges)。
*   **Lance (`FileSystem`)**: 存储实际数据。

**代码证据:**
*   `SessionManager::persist_sessions` (L96) 直接覆写 JSON 文件，与 `MetadataManager` 的 SQLite 操作无任何关联。
*   如果 `Dataset::write` 成功但 JSON 写入失败，会导致数据“孤儿化”。

### 1.2 内存管理不可控 (Unbounded Memory Usage)
*   **现状:** `SessionManager` 使用 `Vec<RecordBatch>` (L55) 在内存中持有整个 Session 的数据。
*   **代码证据:** `checkout_version` (L459) 调用 `try_collect::<Vec<RecordBatch>>()` 将整个数据集加载到 RAM。
*   **后果:** 打开多个大表格将直接导致 OOM (Out of Memory)。`CacheManager` 目前仅管理查询缓存，无法感知 Session 的内存压力。

---

## 2. 演进方案 (Proposed Solution)

### 2.1 方案一：统一元数据存储 (Unified Metadata Store)

**目标:** 废弃 `sessions.json`，将 Session 管理和 Sheet 元数据下沉到 SQLite，利用 SQL 事务保障一致性。

#### 2.1.1 SQLite Schema 变更
在 `metadata.db` 中新增两张表：

```sql
-- 1. Session 注册表 (替代 sessions.json 的结构)
CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY,
    table_name TEXT NOT NULL, -- 关联 tables 表
    friendly_name TEXT,       -- 原 name 字段
    lance_path TEXT NOT NULL,
    created_at INTEGER,
    is_default BOOLEAN DEFAULT 0,
    parent_session_id TEXT,   -- 支持 Fork 溯源
    FOREIGN KEY(table_name) REFERENCES tables(name) ON DELETE CASCADE
);

-- 2. Sheet 属性表 (替代 SheetMetadata)
-- 使用 EAV (Entity-Attribute-Value) 模型存储稀疏的样式数据
CREATE TABLE sheet_attributes (
    session_id TEXT NOT NULL,
    cell_key TEXT NOT NULL,   -- "2,3" (Row,Col) 或 "global"
    attr_type TEXT NOT NULL,  -- "style", "merge", "col_width"
    attr_value TEXT,          -- JSON String: {"bold": true, "bg_color": "#FF0000"}
    PRIMARY KEY (session_id, cell_key, attr_type),
    FOREIGN KEY(session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);
```

#### 2.1.2 改造 `SessionManager`
*   **移除:** `sessions: Mutex<HashMap<String, SessionInfo>>` 内存缓存。
*   **新增:** `metadata_store: Arc<MetadataStore>` 引用。
*   **操作流:**
    *   `list_sessions(table)`: `SELECT * FROM sessions WHERE table_name = ?`.
    *   `update_style(...)`: `INSERT OR REPLACE INTO sheet_attributes ...`.

### 2.2 方案二：引入 Buffer Manager (内存治理)

**目标:** 解耦 `Session` 与 `Data`。`Session` 仅持有元数据引用，数据页按需加载并受 LRU 限制。

#### 2.2.1 新增组件 `BufferManager`

```rust
pub struct BufferManager {
    // 全局 LRU 缓存：Key = (SessionID, BatchIndex), Value = RecordBatch
    // 限制总内存大小 (e.g., 512MB)
    cache: Arc<ShardedLruCache<(String, usize), RecordBatch>>,
    
    // 脏页追踪 (用于写优化)
    dirty_pages: DashMap<(String, usize), RecordBatch>
}

impl BufferManager {
    /// 获取数据页。如果不在内存，从 Lance 读取。
    pub async fn get_batch(&self, session_id: &str, batch_idx: usize) -> Result<RecordBatch> {
        // 1. Check Cache
        // 2. If Miss -> Load from Lance (Range Scan) -> Insert Cache
    }
    
    /// 标记页面为脏 (用户编辑)
    pub fn mark_dirty(&self, session_id: &str, batch_idx: usize, data: RecordBatch) {
        self.dirty_pages.insert((session_id.to_string(), batch_idx), data);
    }
    
    /// 刷盘 (Save/Checkpoint)
    pub async fn flush_session(&self, session_id: &str) -> Result<String> {
        // 将脏页和未修改页合并，写入新的 Lance Version
        // 更新 SQLite 中的 session 指针
    }
}
```

### 2.3 方案三：事务性 Create Table (Atomic Create)

**目标:** 实现安全的 `create_table` 接口。

#### 2.3.1 SAGA 流程
1.  **Phase 1 (Prepare):**
    *   在临时目录 `data/staging/{uuid}` 创建 Lance 数据集。
    *   写入初始数据 (Empty Schema or Imported Data)。
2.  **Phase 2 (Local Commit):**
    *   `fs::rename("data/staging/{uuid}", "data/tables/{table_name}")`.
3.  **Phase 3 (Meta Commit):**
    *   `BEGIN TRANSACTION`
    *   `INSERT INTO tables ...`
    *   `INSERT INTO sessions ...` (Create default session)
    *   `COMMIT`
4.  **Rollback:**
    *   如果 Phase 3 失败 -> `fs::remove_dir_all("data/tables/{table_name}")` -> 返回错误。

---

## 3. 实施计划 (Implementation Plan)

### Phase 1: 元数据迁移 (Immediate)
1.  在 `metadata_manager` 中添加 `sessions` 和 `sheet_attributes` 表创建 SQL。
2.  编写 `migration_tool`：读取 `sessions.json` -> 写入 SQLite -> 重命名 `sessions.json` 为 `sessions.json.bak`。
3.  重构 `SessionManager` 使用 SQLite。

### Phase 2: 内存安全 (Next Step)
1.  创建 `BufferManager` struct。
2.  修改 `SessionInfo`，移除 `current_data` 字段。
3.  修改 `ExcelDataSource` (DataFusion Provider) 通过 `BufferManager` 读取数据。

### Phase 3: 功能开发 (Features)
1.  实现 `create_table` API。
2.  实现跨 Sheet 公式解析 (依赖 `BufferManager` 按需加载其他 Sheet 数据)。

---

## 4. 结论

采用此架构将使系统从简单的“文件阅读器”转变为真正的“分析型数据库”。虽然短期内重构工作量较大（特别是移除 `sessions.json`），但这消除了数据丢失的根本风险，并为处理百万行级数据奠定了内存基础。
