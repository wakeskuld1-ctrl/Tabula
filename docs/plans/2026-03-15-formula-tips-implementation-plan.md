# Formula Tips (Always-On) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在公式栏下方常驻显示公式 tips，并支持输入过滤。

**Architecture:** 新增纯函数 `selectFormulaHelpItems` 处理过滤/默认展示；FormulaBar 直接渲染常驻列表；样式通过固定高度与滚动控制。

**Tech Stack:** React, TypeScript, CSS, Vitest

---

### Task 1: 写失败测试（TDD RED）

**Files:**
- Modify: `frontend/src/utils/__tests__/formulaHelp.test.ts`

**Step 1: 新增 selectFormulaHelpItems 的用例**

```ts
import { filterFormulaHelpItems, selectFormulaHelpItems } from "../formulaHelp";

it("returns top items when query is empty", () => {
  const result = selectFormulaHelpItems(sampleItems, "", 1);
  expect(result.length).toBe(1);
  expect(result[0].name).toBe("SUM");
});

it("filters when query is provided", () => {
  const result = selectFormulaHelpItems(sampleItems, "=VLOOK", 5);
  expect(result[0].name).toBe("VLOOKUP");
});
```

**Step 2: 运行测试确认失败**

Run: `npx vitest run frontend/src/utils/__tests__/formulaHelp.test.ts`
Expected: FAIL（因为 `selectFormulaHelpItems` 未实现）

---

### Task 2: 实现 selectFormulaHelpItems（TDD GREEN）

**Files:**
- Modify: `frontend/src/utils/formulaHelp.ts`

**Step 1: 新增函数**

```ts
export function selectFormulaHelpItems(items: FormulaHelpItem[], query: string, limit: number) {
  // ...
}
```

**Step 2: 运行测试确认通过**

Run: `npx vitest run frontend/src/utils/__tests__/formulaHelp.test.ts`
Expected: PASS

---

### Task 3: FormulaBar 常驻 tips UI

**Files:**
- Modify: `frontend/src/components/layout/FormulaBar.tsx`

**Step 1: 引入数据与选择器**

```ts
import formulaHelpData from "../../data/formula_help.json";
import { selectFormulaHelpItems } from "../../utils/formulaHelp";
```

**Step 2: 计算 tips 列表并渲染常驻区域**

- 使用 `selectFormulaHelpItems`，当输入为空时显示 Top N
- 渲染标题、空态提示、列表项

---

### Task 4: 样式调整

**Files:**
- Modify: `frontend/src/App.css`

**Step 1: 新增 formula-tips 样式**

- `.formula-bar` 改为 column，新增 `.formula-bar-row`
- `.formula-tips` 固定高度、滚动区域

---

### Task 5: 记录日志

**Files:**
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: 记录 TDD 与 UI 变更**

---

### Task 6: 建议验证

- `npx vitest run frontend/src/utils/__tests__/formulaHelp.test.ts`
- `npm run build`（可选）
