# 联邦查询引擎与在线 Excel 系统架构设计方案

## 1. 核心挑战与目标

本系统旨在解决以下核心技术挑战：
*   **复杂数据交互**：支持数据库/文件的增删改查（CRUD）。
*   **大文件传输**：高效处理 GB 级 Excel/CSV/Parquet 文件。
*   **内存限制**：在有限内存（如 8GB/16GB）下处理超大数据集。
*   **实时性与一致性**：多人协作或并发修改时的数据一致性。

---

## 2. 架构设计 (Architecture)

### 2.1 总体架构
*   **前端 (Frontend)**: React + FortuneSheet/Univer。采用**虚拟滚动 (Virtual Scrolling)** 技术，仅渲染可视区域数据，避免浏览器崩溃。
*   **后端 (Backend)**: Rust (Axum) + DataFusion (查询引擎) + SQLite (元数据存储)。
*   **缓存层 (Cache)**: 两级缓存策略（内存 + 本地磁盘 Parquet），配合 LRU/TTI 淘汰机制。

---

## 3. 详细解决方案

### 3.1 查看与大文件加载 (Viewing & Large Files)
**挑战**: 用户打开一个 100MB 的 Excel（100万行），浏览器直接加载会 OOM（内存溢出）。

**方案**: **视窗加载 (Windowing) + 分页查询**
1.  **后端**: 不一次性返回所有数据。提供 `/api/execute` 接口，支持 `LIMIT` 和 `OFFSET`。
    *   SQL: `SELECT * FROM big_table LIMIT 100 OFFSET 5000`
2.  **前端**:
    *   初始化时，只请求前 100 行数据用于首屏展示。
    *   监听滚动条事件。当用户滚动到底部时，动态请求下一页数据（Lazy Loading）。
    *   对于 FortuneSheet/Univer，使用其提供的 `loadData` 钩子，按需填充数据。

### 3.2 大文件上传与传输 (Large File Upload)
**挑战**: 上传大文件容易超时或占用过多服务器内存。

**方案**: **流式传输 (Streaming) + 分片上传**
1.  **流式上传**: 前端使用 `FormData` 流式上传，后端 Axum 使用 `Multipart` 流式读取，直接写入磁盘临时文件，**不将整个文件加载到 RAM**。
2.  **转码优化**: 上传完成后，后端后台任务将 Excel/CSV 转换为 **Parquet** 格式。
    *   Parquet 是列式存储，压缩率高，读取速度快，且支持“投影推下”（只读需要的列）。

### 3.3 数据保存与更新 (Saving & Updating)
**挑战**: 全量保存太慢；并发修改可能覆盖数据。

**方案**: **单元格级增量更新 (Cell-Level Incremental Update)**
1.  **编辑模式**:
    *   用户修改单元格 `(Row: 5, Col: C)`。
    *   前端不立即发送请求，而是记录在 `dirty_cells` 队列中。
    *   **防抖 (Debounce)**: 用户停止输入 500ms 后，批量发送修改请求。
2.  **后端处理**:
    *   API: `POST /api/update_cells`
    *   Payload: `[{ row_id: 101, col: "age", value: 30 }]`
    *   **实现**: DataFusion 本身不支持直接 Update Parquet。需要：
        *   **策略 A (小数据)**: 内存更新，定期 Flush 到磁盘。
        *   **策略 B (大数据)**: 使用 **Delta Lake** 思想，写入增量日志 (Delta Log)。读取时合并 (Merge on Read)。

### 3.4 内存限制管理 (Memory Management)
**挑战**: 服务器内存有限，不能加载 TB 级数据。

**方案**: **DataFusion 内存池 + 磁盘溢出 (Spilling)**
1.  **执行计划配置**: 配置 DataFusion 的 `MemoryLimit`。当排序/聚合操作使用的内存超过阈值（如 4GB）时，自动将中间结果**溢写到磁盘 (Spill to Disk)**。
2.  **批处理 (Batching)**: 所有的读取和计算都以 `RecordBatch` (默认 8192 行) 为单位流式处理。

---

## 4. 实施路线图 (Roadmap)

### 阶段一：基础浏览 (已完成/进行中)
- [x] 文件上传与注册
- [x] SQL 查询执行
- [x] 基础前端展示 (FortuneSheet)
- [x] 简单的全量加载 (当前瓶颈)

### 阶段二：性能优化 (接下来的重点)
- [ ] **分页接口改造**: 后端支持基于游标的分页。
- [ ] **前端虚拟滚动对接**: 对接 FortuneSheet 的数据懒加载接口。
- [ ] **流式上传优化**: 确保 1GB 文件上传不崩。

### 阶段三：可编辑与持久化
- [ ] **主键识别**: 必须确定每行的唯一标识（RowID）。
- [ ] **增量更新 API**: 实现单元格级别的更新逻辑。
- [ ] **写回策略**: 将内存修改写回 CSV/Excel/Parquet 源文件。
