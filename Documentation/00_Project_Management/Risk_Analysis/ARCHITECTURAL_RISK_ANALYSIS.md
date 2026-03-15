# 架构风险分析：新增数据表与高级公式引擎集成

**日期:** 2026-02-01
**分析对象:** 后端存储架构 (Rust/DataFusion/Lance) 与 前端交互 (Excel/GlideGrid)
**目标:** 评估实现 "Create Table" 及 "Cross-Sheet Formulas" 的技术风险与架构演进方向。

## 1. 背景与现状 (Context)

当前系统架构主要是一个 **Tabula 查询引擎**，侧重于**读**能力：
*   **元数据 (Metadata):** 使用 SQLite (`metadata.db`) 存储表结构、文件路径。
*   **数据源:** 支持 CSV, Excel, Parquet, SQLite。
*   **会话管理 (Session):** 使用 Lance 进行多版本管理，但元数据（样式、合并单元格）存储在 `sessions.json`。
*   **缓存:** `CacheManager` 负责文件转 Parquet 的读缓存及 TTL 管理。

**新需求** 将系统推向 **写密集型 (Write-Heavy)** 的 DBMS 方向：
1.  **创建表:** 直接生成 Parquet/Lance 文件。
2.  **高级公式:** 跨文件/跨表的随机读写依赖。

---

## 2. 核心风险分析 (Core Risks)

### 2.1 "脑裂" 风险：元数据存储分散 (The Triple-Store Problem)
目前系统存在三个“真理来源” (Source of Truth)，导致一致性维护极其困难：
1.  **SQLite**: 存储表的全局注册信息 (Catalog)。
2.  **JSON (sessions.json)**: 存储样式、合并单元格、Session 列表。
3.  **Lance/Parquet**: 存储实际数据与 Schema。

**风险场景:**
*   **创建失败:** 如果在 SQLite 注册了表，但 Lance 文件创建失败（如磁盘满），系统会认为表存在但无法查询。
*   **Schema 漂移:** 在 Excel 中新增一列，Lance 文件更新了 Schema，但 SQLite 中的 `schema_json` 未同步，导致查询引擎优化器失效。
*   **原子性缺失:** 重命名表时，需要同时更新 SQLite, JSON 和 文件系统。任何一步失败都会导致状态不一致。

### 2.2 内存管理失控 (Memory Pressure)
*   **现状:** `SessionManager` 将编辑中的数据 (`current_data`) 以 `Vec<RecordBatch>` 形式保存在内存中。
*   **风险:**
    *   **LRU 失效:** 现有的 `CacheManager` 主要管理 *读缓存* (Parquet转换文件)，无法感知或控制 `SessionManager` 的内存占用。
    *   **OOM (内存溢出):** 如果用户创建了 10 个 Sheet，每个 Sheet 都有百万行数据，且都在内存中，服务器将崩溃。
    *   **冷启动慢:** 跨 Sheet 公式计算时，如果依赖的 Sheet 不在内存中，需要实时加载 Lance 文件，可能导致前端 UI 卡顿。

### 2.3 跨 Sheet 公式的依赖地狱 (Dependency Hell)
*   **现状:** 公式引擎目前主要在前端 (JS) 运行，或后端简单的单表计算。
*   **风险:**
    *   **死锁/循环引用:** Sheet A 引用 Sheet B，Sheet B 引用 Sheet A。
    *   **文件锁 (Windows):** 当公式引擎读取 Sheet B 的 Parquet 文件进行计算时，如果此时用户正尝试保存/写入 Sheet B，Windows 的文件锁机制可能导致写入失败 (`Permission Denied`)。
    *   **脏读:** 公式计算是基于磁盘上的旧版本，还是内存中未保存的新版本？如果没有统一的 "Buffer Manager"，计算结果可能不准确。

---

## 3. 架构演进建议 (Recommendations)

### 3.1 统一元数据管理 (Unified Metadata Strategy)
**建议废弃 `sessions.json`，将所有元数据收敛至 SQLite 和 Lance。**

*   **方案 A (推荐):** 将样式 (Styles) 和合并 (Merges) 作为 Lance 文件的 **Metadata** 存储。
    *   *优势:* 数据与样式同生共死，回滚版本时样式自动回滚。
    *   *实现:* Lance 支持 key-value metadata，可存储压缩后的 JSON。
*   **方案 B:** 将样式存储在 SQLite 的新表 `sheet_attributes` 中。
    *   *优势:* 支持 SQL 查询样式，便于统计。
    *   *劣势:* 回滚 Lance 版本时，需要手动回滚 SQLite 中的样式数据，增加了事务复杂性。

### 3.2 实现 "Buffer Manager" (统一缓冲管理)
不要让 `SessionManager` 独立持有内存数据。引入数据库级别的 Buffer Manager 概念：

*   **LRU 策略:** 所有 Sheet 的数据页（RecordBatch）统一由 CacheManager 管理。
*   **Spill-to-Disk:** 当内存压力大时，强制将“脏页”（未保存的编辑）写入临时的 Lance 版本（Checkpoint），并释放内存。
*   **公式引用:** 公式引擎通过 Buffer Manager 请求数据。如果数据在内存，直接返回；如果不在，透明加载。

### 3.3 事务性创建表 (Transactional Create)
实现 `create_table` 接口时，必须采用 **SAGA 模式** 或 **两阶段提交** 的简化版：

1.  **Prepare:** 在临时目录创建 Lance/Parquet 文件。
2.  **Commit:**
    *   原子重命名文件到正式目录。
    *   在 SQLite 开启事务，写入元数据。
    *   (可选) 写入 SQLite 成功提交事务。
3.  **Rollback:** 任何一步失败，清理临时文件，回滚 SQLite 事务。

### 3.4 针对 Parquet/Lance 的写优化
*   **新建表默认使用 Lance:** Parquet 是列式存储，一旦生成即不可变 (Immutable)，修改一行需要重写整个 Row Group。Lance 支持版本控制和追加写入，更适合作为 "Excel 的后端"。
*   **导出时转 Parquet:** 仅在用户需要“导出文件”或“归档”时，将 Lance 转为 Parquet。

---

## 4. 实施路线图 (Roadmap)

1.  **Phase 1: 稳定性加固**
    *   将 `sessions.json` 的内容迁移至 SQLite 或 Lance Metadata。
    *   引入 `Result<Transaction>` 机制，确保 SQLite 和 文件操作的原子性。

2.  **Phase 2: 内存管理升级**
    *   改造 `SessionManager`，使其接入 `CacheManager` 的 LRU 淘汰机制。
    *   实现“自动保存 (Auto-Save)”策略，避免内存堆积。

3.  **Phase 3: 高级功能**
    *   实现 `create_table` API (基于 Lance)。
    *   开发后端公式解析器，支持 `REF(Table, Cell)` 语法，通过 Buffer Manager 获取数据。
