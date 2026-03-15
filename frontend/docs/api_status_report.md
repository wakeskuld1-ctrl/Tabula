# 前端接口与 Mock 状态梳理报告

**日期**: 2026-03-12
**状态**: 持续优化中（接口连通性已明显提升）
**目标**: 移除前端 Mock 逻辑，全面对接到后端真实接口。

---

## 1. 现状总结

目前系统处于 **“核心链路已打通、剩余接口待补齐”的状态**：
1.  **读数据**：已打通真实后端（SQL 与 `grid-data` 双路径可用）。
2.  **写数据**：`update_cell`/`batch_update_cells`/`update_style` 已具备可用链路并有自动化验证。
3.  **管理功能**：仍有少量结构类接口未实现（`update_style_range`/`insert-column`/`delete-column`）。

---

## 2. 接口详细清单

### ✅ 已对接真实接口 (Direct Backend / SQL)
这些接口直接调用后端的 `/api/execute` 执行 SQL，或调用其他已注册的基础接口，数据是真实的。

| 接口/功能 | 请求路径 | 状态 | 说明 | 代码位置 |
| :--- | :--- | :--- | :--- | :--- |
| **获取表格数据** | `/api/execute` | ✅ 真实 | 通过 `SELECT * ... LIMIT ...` 分页获取数据 | `GridAPI.ts` |
| **获取总行数** | `/api/execute` | ✅ 真实 | 通过 `SELECT COUNT(*)` 获取 | `GridAPI.ts` |
| **获取筛选值** | `/api/execute` | ✅ 真实 | 通过 `SELECT DISTINCT ...` 获取列值 | `GridAPI.ts` |
| **透视表查询** | `/api/execute` | ✅ 真实 | 通过 SQL 聚合查询生成透视数据 | `PivotEngine.ts` |
| **健康检查** | `/api/health` | ✅ 真实 | 检查后端存活状态 | `App.tsx` |
| **获取表列表** | `/api/tables` | ✅ 真实 | 列出数据库中的所有表 | `App.tsx` |
| **删除表 (Fallback)** | `/api/execute` | ✅ 真实 | 当 `/api/delete_table` 失败时，尝试执行 `DROP TABLE` | `App.tsx` |

### ⚠️ 前端 Mock 接口 (历史状态，需持续核对)
以下内容为历史梳理项，当前后端侧对应接口已可用；前端是否完全移除旧 Mock 分支仍需持续回归确认。

| 接口/功能 | 模拟方式 | 状态 | 待办动作 | 代码位置 |
| :--- | :--- | :--- | :--- | :--- |
| **更新单元格** | 历史为 `setTimeout` | ⚠️ 待持续核对前端分支 | 对接 `/api/update_cell` 并回归验证 UI 真实落库 | `GridAPI.ts` |
| **批量更新** | 历史为 `setTimeout` | ⚠️ 待持续核对前端分支 | 对接 `/api/batch_update_cells` 并回归验证聚合结果 | `GridAPI.ts` |
| **样式更新** | 历史为内存更新 | ⚠️ 待持续核对前端分支 | 对接 `/api/update_style` 并回归验证 metadata 回读 | `GlideGrid.tsx` |

### ❌ 无效/缺失接口 (需后端补充)
以下接口中，部分已完成路由注册并可用；仍有少数接口保持未实现状态。

| 接口路径 | 功能描述 | 现状 | 优先级 |
| :--- | :--- | :--- | :--- |
| `/api/create_session` | 创建沙盘/会话 | ✅ 已注册并可用 | High |
| `/api/delete_table` | 删除数据表 | ✅ 已注册并可用 | Medium |
| `/api/save_session` | 保存当前会话 | ✅ 已注册并可用 | High |
| `/api/update_style` | 更新单元格样式 | ✅ 已注册并可用 | Medium |
| `/api/update_style_range` | 更新区域样式 | ❌ 后端未注册 | Low |
| `/api/insert-column` | 插入列 | ❌ 后端未注册 | Low |
| `/api/delete-column` | 删除列 | ❌ 后端未注册 | Low |
| `/api/grid-data` | 获取网格数据 | ✅ 已注册并可用 | - |

