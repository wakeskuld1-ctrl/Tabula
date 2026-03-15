# 时光机回滚前端接入与提示设计

**目标**：前端时光机回滚按钮调用 `/api/checkout_version`，失败时使用“现有 toast（debug-overlay）”提示错误；补充后端 session/table mismatch 负向测试。

## 范围
- 前端：TimeMachineDrawer 回滚触发 -> App 统一调用回滚接口并显示 toast（debug-overlay）。
- 后端：新增“session_id 与 table_name 不匹配”负向测试（不改接口形态）。

## 方案选择
- 采用方案A1：复用现有 `debug-overlay` 作为 toast 展示（最小改动，避免引入新 UI 组件）。

## 架构与数据流
1. 用户点击版本 -> confirm。
2. App 发起 `POST /api/checkout_version?table_name=...&version=...&session_id=...`。
3. 成功：刷新表格数据并关闭抽屉；toast 提示成功（可选）。
4. 失败：toast 显示错误信息（来自后端 `message` 或 HTTP 文本）。

## 接口约定
- URL：`/api/checkout_version`（query 参数：`table_name`、`version`、`session_id` 可选）。
- 响应：`{ status: "ok" | "error", message?: string }`。

## 错误处理
- HTTP 非 2xx 或 `status=error` -> toast 提示。
- 统一解析逻辑：优先 `message`，否则落到 HTTP 状态或文本。

## 测试
- 后端新增集成测试：当 `session_id` 与 `table_name` 不匹配时，接口返回 `status=error`。
- 前端无现成测试框架，保留手工验证步骤（点击回滚成功/失败 toast）。

## 风险与缓解
- 风险：debug-overlay 语义偏“调试”，用户提示不够显眼。
- 缓解：短时显示+明显错误色（不引入新组件）。
