# Excel 类表格能力建设产品需求文档 (PRD)
版本 : v1.0 状态 : 待评审 涉及组件 : ExcelShell , FormulaEditor , FormulaEngine , GlideGrid , Toolbar , FormulaBar

## 1. 项目背景与目标
当前系统已具备高性能表格渲染（Glide Data Grid）与基础公式计算（HyperFormula）能力。为满足复杂数据处理需求，需构建接近原生 Excel 体验的交互层，核心目标是将独立的公式编辑器、工具栏与表格网格深度整合，实现“所见即所得”的电子表格体验。

## 2. 现状分析 (As-Is)
- UI 框架 : ExcelShell 已实现经典的顶部工具栏、公式栏、底部 Sheet 栏布局。
- 公式编辑 : FormulaEditor 实现了自动补全、函数提示，但目前仅作为 Grid 单元格的独立编辑器。
- 公式栏 : FormulaBar 仅做简单展示，未与 Grid 选区及 FormulaEditor 的高级能力打通。
- 计算引擎 : FormulaEngine (HyperFormula) 单例运行，尚未深度集成多 Sheet 上下文与跨表引用。
## 3. 核心用户故事 (User Stories)
### Epic 1: 沉浸式公式编辑体验
ID Story 名称 描述 验收标准 (AC) US-1.1 公式栏双向同步 作为用户，我在 Grid 中选中单元格时，顶部公式栏应实时显示原始内容；在公式栏输入时，Grid 单元格应实时预览结果。 1. Grid 选区变更 -> 公式栏更新
 2. 公式栏输入 -> Grid 单元格进入编辑模式并同步内容 US-1.2 智能函数补全 作为用户，在顶部公式栏输入 = 时，应获得与单元格编辑一致的函数提示列表。 1. 复用 FormulaEditor 的 suggestions 逻辑
 2. 支持键盘上下选择与 Tab 键补全 US-1.3 跨 Sheet 引用 作为用户，我希望在 Sheet1 的公式中引用 Sheet2 的数据（如 =Sheet2!A1 ）。 1. FormulaEngine 支持多 Sheet 实例
 2. 解析并计算跨 Sheet 依赖

### Epic 2: 表格交互与样式增强
ID Story 名称 描述 验收标准 (AC) US-2.1 样式快捷操作 作为用户，点击工具栏的“加粗/变色”按钮时，应直接应用到当前选中的单元格区域。 1. 样式操作支持 Undo/Redo
 2. 样式数据随 PageData 持久化 US-2.2 函数向导 (fx) 作为用户，点击公式栏旁的 fx 按钮，应弹出函数选择面板。 1. 点击 fx 弹出浮窗
 2. 双击函数名插入到当前光标位置

## 4. 技术方案与复用策略
1. 逻辑复用 : 将 FormulaEditor.tsx 中的自动补全 ( AutoSuggestion ) 和浮窗逻辑 ( FxPopup ) 抽离为 useFormulaLogic Hook，供 FormulaBar 和 GridCellEditor 共同使用。
2. 状态管理 : 提升 ExcelShell 为单一数据源（Source of Truth），管理 currentSelection , editMode , formulaValue 等共享状态，通过 Context 或 Props 下发。
3. 引擎升级 : 改造 FormulaEngine.ts 单例，使其内部维护 Map<SheetId, SheetId> 映射，支持 HyperFormula 的多 Sheet API。
## 5. 里程碑计划 (Milestones)
### Phase 1: 基础整合与复用 (预计周期: 1周)
- 目标 : 打通 UI 组件与底层逻辑，消除代码冗余。
- 任务 :
  - [Refactor] 从 FormulaEditor 抽离 useFormulaSuggestion 。
  - [Feat] FormulaBar 接入自动补全 Hook。
  - [Feat] ExcelShell 实现选区状态提升，打通 Grid -> FormulaBar 的单向数据流。
### Phase 2: 双向交互与计算增强 (预计周期: 1.5周)
- 目标 : 实现流畅的公式编辑与跨表计算。
- 任务 :
  - [Feat] 实现 FormulaBar -> Grid 的实时输入同步。
  - [Feat] FormulaEngine 升级支持多 Sheet 管理 ( addSheet , removeSheet )。
  - [Feat] 实现 fx 函数向导按钮的点击交互。
### Phase 3: 样式与持久化 (预计周期: 1周)
- 目标 : 完善视觉反馈与数据保存。
- 任务 :
  - [Feat] 对接 Toolbar 样式按钮到 GlideGrid 的 themeOverride 或样式数据列。
  - [Test] 编写针对跨 Sheet 公式计算的 E2E 测试用例。
  - [Docs] 更新用户操作手册。
## 6. 风险与缓解
- 风险 : 跨 Sheet 引用可能导致循环依赖计算性能下降。
- 缓解 : 在 FormulaEngine 中通过 HyperFormula 的 validateFormula 预检循环依赖，并设置计算超时熔断。
