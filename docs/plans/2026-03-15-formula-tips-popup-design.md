# Formula Tips Popup & Top Bar Row Design

**Date:** 2026-03-15

## 背景
- 公式帮助常驻占用高度，影响表格可视区域。
- 需要按条件触发显示，并折叠为单行以降低视觉占用。
- 顶部表选择与 Pivot 控件目前换行，影响操作流畅度。
- 前端请求 /api/* 在 dev 环境出现 404/405，需要明确后端地址与代理策略。

## 目标
- 公式帮助仅在“输入以 `=` 开头”或“点击 fx”时弹出。
- 弹出面板默认折叠为单行信息，点击条目展开完整说明。
- 顶部表选择与 Pivot 置于同一行展示。
- dev 环境 /api 请求指向后端 `http://localhost:3000/`。

## 非目标
- 不改动公式数据源内容与字段结构。
- 不变更后端接口实现，仅修复前端 dev 代理行为。

## 设计

### 1) 触发逻辑
- `showFormulaHelp = inputStartsWithEqual || isFxToggled`
- `inputStartsWithEqual`：`text.trim().startsWith("=")`
- `isFxToggled`：沿用现有 `fx` 按钮交互态（或新增本地状态）

### 2) 折叠/展开
- 默认折叠展示：`{purpose} = {syntax}`
- 展开时展示：name / syntax / example / purpose / paramNotes / note
- 展开状态为本地 state（按条目 name 追踪）

### 3) 位置与布局
- 面板定位在公式栏下方（随输入栏整体移动）
- 未触发时不渲染面板（释放空间）

### 4) 顶部行合并
- `status-bar` 采用单行 flex 布局
- 表选择与 Pivot 按钮同组展示

### 5) API 代理
- Vite dev server 将 `/api` 代理至 `http://localhost:3000/`
- 预期消除 `TimeMachineDrawer` 与 `update_style_range` 的 404/405

## 风险
- 折叠行文本较长时可能溢出，需要省略号与 tooltip。
- fx 弹出与已有 suggestions 逻辑可能冲突，需要明确优先级。

## 验收
- 输入 `=` 或点击 `fx` 后显示折叠面板。
- 点击条目可展开/折叠。
- 顶部表选择与 Pivot 同行显示。
- dev 环境 /api 请求走 `localhost:3000`。
