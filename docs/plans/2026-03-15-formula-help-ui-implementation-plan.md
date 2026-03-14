# Formula Help UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在前端界面提供“公式帮助”提示抽屉，展示全量公式用法（双语），并支持搜索过滤。

**Architecture:** 生成器输出 JSON 数据源（与 README 同源），前端通过纯函数过滤并在 App 中渲染右侧抽屉面板。

**Tech Stack:** React (Vite), TypeScript, JSON data, Vitest

---

### Task 1: 建立过滤函数测试（TDD 红）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/__tests__/formulaHelp.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { filterFormulaHelpItems } from "../formulaHelp";

const items = [
  { name: "SUM", syntax: "SUM(range)", example: "=SUM(A1:A5)", paramNotes: "range/范围", purpose: "统计与汇总计算 / Statistical", note: "—" },
  { name: "VLOOKUP", syntax: "VLOOKUP(lookup_value, table)", example: "=VLOOKUP(A1, table)", paramNotes: "lookup_value/查找值", purpose: "查找与引用数据 / Lookup", note: "—" },
];

describe("filterFormulaHelpItems", () => {
  it("returns all items when query is empty", () => {
    expect(filterFormulaHelpItems(items, "").length).toBe(2);
  });

  it("filters by name", () => {
    expect(filterFormulaHelpItems(items, "sum")[0].name).toBe("SUM");
  });

  it("filters by purpose", () => {
    expect(filterFormulaHelpItems(items, "查找")[0].name).toBe("VLOOKUP");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/__tests__/formulaHelp.test.ts`  
Expected: FAIL with “module not found” or “filterFormulaHelpItems is not defined”

**Step 3: Commit**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/__tests__/formulaHelp.test.ts
git commit -m "test: add formula help filter tests"
```

---

### Task 2: 实现过滤函数（TDD 绿）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/formulaHelp.ts`

**Step 1: Implement minimal code**

```ts
export function filterFormulaHelpItems(items, query) {
  // implement minimal filtering
}
```

**Step 2: Run test to verify it passes**

Run: `npx vitest run D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/__tests__/formulaHelp.test.ts`  
Expected: PASS

**Step 3: Commit**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/formulaHelp.ts
git commit -m "feat: add formula help filter"
```

---

### Task 3: 生成 JSON 数据源（与 README 同源）

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/data/formula_help.json`

**Step 1: Write failing test (node:test) for JSON output presence**

Add to `frontend/scripts/tests/generate_formula_docs.test.cjs`:
```js
// assert JSON output contains entries
```

**Step 2: Implement JSON output in generator**

- 生成结构化数组（函数名/语法/示例/参数说明/用途/备注）
- 写入 `src/data/formula_help.json`
- `--check` 模式对 JSON 也进行一致性校验

**Step 3: Run tests**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: PASS

**Step 4: Commit**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/data/formula_help.json D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs
git commit -m "feat: export formula help json"
```

---

### Task 4: 前端 UI 提示抽屉

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/App.tsx`
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/App.css`

**Step 1: Write failing test (minimal UI state)**

Add to `formulaHelp.test.ts`:
```ts
// assert filter applied by query state via helper function (no React test)
```

**Step 2: Implement UI**

- 增加“公式帮助 / Formula Help”按钮
- 抽屉包含搜索框与表格列表
- 调用 `filterFormulaHelpItems`

**Step 3: Run tests**

Run: `npx vitest run D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/utils/__tests__/formulaHelp.test.ts`

**Step 4: Commit**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/App.tsx D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/src/App.css
git commit -m "feat: add formula help drawer"
```

---

### Task 5: 生成 README + 最终验证

**Step 1: Run generator**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`

**Step 2: Run tests**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`

**Step 3: Commit**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/README.md D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/README.md
git commit -m "docs: refresh formula docs output"
```
