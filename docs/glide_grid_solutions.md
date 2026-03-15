# Glide Data Grid 解决方案与最佳实践文档

## 1. 背景与问题
在开发基于 Glide Data Grid (GDG) 的表格组件时，我们遇到了以下主要挑战：
1.  **合并单元格 (Merge Cells)**：需要实现类似 Excel 的合并/解除合并功能。
2.  **视觉刷新 (Visual Refresh)**：合并操作后，Grid 未能及时刷新显示最新的合并状态。
3.  **边框样式 (Border Styling)**：合并后的单元格区域仍显示默认的灰色网格线，导致视觉上未完全合并。

本文档基于 Glide Data Grid 官方文档及社区最佳实践，整理了针对上述问题的解决方案。

## 2. 核心架构理解 (Official Architecture)
Glide Data Grid 是一个**高性能、无状态**的渲染引擎。
*   **Event-Based**: GDG 不直接管理数据状态。它完全依赖 `getCellContent` 回调函数从外部数据源（Backing Store）拉取数据。
*   **On-Demand**: 仅渲染视口内的单元格。
*   **Immutability**: Grid 假设数据是不可变的，除非明确通知更新。

## 3. 刷新机制解决方案 (Refresh Solutions)

由于 GDG 不会自动检测外部数据的变化，我们需要显式触发更新。

### 方案一：依赖注入 (Dependency Injection) - **推荐**
这是官方推荐的最通用做法。通过 React 的 `useCallback` 依赖数组机制，强制 `getCellContent` 函数在数据版本变化时重新创建。

**原理**：
当 `getCellContent` 的引用发生变化时，GDG 会认为数据源可能已改变，从而触发全量重绘。

**代码示例**：
```typescript
const [dataVersion, setDataVersion] = useState(0);

const getCellContent = useCallback((cell: Item): GridCell => {
    // ... 获取数据的逻辑
}, [dataVersion]); // <--- 关键：将 version 加入依赖

// 当发生合并/编辑操作后：
setDataVersion(v => v + 1);
```

### 方案二：细粒度更新 (Granular Update / lastUpdated)
对于高性能要求的场景，全量重绘可能太重。可以使用 `GridCell` 接口中的 `lastUpdated` 属性。

**原理**：
`lastUpdated` 接受一个 `performance.now()` 时间戳。当该值变化时，GDG 会仅重绘该特定单元格（通常伴随一个高亮 Flash 动画，但可配置）。

**代码示例**：
```typescript
return {
    kind: GridCellKind.Text,
    displayData: "Value",
    // ...
    lastUpdated: performance.now(), // 强制该单元格重绘
};
```

## 4. 合并单元格与边框视觉解决方案 (Merge & Borders)

GDG 通过 `span` 属性支持合并，但默认不会处理被覆盖单元格的边框渲染。

### 方案三：样式覆盖 (Theme Override) - **当前采用**
通过 `themeOverride` 属性，在单元格级别覆盖全局主题样式。

**原理**：
将合并区域（主单元格）的边框颜色设置为与背景色一致，从而在视觉上"隐藏"网格线。

**代码示例**：
```typescript
if (isMergeStart) {
    themeOverride = {
        bgCell: "#ffffff", // 背景色
        borderHorizontal: "#ffffff", // 水平边框同背景色
        borderVertical: "#ffffff",   // 垂直边框同背景色
    };
}
```

### 方案四：自定义绘制 (Custom Renderer / drawCell) - **高级备选**
如果 `themeOverride` 无法满足需求（例如需要复杂的边框样式或完全透明），可以使用 `drawCell` 回调。

**原理**：
`drawCell` 允许完全接管 Canvas 的绘制过程。可以先绘制自定义背景，再调用默认绘制逻辑。

**代码示例**：
```typescript
const drawCell = useCallback((args, draw) => {
    const { ctx, rect, theme } = args;
    // 1. 自定义绘制背景
    ctx.fillStyle = theme.bgCell;
    ctx.fillRect(rect.x, rect.y, rect.width, rect.height);
    
    // 2. 调用默认绘制（绘制文字等）
    draw();
    
    // 3. 绘制自定义覆盖层（如选中框）
}, []);
```

## 5. 当前实现状态 (Implementation Status)

我们在 `GlideGrid.tsx` 中集成了以下方案：

1.  **刷新机制**：采用了 **方案一 (Dependency Injection)** 和 **方案二 (lastUpdated)** 的混合模式。
    *   引入 `version` 状态。
    *   `getCellContent` 依赖 `version`。
    *   合并单元格的 `lastUpdated` 绑定 `version`，确保合并操作后立即刷新。

2.  **边框处理**：采用了 **方案三 (Theme Override)**。
    *   在检测到合并起始单元格时，动态生成 `themeOverride`。
    *   强制将 `borderHorizontal` 和 `borderVertical` 设置为与 `bgCell` 相同的颜色（默认白色或样式背景色）。

## 6. 参考资料
*   [Glide Data Grid Official Docs](https://docs.grid.glideapps.com/)
*   [GitHub Repository](https://github.com/glideapps/glide-data-grid)
*   [GDG API - BaseGridCell](https://docs.grid.glideapps.com/api/cells/basegridcell)
