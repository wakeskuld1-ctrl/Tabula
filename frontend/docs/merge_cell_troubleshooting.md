# Glide Data Grid 合并单元格问题分析与整改方案

## 1. 问题现状

在 `GlideGrid.tsx` 中实现合并单元格功能时，遇到以下主要问题：
1. **报错**：控制台频繁出现 `TypeError: input.columns.offset is not a function`。
2. **渲染异常**：合并后的单元格内容可能被裁剪，或者显示不完整（仅在左上角显示），看起来“不生效”。
3. **交互失效**：在某些情况下，点击合并按钮后 UI 无响应，或需刷新页面才能看到变化。

## 2. 根本原因分析

经过代码审查和 E2E 测试日志分析，定位到以下两个核心问题：

### 2.1 Selection 对象原型丢失 (导致 `TypeError`)
- **现象**：`DataEditor` 内部抛出 `input.columns.offset is not a function`。
- **原因**：`GlideGrid` 组件在处理 `onGridSelectionChange` 时，可能手动构建了 `GridSelection` 对象，或者在传递过程中 `CompactSelection` 对象丢失了其原型链（Prototype），退化为普通 JSON 对象。因此，当 Grid 内部尝试调用 `offset` 方法时失败。
- **触发点**：通常发生在热更新 (HMR) 或手动通过 `setSelection` 更新状态时。

### 2.2 `span` 属性定义歧义 (导致渲染裁剪)
- **现象**：合并单元格内容被裁剪，只能看到左上角的部分内容。
- **原因**：`Glide Data Grid` 依赖 `GridCell.span` 属性来计算单元格的裁剪区域 (Clip Region)。如果 `span` 设置不正确，Grid 会默认该单元格为 1x1 大小，从而裁剪掉超出的绘制内容。
- **歧义点**：
  - 代码当前实现：`span: [width, height]` (例如 `[2, 2]` 代表 2行2列)。
  - 可能的正确定义：在某些版本中，`span` 被定义为 `[startCol, endCol]` (仅列合并) 或其他格式。若格式不匹配，Grid 将忽略 `span` 属性。
- **文档疑点**：`data-grid-types.d.ts` 中定义为 `readonly [start: number, end: number]`，参数命名暗示可能是范围索引而非尺寸。

## 3. 整改思路

建议按照以下步骤进行修复，无需重写整个组件：

### 3.1 修复 Selection 对象构造
在 `onGridSelectionChange` 和 `mergeSelection` 中，严谨地处理 `GridSelection` 对象：
- **禁止手动字面量构造**：不要使用 `{ columns: { ... } }` 这种方式手动创建 `CompactSelection`。
- **使用静态方法**：始终使用 `CompactSelection.empty()` 或 `CompactSelection.fromSingleSelection()` 来创建实例。
- **防御性编程**：在 `setSelection` 前检查 `columns` 和 `rows` 是否具备 `offset` 方法，若丢失则尝试重建。

### 3.2 修正 `span` 计算逻辑
根据 Glide Data Grid v6.0.3 的规范，验证并修正 `getCellContent` 中的 `span` 返回值：

**方案 A (当前假设)**：`[colSpan, rowSpan]`
```typescript
span = [mEndCol - mStartCol + 1, mEndRow - mStartRow + 1];
```
如果此方案无效（已被证明存在裁剪问题），尝试方案 B。

**方案 B (可能的正确定义)**：`[startCol, endCol]` (仅支持列合并?) 或 `[startIdx, endIdx]`
需查阅官方文档确认 `span` 的确切含义。如果 `span` 是 `[start, end]`，则应改为：
```typescript
span = [mStartCol, mEndCol]; // 仅示例，需验证
```

**关键验证方法**：
修改 `getCellContent`，硬编码一个 `span: [0, 1]` 或 `span: [2, 2]`，观察 Grid 是否扩大了裁剪区域。

### 3.3 渲染层优化 (`drawCell`)
一旦 `span` 正确生效，Grid 会自动处理裁剪。此时 `drawCell` 的实现应简化：
- **仅绘制 Master Cell**：通过 `args.rect` 获取的已经是合并后的大矩形。
- **Covered Cells**：直接返回 `span: undefined` 且不进行任何绘制（Grid 甚至可能不会对 Covered Cells 调用 `drawCell`，如果 `span` 生效的话）。

## 4. 推荐代码调整 (伪代码)

```typescript
// 1. 修复 Selection
const newCols = newSelection.columns instanceof CompactSelection 
    ? newSelection.columns 
    : CompactSelection.empty(); // 确保是实例

// 2. 修正 Span (待验证)
// 如果文档确认 span 是 [colSpan, rowSpan]
const span = [width, height] as const; 

// 3. 简化 drawCell
// 不再需要手动计算 mergeX, mergeY，直接利用 args.rect
const drawCell = (args) => {
    if (isMasterCell) {
        // args.rect 应该是合并后的大小 (如果 span 生效)
        ctx.fillStyle = bg;
        ctx.fillRect(args.rect.x, args.rect.y, args.rect.width, args.rect.height);
        ctx.fillText(text, ...);
    }
    // Covered cells do nothing
}
```

## 5. 结论
当前问题的核心在于 **Selection 对象的类型丢失** 和 **Span 属性的定义不匹配**。建议优先解决 Selection 报错，然后通过实验确定正确的 Span 格式，最后清理 `drawCell` 的冗余计算。

## 6. 本次整改记录
### 6.1 修改点
1. 合并索引重建：以缓存页的 merges 为唯一来源重建索引，避免残留映射导致选区与渲染失配。
2. 会话与刷新清理：切换表/会话和刷新时清空合并与样式索引，避免跨会话污染。
3. span 计算修正：统一按列跨与行跨返回 `[colSpan, rowSpan]`，纵向与矩阵合并可正确生效。
4. spanRangeBehavior 启用：显式开启合并跨度行为，让 GDG 原生识别合并裁剪区域（当前使用 allowPartial）。

### 6.2 解除合并异常处理
解除合并时依赖全局合并索引。如果索引未及时刷新，会导致选区仍被旧合并范围“吸附”，出现报错或交互异常。当前通过合并完成后与分页数据加载后的索引重建，确保解除合并后映射立即清空。

### 6.3 代码位置
- 合并索引与重建流程：[GlideGrid.tsx](file:///d:/Rust/metadata/frontend/src/components/GlideGrid.tsx)
- 合并索引重建函数：[merge.js](file:///d:/Rust/metadata/frontend/src/utils/merge.js)
