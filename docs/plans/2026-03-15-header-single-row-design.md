# Header Single-Row + Brand Rename Design

**Date:** 2026-03-15

## 背景
- 当前品牌标题仍为 `Trae Excel`，需要统一为 `Tabula`。
- 顶部状态区域仍为两行，用户要求“合并成一行”。

## 目标
- 将品牌标题改为 `Tabula`。
- 把品牌、表选择、Pivot、状态信息合并到同一行展示。
- 保持可读性，超出时使用省略。

## 非目标
- 不调整后端接口行为。
- 不改变功能逻辑，仅做布局与文案调整。

## 设计

### 1) 结构布局
- 以 `status-header` 为主行容器，承载：
  - 左侧：品牌标题 + Sandbox 标签 + 表选择 + Pivot
  - 右侧：Fetching/Backend 状态/Debug
- 原 `status-bar` 可保留为容器名，但内容合并入同一行。

### 2) 交互与可访问性
- 表选择与 Pivot 保持原 `aria-label` 与 `title`。
- 状态文本保持 `aria-live`。

### 3) 文案
- 品牌标题统一改为 `Tabula`。

## 风险
- 右侧状态区域在窄屏下可能拥挤，需要省略策略。

## 验收
- 品牌标题显示为 `Tabula`。
- 表选择与 Pivot 与品牌同一行。
- 顶部区域不再占用两行高度。
