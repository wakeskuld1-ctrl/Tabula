# 公式列 CAST 兼容性测试设计（方案B）

## 变更记录
- **[2026-02-17]** 变更原因：需要记录跨 DataFusion/SQLite 的 CAST 兼容性测试方案; 变更目的：统一方案并降低回归风险

## 背景
公式列 SQL 当前采用 `TRY_CAST(NULLIF(CAST(col AS VARCHAR), '') AS DOUBLE)`，用于避免空字符串与数值列混算导致的 Arrow Cast error。为覆盖潜在的方言差异与前端缓存风险，需要补充后端集成测试与 E2E 端到端测试。

## 目标
- 覆盖 DataFusion 与 SQLite 两条路径对 `CAST(... AS VARCHAR)` 的兼容性
- 覆盖 E2E 中“公式列更新 + 重新拉取 grid-data”的关键路径
- 自动准备测试数据并在 E2E 结束后清理

## 非目标
- 不扩展到其他数据库方言（如 Oracle）
- 不更改生产逻辑，仅新增测试与脚本验证

## 方案概述（方案B）
### 后端
- 在现有测试文件中新增集成测试，分为 DataFusion 路径与 SQLite 路径
- DataFusion 路径使用内存表注册与查询验证
- SQLite 路径使用临时 sqlite db + SQL 执行验证

### 前端 E2E
- 若当前表数值列不足，则自动创建临时表（两列数值）
- 写入样例数据、更新公式列、再次拉取 grid-data 验证
- 结束后删除临时表以保持环境清洁

## 测试设计
### 后端用例
1. DataFusion：
   - SQL：`SELECT TRY_CAST(NULLIF(CAST(col AS VARCHAR), '') AS DOUBLE)`
   - 断言：空字符串得到 NULL；数值字符串得到数值
2. SQLite：
   - SQL：`SELECT CAST(NULLIF(CAST(col AS VARCHAR), '') AS REAL)`
   - 断言：空字符串得到 NULL；数值字符串得到数值

### E2E 用例
1. 创建临时表并插入样例数据（两列数值）
2. 调用 `/api/update-column-formula` 更新公式列
3. 再次请求 `/api/grid-data` 并检查 `status=ok` 与数据非空
4. 清理临时表

## 风险与回退
- 若 SQLite 不支持 `VARCHAR`，改用 `TEXT` 作为回退（仅测试层）
- 若临时表创建失败，E2E 需记录跳过原因并不中断其他步骤

## 验证标准
- 后端集成测试通过
- E2E 执行报告中“公式列加载”步骤为 PASS 或合理 SKIP
