# Glide DataEditor Capability Expansion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 按既定优先级补齐 Glide DataEditor 能力缺口，提升行级状态展示、滚动体验、表头一致性与批量编辑闭环。

**Architecture:** 以 `GlideGrid.tsx` 为中心统一注入 DataEditor props，状态来源聚合在本组件内部（公式元信息、筛选状态、行尺寸等）。新增能力尽量复用现有脚本验证链路（`verify_state_integration.cjs` 等）。

**Tech Stack:** React 18、Vite、@glideapps/glide-data-grid、Puppeteer 脚本验证

---

### Task 1: 接入 getRowThemeOverride（P0）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/src/scripts/verify_state_integration.cjs`

**Step 1: Write the failing test**
```javascript
// 在 verify_state_integration.cjs 新增用例：
// 1) 设置某行公式为过期态（复用现有 stale 逻辑或刷新逻辑）
// 2) 读取该行单元格的渲染颜色（像素采样或暴露只读调试值）
// 3) 断言该行使用了 row theme 的 bgCell/text 颜色
```

**Step 2: Run test to verify it fails**
Run: `node frontend/src/scripts/verify_state_integration.cjs`  
Expected: row theme 未生效，颜色断言失败

**Step 3: Write minimal implementation**
```tsx
// 在 GlideGrid.tsx：
// 1) 从 formulaMetaMap 推导 staleRows Set（useMemo）
// 2) 添加 getRowThemeOverride，命中 staleRows 返回 Partial<Theme>
// 3) 保持函数轻量无异步，避免 render 性能下降
// 4) 按要求添加 Markdown 格式变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Run: `node frontend/src/scripts/verify_state_integration.cjs`  
Expected: row theme 生效，颜色断言通过

---

### Task 2: 接入 overscrollX / overscrollY（P1）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/TEST_CASES.md`（补充用例步骤）

**Step 1: Write the failing test**
```
新增手工用例：末列缩放时可继续拖动一定距离；滚动到边界不突兀
```

**Step 2: Run test to verify it fails**
Manual: 末列缩放体验不佳或无法超出边界

**Step 3: Write minimal implementation**
```tsx
// 在 DataEditor props 添加 overscrollX/overscrollY
// 数值以列宽/行高为基准（如 80~120px）
// 添加变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Manual: 末列缩放与滚动体验提升

---

### Task 3: 接入 headerIcons / headerHeight（P2）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/src/scripts/verify_state_integration.cjs`（新增表头可见性断言）

**Step 1: Write the failing test**
```javascript
// 检查列头 icon 存在且 svg 注入成功（读取 DOM 中 svg）
```

**Step 2: Run test to verify it fails**
Run: `node frontend/src/scripts/verify_state_integration.cjs`  
Expected: 未找到自定义 headerIcons

**Step 3: Write minimal implementation**
```tsx
// 1) 提供 headerIcons 映射（函数返回 svg 字符串）
// 2) 在 columns 中使用自定义 icon 名称
// 3) 设置 headerHeight（保持一致性）
// 4) 添加变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Run: `node frontend/src/scripts/verify_state_integration.cjs`  
Expected: 自定义图标可见，表头高度一致

---

### Task 4: 接入 onCellsEdited（P3）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/verify_tc15.js`、`frontend/verify_tc16.js` 或 `frontend/src/scripts/verify_state_integration.cjs`

**Step 1: Write the failing test**
```
批量粘贴 3x3 区域后，期望所有单元格值正确且只走一次批量更新入口
```

**Step 2: Run test to verify it fails**
Run: `node frontend/verify_tc15.js`  
Expected: 仅单格更新或缺失批量入口

**Step 3: Write minimal implementation**
```tsx
// 1) 添加 onCellsEdited
// 2) 将 onCellEdited 的核心更新逻辑抽出复用
// 3) 批量更新时维护 undo 栈一致性
// 4) 添加变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Run: `node frontend/verify_tc15.js`  
Expected: 批量更新正确且无异常弹窗

---

### Task 5: 接入 rightElement / rightElementProps（P4）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/scripts/smoke_test.js`

**Step 1: Write the failing test**
```
检查右侧扩展区 DOM 是否存在（通过 data-testid）
```

**Step 2: Run test to verify it fails**
Run: `node frontend/scripts/smoke_test.js`  
Expected: 未找到右侧扩展区

**Step 3: Write minimal implementation**
```tsx
// 1) 添加 rightElement（展示筛选数量或提示文案）
// 2) 配置 rightElementProps（sticky/fill）
// 3) 添加变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Run: `node frontend/scripts/smoke_test.js`  
Expected: 右侧扩展区可见且不遮挡网格

---

### Task 6: 接入 verticalBorder / scaleToRem（P5）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/TEST_CASES.md`（补充视觉项）

**Step 1: Write the failing test**
```
确认列竖线可控、REM 缩放后字号一致
```

**Step 2: Run test to verify it fails**
Manual: 竖线与字号仍为默认

**Step 3: Write minimal implementation**
```tsx
// 1) 添加 verticalBorder（全局或按列策略）
// 2) 添加 scaleToRem（与全局字号体系一致）
// 3) 添加变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Manual: 竖线与字号表现正确

---

### Task 7: 接入 getGroupDetails / groupHeaderHeight（P6）

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Test: `frontend/src/scripts/verify_state_integration.cjs`

**Step 1: Write the failing test**
```javascript
// 渲染分组头后检查 group header 文本与样式
```

**Step 2: Run test to verify it fails**
Run: `node frontend/src/scripts/verify_state_integration.cjs`  
Expected: 未出现分组头

**Step 3: Write minimal implementation**
```tsx
// 1) 依据列类型构造分组结构
// 2) 提供 getGroupDetails 与 groupHeaderHeight
// 3) 添加变更记录（日期/原因/目的）
```

**Step 4: Run test to verify it passes**
Run: `node frontend/src/scripts/verify_state_integration.cjs`  
Expected: 分组头显示正确

---

### Global Verification Steps (after all tasks)

1. `node frontend/src/scripts/verify_state_integration.cjs`
2. `node frontend/scripts/smoke_test.js`
3. 手工回归：滚动、列缩放、表头图标、分组头

> **Note:** 代码变更需遵循“备注比例 ≥ 60%”与“变更原因/目的/日期”的注释要求。
