# Excel 能力技术实施与重构方案

**版本**: v1.0
**日期**: 2026-02-01
**关联文档**: [Excel 类表格能力建设产品需求文档 (PRD)](./Excel%20类表格能力建设产品需求文档%20(PRD).md)

本文档基于 PRD 需求，针对 `FormulaEditor.tsx` 的代码重构、风险治理、架构演进及前端开发规范进行详细拆解。

## 1. FormulaEditor 代码重构方案

当前 `FormulaEditor.tsx` 耦合了 UI 渲染（Input, Popup）与业务逻辑（正则匹配、函数检索、键盘导航）。为了实现 `FormulaBar` 与 Grid 单元格编辑器的逻辑复用，需进行 Hook 化拆分。

### 1.1 现状分析
- **输入与状态**: 内部维护 `text`, `suggestions`, `showFxPopup` 等状态。
- **业务逻辑**: 包含 `useEffect` 监听文本变化触发正则匹配，以及 `handleKeyDown` 处理建议列表选择。
- **UI 呈现**: 包含 `<input>`, FX 按钮, 建议列表 Portal, FX 面板 Portal。

### 1.2 重构目标 (Refactoring Plan)
将逻辑抽离为自定义 Hook `useFormulaLogic`，使 UI 组件变薄 (dumb component)。

#### 步骤 1: 抽离 `useFormulaLogic` Hook
该 Hook 应包含以下核心能力：
- **输入状态管理**: `value`, `setValue`
- **智能提示计算**: 监听 `value` 变化，调用 `FormulaEngine` 获取 `suggestions`。
- **键盘导航逻辑**: 处理 `ArrowUp`, `ArrowDown`, `Enter`, `Tab`，维护 `selectedIndex`。
- **函数应用逻辑**: 提供 `applySuggestion(funcName)` 方法，处理文本替换与光标定位。

**代码接口设计示例**:
```typescript
interface UseFormulaLogicProps {
    initialValue: string;
    onCommit: (val: string) => void;
}

const useFormulaLogic = ({ initialValue, onCommit }: UseFormulaLogicProps) => {
    // ... 内部状态逻辑 ...
    return {
        text,
        setText,
        suggestions,
        selectedIndex,
        handleKeyDown,
        applySuggestion,
        // ...
    };
};
```

#### 步骤 2: 改造 `FormulaEditor` 组件
`FormulaEditor.tsx` 将不再包含核心逻辑，改为直接调用 `useFormulaLogic`，仅负责渲染：
- 输入框渲染
- `ReactDOM.createPortal` 渲染悬浮窗
- 样式布局

#### 步骤 3: 复用于 `FormulaBar`
`FormulaBar.tsx` 同样引入 `useFormulaLogic`，实现与单元格编辑器一致的智能提示体验。

---

## 2. Formula 到 Grid 的能力改造 (Architecture Evolution)

从“独立编辑器”向“Excel 整体协同”转变，核心是**状态提升 (Lifting State Up)** 与 **单一数据源 (Single Source of Truth)**。

### 2.1 架构变更
- **Before**: `FormulaEditor` 是 Grid 内部的黑盒，状态封闭。
- **After**: `ExcelShell` 作为容器，持有 `GlobalSelection` 和 `GlobalEditingState`。

### 2.2 改造路径
1.  **选区感知 (Selection Awareness)**:
    - `GlideGrid` 的 `onGridSelectionChange` 事件需抛出当前选中单元格的完整数据（坐标、原始值、计算值）给 `ExcelShell`。
    - `ExcelShell` 将选中状态传递给 `FormulaBar`，使其回显当前单元格内容。

2.  **双向同步 (Two-way Sync)**:
    - **Grid -> FormulaBar**: 选中单元格 -> `ExcelShell` -> 更新 `FormulaBar` 的 `value`。
    - **FormulaBar -> Grid**: `FormulaBar` 输入 -> `ExcelShell` -> 调用 `Grid` 的数据更新方法 (或更新 `PageData`) -> 触发 Grid 重绘。

3.  **统一公式引擎上下文**:
    - `FormulaEngine` 需注入到 App 上下文或通过 Singleton 全局访问，确保 `FormulaBar` 和 `Grid` 访问的是同一个计算实例（共享 Sheet 数据）。

---

## 3. 风险治理策略 (Risk Management)

针对 PRD 中提到的风险点，制定以下技术应对措施。

### 3.1 循环依赖 (Circular Dependency)
- **风险**: A1 引用 B1，B1 引用 A1，导致计算死循环栈溢出。
- **处理**:
    - **预检**: 在 `FormulaEngine.setCellValue` 前，使用有向图算法或 HyperFormula 自带的 `validateFormula` 检测循环。
    - **熔断**: 若检测到循环，不执行计算，直接返回 `#CYCLE!` 错误码。

### 3.2 大数据量性能 (Performance)
- **风险**: 自动补全正则匹配在百万行数据或长文本下卡顿。
- **处理**:
    - **防抖 (Debounce)**: 对 `text` 变化监听增加 100-200ms 防抖，避免每次击键都触发正则与检索。
    - **数量限制**: `suggestions` 列表仅截取前 10-20 项渲染，避免长列表 DOM 性能问题。

### 3.3 跨 Sheet 引用失效
- **风险**: 删除 Sheet 后，引用该 Sheet 的公式报错。
- **处理**:
    - **监听器**: `FormulaEngine` 需建立依赖图谱 (Dependency Graph)。当 Sheet 删除时，反向查找所有依赖节点，标记为 `#REF!` 错误。

---

## 4. 前端开发规范 (Frontend Standards)

为保证代码质量与可维护性，执行以下规范：

### 4.1 命名规范
- **组件 (Components)**: PascalCase，如 `FormulaEditor.tsx`, `ExcelShell.tsx`。
- **Hook**: camelCase，以 `use` 开头，如 `useFormulaLogic.ts`。
- **工具类**: Singleton 模式类名 PascalCase (`FormulaEngine`)，普通函数文件 camelCase。
- **Props**: 明确的接口定义，避免使用 `any`。事件处理函数以 `on` 开头 (如 `onCommit`)，处理方法以 `handle` 开头 (如 `handleKeyDown`)。

### 4.2 代码结构
```text
src/
  components/
    layout/         # 布局组件 (ExcelShell, Toolbar)
    core/           # 核心业务组件 (FormulaEditor)
  hooks/            # 自定义 Hooks (useFormulaLogic)
  utils/            # 工具类 (FormulaEngine)
  types/            # 全局类型定义
```

### 4.3 状态管理
- **原则**: 优先使用局部 State，仅需跨组件共享时提升至父组件 (`ExcelShell`)。
- **Immutability**: 涉及 Grid 数据更新时，必须遵循不可变原则，创建新对象/数组，确保 `React.memo` 或 `useCallback` 依赖正确触发。

### 4.4 样式规范
- **方案**: 当前使用 Inline Style (`style={{...}}`)，建议后续迁移至 CSS Modules 或 Styled Components 以提升复用性和性能。
- **主题**: 颜色、字体等硬编码值应提取为常量或 CSS Variables (`var(--gdg-font-family)`).

### 4.5 测试规范
- **单元测试**: 核心逻辑 Hook (`useFormulaLogic`) 必须有对应的单元测试，覆盖正则匹配、按键处理等边界情况。
- **集成测试**: 针对公式计算链路 (`Input -> Engine -> Output`) 编写测试用例。
