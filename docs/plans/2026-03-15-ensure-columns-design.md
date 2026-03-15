# Ensure Columns API Design

**Date:** 2026-03-15

## 背景与目标
Pivot 结果可能产生超过原表列数的新列，需要在当前 session 级 schema 中追加列后，允许 `batch_update_cells` 写入。

## 需求要点
- 新增接口 `POST /api/ensure_columns`
- 请求中传入 `table_name`、`session_id`、`columns`（name + type）
- 若列不存在则追加（按请求顺序追加到 **session** 末尾）
- 幂等：重复调用不重复创建
- 完成后返回最新列清单或 ok
- `batch_update_cells` 能写入新增列

## 选择方案（已确认）
**方案A**：新增 `/api/ensure_columns`，后端批量追加列（session 级），返回列清单。

## 设计细节
### 接口
**Request**
```json
{
  "table_name": "orders",
  "session_id": "xxxx",
  "columns": [
    { "name": "pivot_col_5", "type": "utf8" },
    { "name": "pivot_col_6", "type": "utf8" }
  ]
}
```

**Response**
```json
{ "status": "ok", "columns": ["id", "...", "pivot_col_5", "pivot_col_6"] }
```

### 后端行为
1. 读取 session 的当前 schema
2. 对 columns 顺序遍历：
   - 若列已存在（大小写区分），跳过
   - 不存在则调用 `SessionManager::insert_column` 追加到末尾
3. 返回最新列名列表

### Type 解析
- `type` 解析为 Arrow `DataType`，尽量覆盖所有常见类型（utf8/int64/float64/bool/date/timestamp/decimal 等）
- 不可识别的 type 返回 error

### 幂等与一致性
- 幂等：已有列不重复创建
- Session 级：不写回原表，只修改当前 session schema

## 测试策略（TDD）
- 集成测试：
  1) ensure_columns 新增两列 -> 返回列清单包含新增列
  2) 再次调用 ensure_columns -> 列数不变
  3) 调用 batch_update_cells 写入新增列 -> status ok

## 风险与缓解
- **类型解析失败**：返回明确错误，调用方可回退到 utf8
- **列名冲突**：保持区分大小写，调用方自行保证唯一

---

**结论：** 按方案A执行。
