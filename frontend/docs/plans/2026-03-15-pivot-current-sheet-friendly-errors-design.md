# Pivot Current-Sheet Persistence + Friendly Errors Design

## 背景
当前 Pivot 仅支持新 Session 落库，且失败提示较生硬。用户要求：
1) current-sheet 输出也落库
2) 失败提示更友好（中文）
3) current-sheet 从当前选中单元格开始写

## 目标
- current-sheet 输出落库，从选中单元格作为左上角写入。
- 失败提示中文化且可操作。
- 批量写入仍保留进度提示。

## 非目标
- 不新增后端接口。
- 不调整 Pivot UI 布局。

## 方案对比
### 方案 A（采用）
- current-sheet 按选中单元格写入
- 未选中时回退 A1 并提示
- 只读会话直接提示不可写
- 错误统一转中文提示

### 方案 B
- 固定 A1 写入

### 方案 C
- 只读时自动改为新 Session

## 设计细节（方案 A）
### 写入定位
- 使用 `selectedPosition` 作为 `(rowOffset, colOffset)`
- 未选中则默认 `(0,0)` 并提示

### 扩列
- 若列数超出，仍使用 `/api/ensure_columns` 扩列

### 友好错误提示
- create_session / ensure_columns / batch_update 统一转中文提示
- 文案示例：
  - `当前会话只读，无法写入 Pivot 结果`
  - `扩列失败：后端未实现扩列接口`
  - `落库失败：批量写入被后端拒绝（状态码 500）`

## 验收标准
- current-sheet 输出落库成功并从选中单元格写入
- 只读会话有明确提示
- 失败提示中文且易懂
