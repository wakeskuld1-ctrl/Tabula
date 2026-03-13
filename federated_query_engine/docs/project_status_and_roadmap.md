# 联邦查询引擎项目状态与规划 (Project Status & Roadmap)

**更新日期**: 2026-01-30
**当前阶段**: 功能完善与优化 (Optimization & Hardening)

## 1. 项目背景 (Context)
本项目旨在构建一个基于 Apache Arrow DataFusion 的联邦查询引擎，支持异构数据源（YashanDB, Oracle, SQLite, CSV/Parquet）的统一查询。核心目标是实现高性能的跨源查询、智能缓存（Sidecar Pattern）以及基于代价的查询优化（CBO）。

## 2. 已完成功能 (Completed Features)

### 2.1 核心引擎与架构
- [x] **DataFusion 集成**: 升级至 DataFusion 52.1.0，适配新的 `FileScanConfig` 和 `ParquetFormat` API。
- [x] **元数据管理**: 实现 `MetadataManager`，支持元数据持久化存储（Parquet格式），支持动态注册/注销数据源。
- [x] **Sidecar 缓存机制**:
    - 实现 YashanDB/Oracle 的后台异步缓存（`LIMIT 100` 快速响应 + 全量后台抓取）。
    - 采用 Parquet 作为本地缓存格式，支持 LRU 淘汰策略。
- [x] **前端可视化**:
    - 实现执行计划的可视化展示（基于 Mermaid），支持中文算子映射。
    - 实现分层数据源视图（Lazy Loading）。

### 2.2 数据源适配
- [x] **YashanDB**: 支持 ODBC 连接，实现谓词下推（Predicate Pushdown）。
- [x] **Oracle**: 支持直连及 ODBC，实现 `EXPLAIN PLAN` 统计信息获取。
- [x] **文件源**: 支持 CSV（自动编码转换 GBK->UTF8）、Excel、Parquet。

### 2.3 查询优化 (Optimization)
- [x] **单源下推 (Single Source Pushdown)**:
    - 当查询涉及的所有表均来自同一数据源时，绕过 DataFusion 本地执行，直接生成 SQL 下推至远端数据库。
    - 解决了本地 Schema 推断不一致（如大小写、字段顺序）导致的执行错误。
- [x] **基于代价的源选择 (CBO - Source Selection)**:
    - 针对同名表存在于多个数据源的情况（如 Oracle 和 YashanDB 都有 `tpcc.warehouse`）。
    - 实现基于 `NUM_ROWS`（行数）的代价比较，自动选择数据量最小或网络代价最低的源。
- [x] **统计信息收集**:
    - **YashanDB**: 通过 `EXPLAIN PLAN` 获取 `CARDINALITY`。
    - **Oracle**: 实现 `EXPLAIN PLAN` 机制，获取精确的 `CARDINALITY` 和 `BYTES`，提供给 DataFusion 优化器。
- [x] **源路由 (Source Routing)**:
    - 解决表名冲突问题（如多个源都有 `TPCC` Schema）。
    - 实现 SQL 重写逻辑，将逻辑表名（`tpcc.table`）动态映射为物理表名（`oracle_tpcc_table`）。

### 2.4 特性功能
- [x] **智能零拷贝链接 (Smart Zero-Copy Linking)**:
    - 识别内容完全一致的表（通过 `EXCEPT` 校验），在不同数据源间共享同一份本地 Parquet 缓存，减少冗余存储和网络传输。

## 3. 待办事项 (Pending Tasks)

### 3.1 验证与测试
- [ ] **TPCC Join 全链路验证**: 在真实环境下验证 TPCC 复杂 Join 查询的执行计划，确保 CBO 正确选择了有数据的源，且生成的 SQL 符合预期。
- [ ] **端到端自动化测试**: 完善 `tests/e2e_simulation.rs`，覆盖从注册、缓存、查询到下推的完整流程。
- [ ] **性能基准测试**: 对比 "本地执行" vs "下推执行" 的性能差异，校准 CBO 的代价公式参数。

### 3.2 功能增强
- [ ] **更精细的 CBO**: 目前仅基于行数。未来引入网络带宽、Filter 选择率等更细粒度的代价因子。
- [ ] **缓存一致性增强**: 目前基于文件名哈希。需要引入基于时间戳或 Checksum 的缓存失效机制。
- [ ] **错误处理**: 增强对 ODBC 连接断开的自动重连和错误提示。

### 3.3 代码维护
- [ ] **清理临时代码**: 移除 `query_rewriter.rs` 中的临时单元测试，迁移至 `tests/` 目录。
- [ ] **依赖治理**: 检查并移除未使用的 Rust 依赖 crate。

## 4. 已知问题 (Known Issues)
- **Schema 大小写敏感性**: DataFusion 对字段名大小写敏感，而 Oracle/YashanDB 通常大写。目前通过 "全 SQL 下推" 规避了此问题，但混合源查询（Cross-Source Join）仍可能遇到。
- **ODBC 驱动兼容性**: 在某些 Windows 环境下，YashanDB ODBC 驱动可能出现连接字符串解析错误（已通过特定格式规避）。

---
**备注**: 本文档旨在汇总项目当前状态，便于后续开发查阅。请勿直接删除，建议在此基础上追加更新。