---

## 2.1 🧪 已测试通过接口（自动化）

基于 `federated_query_engine/tests/api_integration_test.rs` 自动化测试，以下接口已通过。当前覆盖已从“路由可达性 + 错误路径”扩展到“正向业务链路”。

| 接口路径 | 测试场景 | 结果 | 备注 |
| :--- | :--- | :--- | :--- |
| `/api/health` | GET 健康检查 | ✅ 通过 | 返回 `status=ok` |
| `/api/tables` | GET 表列表 | ✅ 通过 | 返回 JSON 且包含 `status` 字段 |
| `/api/save_session` | POST 保存会话 | ✅ 通过 | 返回 `status=ok` |
| `/api/create_session` | 传入不存在的 `table_name`，验证返回错误响应 | ✅ 通过 | 证明接口已可访问，且错误路径返回结构可用 |
| `/api/update_cell` | 传入无效 `session_id`，验证返回错误响应 | ✅ 通过 | 证明接口已可访问，且错误路径返回结构可用 |
| `/api/batch_update_cells` | 传入无效 `session_id` 的批量更新 | ✅ 通过 | 返回 `status=error`，错误路径可用 |
| `/api/update_style` | 无活动会话时更新样式 | ✅ 通过 | 返回 `status=error`，错误路径可用 |
| `/api/delete_table` | 删除不存在表 | ✅ 通过 | 路由可访问，返回结构可解析 |
| `/api/grid-data` | 查询不存在表分页数据 | ✅ 通过 | 返回 `status=error`，错误路径可用 |
| `/api/register_table` | 注册真实 CSV 测试表 | ✅ 通过 | 为后续 create_session / update / query 提供数据基线 |
| `/api/create_session -> /api/update_cell -> /api/save_session -> /api/grid-data,/api/execute` | 正向链路：更新后读取一致性 | ✅ 通过 | 已验证会话创建、单元格更新、保存确认、读回一致 |
| `/api/batch_update_cells + /api/execute` | 正向链路：int/float/string 混合更新后 SQL 聚合校验 | ✅ 通过 | `SUM(qty)`、`SUM(price)` 与字符串列更新均符合预期 |
| `/api/update_style + /api/grid-data` | 正向链路：活动会话样式更新后读取 metadata | ✅ 通过 | `metadata.styles["0,1"]` 字段读回正确 |

> 测试命令：`cargo test -p tabula-server --test api_integration_test`
> 当前测试结果：`6 passed; 0 failed`

## 2.2 🧪 已验证仍未实现（自动化）

以下接口已通过自动化确认“当前仍未实现为业务 API”（返回 404 或 405）：

| 接口路径 | 测试场景 | 结果 | 备注 |
| :--- | :--- | :--- | :--- |
| `/api/update_style_range` | POST 请求 | ✅ 已验证未实现 | 返回 `404/405` |
| `/api/insert-column` | POST 请求 | ✅ 已验证未实现 | 返回 `404/405` |
| `/api/delete-column` | POST 请求 | ✅ 已验证未实现 | 返回 `404/405` |

---

## 3. 优化建议与下一步计划

1.  **优先补齐仍未实现接口**：
    *   `/api/update_style_range`、`/api/insert-column`、`/api/delete-column` 仍是 404/405。

2.  **继续强化“成功路径”断言深度**：
    *   在现有正向链路测试基础上，增加跨会话切换、保存后重启读回、边界类型（空值/超长字符串）校验。

3.  **持续收敛文档与实现状态一致性**：
    *   每次新增用例后同步更新 2.1/2.2，确保“已实现”“已验证未实现”边界清晰。
