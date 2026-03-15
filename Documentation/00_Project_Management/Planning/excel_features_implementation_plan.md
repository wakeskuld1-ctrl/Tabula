# 基于 Glide Data Grid 的 Excel 功能扩展方案

## 0. 方案决策分析 (Cost vs Benefit)

您提出的两个方向对比：

| 方案 | A. Wasm (FortuneSheet) 换肤 | B. Glide + 移植逻辑 (推荐) |
| :--- | :--- | :--- |
| **核心思路** | 使用成熟的 FortuneSheet，只修改 CSS/UI 让其变好看 | 保留 Glide 的极速渲染，手动实现 Toolbar/公式/编辑逻辑 |
| **开发成本** | **极低** (仅需调整样式配置) | **中高** (需手写 Excel 业务逻辑) |
| **运行性能** | 中等 (Canvas, 10万行级) | **极高** (Glide, 百万行级) |
| **功能完备性** | **完备** (自带公式、透视表、图表) | **初期简陋** (需逐个功能搬运) |
| **结论** | 适合“功能优先、数据量不大”的场景 | 适合 **“性能优先、定制化强”** 的场景 |

**最终决策**: 鉴于您明确要求**“保持现在的极速网格”**，我们选择 **方案 B**。虽然初期成本较高，但我们可以通过“移植”现有开源库（如 `fast-formula-parser`）来降低逻辑开发成本。

## 1. 核心目标
在保留 **Glide Data Grid (GDG)** 极致渲染性能（百万行秒级加载）的基础上，通过**自研外壳 + 功能注入**的方式，实现类似 Excel/Univer 的交互体验。
**不引入**沉重的全栈框架，而是像搭积木一样，将工具栏、公式栏、样式管理等模块“组装”到 GDG 上。

## 2. 总体架构设计

```mermaid
graph TD
    A[Excel Shell (React Container)] --> B[Toolbar (工具栏)]
    A --> C[Formula Bar (公式栏)]
    A --> D[Glide Data Grid (渲染核心)]
    A --> E[Sheet Bar (底部标签页)]
    
    D --> F[Custom Renderers (自定义渲染)]
    D --> G[Grid State (前端缓存)]
    
    B --> H[Style Manager (样式管理)]
    C --> I[Formula Engine (公式引擎)]
    
    subgraph Backend [Rust Backend]
        J[Session Manager]
        K[Data Engine (LanceDB/DataFusion)]
        L[Metadata Store (SQLite/JSON)]
    end
    
    H -- Sync --> L
    I -- Calc --> K
    G -- Data --> K
```

## 3. 详细功能模块规划

### 3.1 基础编辑与交互 (Phase 1 - 必须优先完成)
目前您提到“还不能编辑”，这是因为我们尚未将 GDG 的编辑事件对接回后端。
*   **功能**: 双击单元格进入编辑模式。
*   **实现**: 
    *   启用 GDG 的 `editable={true}`。
    *   实现 `onCellEdited` 回调 -> 调用后端 `/api/update_cell`。
    *   处理数据类型转换 (Text -> Number/Date)。

### 3.2 UI 外壳 (Phase 2 - 视觉层)
模仿 Excel/Univer 的布局，包裹 GDG。
*   **顶部工具栏 (Toolbar)**:
    *   实现：使用 React 组件库 (如 AntD 或 Tailwind UI) 手写一行图标按钮。
    *   功能：撤销/重做、字体加粗/倾斜、背景色、文字颜色、对齐方式。
*   **公式栏 (Formula Bar)**:
    *   实现：一个受控的 `Input` 组件。
    *   联动：点击 Grid 单元格 -> 更新 Input 值；修改 Input -> 更新 Grid 单元格。
*   **右键菜单 (Context Menu)**:
    *   实现：捕获 GDG 的 `onContextMenu` 事件，弹出自定义 DOM 菜单。
    *   功能：复制、粘贴、插入行/列、删除。

### 3.3 核心逻辑增强 (Phase 3 - 逻辑层)
这是最复杂的“模仿”部分，需要管理数据之外的“元数据”。

#### A. 样式系统 (Styles)
GDG 本身只渲染数据，我们需要“告诉”它哪个格子是红色的。
*   **前端**: 维护一个 `Map<CellId, StyleObj>`。在 `getCellContent` 时，将样式合并到返回对象中。
*   **后端**: 
    *   目前 LanceDB 主要存**值**。
    *   **方案**: 新增一个 `metadata.json` 或 SQLite 表，专门存储 `(row, col) -> { bold: true, color: '#ff0000' }` 的稀疏矩阵数据。

#### B. 公式引擎 (Formulas)
*   **前端**: 集成轻量级解析器 (如 `hyperformula` 或 `fast-formula-parser`)。
    *   当输入以 `=` 开头时，不直接显示值，而是存储公式。
*   **计算**: 
    *   **小数据**: 前端直接计算 (JS)。
    *   **大数据**: 后端 Rust 计算 (利用 DataFusion 的表达式引擎)。

## 4. 数据结构升级 (Data Model)

为了支持上述功能，我们需要扩展前后端的数据协议：

**旧协议 (纯值)**:
```json
{ "data": [ ["1", "Alice"], ["2", "Bob"] ] }
```

**新协议 (值 + 元数据)**:
```json
{
  "data": [
    { "value": "1", "style": { "bold": true } },
    { "value": "Alice", "formula": "=UPPER('alice')" }
  ]
}
```

### 3.4 Header Management (表头策略)
User Requirement: Handle cases where the first row is NOT a header (e.g., data starts at row 0).
*   **Strategy**: Configurable `header_rows` and `header_mode`.
    *   `header_rows`: Number of rows to treat as header. `0` means no header.
    *   `header_mode`: `"first_row"` (default), `"none"` (generate `column_N`), `"merge"` (future: multi-line).
*   **Implementation Status**: 
    *   Backend `ExcelDataSource` updated to support `header_rows=0` and `header_mode="none"`.
    *   When disabled, headers are generated as `column_1`, `column_2`, etc.
    *   Verified with unit tests.

### 3.5 筛选弹层样式复刻（UI 阶段）
需求说明：先完成 Excel 风格筛选弹层的结构与样式复刻，保留搜索框、勾选列表、顶部排序/清除与底部确认/取消布局，不接入真实过滤逻辑。
*   **实现要点**:
    *   弹层仅负责 UI 交互与状态收集，输出统一的确认事件出口。
    *   后续接入数据时，只需将确认事件对接数据源/后端过滤接口。
    *   弹层默认从筛选按钮左下方弹出，第一列时贴边展示。
