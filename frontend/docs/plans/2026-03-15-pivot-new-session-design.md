# Pivot New Session Persistence Design

## 背景
当前 Pivot 生成结果仅在前端覆盖视图，无法落库到新 Session。用户要求生成结果落到“新 Sheet / 新 session tab”，并持久化。

## 目标
- Pivot 生成结果写入新 Session（后端持久化）。
- 新 Session 默认干净，无需清理旧值。
- 写入完成后自动切换到新 Session。

## 非目标
- 不新增后端接口。
- 不实现 Pivot “当前 sheet”写入。

## 方案对比
### 方案 A（采用）
- 流程：create_session -> batch_update_cells（分批写入）-> fetch sessions -> 切换到新 session
- 优点：复用现有接口、改动小、风险低
- 缺点：大结果写入耗时，需分批

### 方案 B
- 新增后端接口一次性生成写入
- 优点：性能好
- 缺点：需要后端配合

### 方案 C
- 临时表 + SQL 插入
- 优点：减少前端压力
- 缺点：逻辑复杂、依赖后端 SQL 约束

## 设计细节（方案 A）
### 数据流
1) `handlePivotApply('new-sheet')` 触发
2) 调 `create_session` 创建新 session
3) 构造更新列表：第 1 行写 headers，后续行写 data
4) 分批调用 `batch_update_cells`
5) 刷新 session 列表并切换到新 session

### 分批策略
- 每批 500 条更新（可调）
- 写入过程中提示 loading

### 数据格式
- `updates: { row, col, val }[]`
- `col` 优先使用当前表列名；若缺失则降级为 `col_{index}`

## 风险与应对
- 结果列数超过基础表列数时可能失败：降级列名并提示潜在风险
- 大结果写入耗时：分批 + 可见 loading

## 验收标准
- 点击 Pivot 生成后出现新 session tab
- 新 session 打开后显示 Pivot 结果（含表头行）
- 失败时有明确错误提示
