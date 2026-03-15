# SessionId Align + Warnings Cleanup Design

**Date:** 2026-03-15

## 背景与目标
前端 TimeMachineDrawer 访问 `/api/versions` 时未携带 `session_id`，与“有 session_id 用指定会话；无则用当前活动会话”的规则不一致。现要求：
- `sessionId` 非空时带 `session_id`
- `sessionId` 为空时明确传 `session_id: null`

同时需要清理现有编译 warnings（不改变行为）。

## 设计原则
- 行为保持：不改变现有业务语义，仅补齐参数与归一化。
- 低侵入：最小改动覆盖完整链路。
- 可测试：新增回归测试覆盖 `session_id=null` 情况。

## 方案（已选）
**方案A（推荐）**
- 前端 `TimeMachineDrawer` 接收 `sessionId`，请求 `/api/versions` 时总携带 `session_id` 参数：
  - 非空：`session_id=<value>`
  - 为空：`session_id=null`
- 后端 `/api/versions` 归一化 `session_id`（`"null"`/空字符串 -> None）
- 新增一条后端集成测试覆盖 `session_id=null`
- 清理 warnings（unused import、unused variable、deprecated chrono 调用）

## 前端改动
- 文件：`frontend/src/components/TimeMachineDrawer.tsx`
  - 新增 `sessionId?: string` props
  - 请求 URL 拼接 `session_id` 参数
- 文件：`frontend/src/App.tsx`
  - 传入 `sessionId` 到 TimeMachineDrawer

## 后端改动
- 文件：`federated_query_engine/src/api/version_handler.rs`
  - 将 `session_id` 归一化为 Option<&str>
- 文件：`federated_query_engine/tests/api_integration_test.rs`
  - 新增 `/api/versions` 使用 `session_id=null` 的回归测试
- 警告清理（不改行为）
  - `federated_query_engine/src/services/register_service.rs`：移除未使用 import
  - `federated_query_engine/src/session_manager/mod.rs`：移除未使用 Timestamp 引用
  - `federated_query_engine/src/lib.rs`：移除未使用 axum 导入
  - `federated_query_engine/src/api/session_handler.rs`：未使用参数改为 `_state`
  - chrono deprecated 调用改用 `.and_utc().timestamp_*`

## 数据流与规则
- 前端发起：`/api/versions?table_name=...&session_id=<value|null>`
- 后端解析：
  - `session_id == "null" || session_id == ""` -> None
  - None -> 使用当前活动会话
  - Some -> 按指定会话查询版本

## 测试策略
- **TDD**：先新增集成测试（期望 status=ok），确认失败后再实现。
- 测试用例：`session_id=null` 仍可返回 versions 列表。
- 编译检查：`cargo test -p federated_query_engine --no-run`

## 风险与缓解
- 风险：前端传 `session_id=null` 仍被后端当作有效值导致找不到会话。
  - 缓解：后端归一化处理。
- 风险：warnings 清理误改行为。
  - 缓解：仅移除未使用导入、替换等价 API。

---

**结论：** 采用方案A，先对齐前端 session_id 入参，再补回归测试与清理 warnings。
