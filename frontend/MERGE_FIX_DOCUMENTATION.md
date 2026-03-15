# GlideGrid 合并单元格渲染修正方案

本文档记录了解决“双框问题（小绿框+大蓝框）”和“内部灰线透视问题”的完整代码实现。

## 1. 核心原理

1.  **解决双框问题**：开启 `spanRangeBehavior="allow"` 并正确返回 `span`，让 Glide Data Grid (GDG) 原生接管焦点框（绿框）的渲染，使其自动撑大以匹配合并区域。
2.  **解决灰线问题**：利用“层级遮盖”原理，给 Master Cell 设置不透明背景色（`bgCell: "#ffffff"`），像涂改液一样遮住底层的网格线。
3.  **Covered Cells**：被覆盖的单元格必须返回纯净的空对象，不带任何样式或边框，防止“Ghost Lines”干扰。

---

## 2. 用户反馈点与解决方案 (User Feedback & Resolution)

针对用户提出的核心问题，我们逐一进行了修复：

### Q1: "灰线（网格线）透视问题"
*   **问题描述**：合并单元格内部依然能看到原有的网格线，没有“一张白纸”的感觉。
*   **原因**：GDG 默认先绘制底层网格线，如果合并单元格背景透明，线条就会透出来。
*   **解决方案**：**层级遮盖**。在 Master Cell 中强制设置 `themeOverride.bgCell = "#ffffff"`。这相当于在合并区域铺了一层不透明的白纸，物理遮挡了下层的网格线。

### Q2: "双框问题（小绿框+大蓝框）"
*   **问题描述**：绿框（Focus）只框住了左上角第一个格子，而蓝框（Selection）框住了整个区域。绿框应该自动撑大覆盖蓝框。
*   **原因**：
    1.  组件未开启 `spanRangeBehavior="allow"`，导致 GDG 忽略了 span 属性，按单格渲染 Focus。
    2.  代码逻辑 bug（变量作用域问题）导致 `span` 属性未能正确传递给返回对象。
*   **解决方案**：
    1.  **Props 配置**：在 `<DataEditor>` 中显式添加 `spanRangeBehavior="allow"`。
    2.  **逻辑修正**：确保 `getCellContent` 正确计算并返回 `span: [col_diff, row_diff]`。

### Q3: "障眼法破坏原生渲染"
*   **问题描述**：之前试图通过把边框改成白色来消除网格线，这破坏了 GDG 原生逻辑。
*   **解决方案**：**完全移除手动边框 hack**。删除所有 `borderHorizontal`, `borderVertical` 的颜色覆盖代码。完全信任 GDG 的原生渲染机制，只通过 `bgCell` 做遮挡，边框交给 GDG 自动处理。

---

## 3. 组件配置 (`DataEditor`)

在 `<DataEditor>` 组件中必须显式开启 `spanRangeBehavior`。

```tsx
<DataEditor
    // ...其他属性
    
    // ★★★ 核心开关：必须设置为 "allow" ★★★
    // 告诉 GDG："请把 getCellContent 返回的 span 属性渲染出来，把绿框撑大！"
    spanRangeBehavior="allow"
    
    // 选区控制 (双向绑定)
    gridSelection={selection}
    onGridSelectionChange={onGridSelectionChange}
    
    // [注意]：不要再在 drawCell 里手动画边框了，否则会出现双重边框
/>
```

---

## 4. `getCellContent` 完整实现 (最终修正版)

这是修复后的核心逻辑，涵盖了 Master Cell 的 Span 计算和 Covered Cell 的隐身处理。

**特别注意**：在 `getCellContent` 中声明 `span` 变量时，务必注意作用域！不要在 `if` 块里用 `const span` 覆盖了外部的 `let span`。

```typescript
const getCellContent = useCallback(
  (cell: Item): GridCell => {
    const [col, row] = cell;
    
    // ... 获取 pageData ...

    // 默认 span 为 undefined
    let span: [number, number] | undefined = undefined;
    let themeOverride: Theme | undefined = undefined;
    let contentAlign: ItemAlign | undefined = undefined;

    if (pageData && pageData.metadata && pageData.metadata.merges) {
         // 查找属于该单元格的合并信息
         const merge = pageData.metadata.merges.find(m => 
             row >= m.start_row && row <= m.end_row &&
             col >= m.start_col && col <= m.end_col
         );

         if (merge) {
             // ==================================================
             // 情况 A: 我是老大 (Start Cell / Master Cell)
             // ==================================================
             if (merge.start_row === row && merge.start_col === col) {
                 // [Fix] 直接赋值给外部 let span，不要用 const span 重新声明！
                 span = [
                     merge.end_col - merge.start_col,
                     merge.end_row - merge.start_row
                 ];

                 // ★ 关键点 1：强制不透明背景
                 // 像涂改液一样盖住底下的线
                 if (!themeOverride) themeOverride = {};
                 if (!themeOverride.bgCell) themeOverride.bgCell = "#ffffff"; 
                 
                 // 默认垂直居中 (Excel 风格)
                 if (!contentAlign) contentAlign = "center";
             }
             
             // ==================================================
             // 情况 B: 我是小弟 (Covered Cell)
             // ==================================================
             else {
                 // 这些单元格被老大的 span 盖住了，但在逻辑上它们还存在。
                 // 我们返回一个“空气”单元格，防止它们画出自己的边框干扰视觉。
                 return {
                     kind: GridCellKind.Text,
                     allowOverlay: false,
                     readonly: true,
                     displayData: "",
                     data: "",
                     span: undefined,
                     // ★ 关键点 2：极致纯净，不带 themeOverride
                 };
             }
         }
    }

    // 3. 返回最终对象 (Master Cell 或 普通 Cell)
    return {
        kind: GridCellKind.Text,
        displayData: val,
        data: val,
        allowOverlay: true,
        span,           // <--- 这里必须接收到计算后的 span
        themeOverride,  // <--- 包含 bgCell: #fff
        contentAlign,
    };
  },
  [/* 依赖项 */]
);
```

## 5. 避坑指南

1.  **变量作用域陷阱**：
    *   **错误写法**：在 `if` 块里写 `const span = [...]`。这会创建一个局部变量，导致 `return` 语句里的 `span` 依然是 `undefined`。
    *   **正确写法**：在函数顶部 `let span`，在 `if` 块里 `span = [...]`。

2.  **Covered Cell 的纯净度**：
    *   如果 Covered Cell 返回了 `themeOverride` (即使是空的)，有时也会触发 GDG 的默认渲染逻辑导致“鬼影线”。
    *   最稳妥的做法是直接 `return` 一个全新的、极简的 `GridCell` 对象。

3.  **DataEditor 的 Props**：
    *   如果不加 `spanRangeBehavior="allow"`，一切计算都是白费。绿框永远不会变大。
