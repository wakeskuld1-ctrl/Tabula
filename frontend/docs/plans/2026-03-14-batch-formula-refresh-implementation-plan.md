# 批量公式回显 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 批量编辑/下拉公式后，按涉及页去重刷新，确保公式结果立即回显且资源可控。

**Architecture:** 将“涉及页计算”抽为纯函数 `collectFormulaPages`，在 `onCellsEdited` 中识别公式输入并在批量提交成功后按页刷新缓存。

**Tech Stack:** React + TypeScript + @glideapps/glide-data-grid + Node (脚本式测试)

---

### Task 1: 新增失败用例（TDD）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/tests/collectFormulaPages.test.ts`

**Step 1: Write the failing test**

```ts
import assert from "node:assert/strict";
import { GridCellKind } from "@glideapps/glide-data-grid";
import { collectFormulaPages } from "../src/utils/collectFormulaPages";

const makeTextCell = (data: string) => ({
  kind: GridCellKind.Text,
  data,
  displayData: data,
  allowOverlay: true,
  readonly: false
});

const edits = [
  { location: [0, 0], value: makeTextCell("=SUM(A1:A2)") },
  { location: [1, 150], value: makeTextCell("=A1+1") },
  { location: [2, 150], value: makeTextCell("=A1+2") }
];

const pages = collectFormulaPages(edits, 100);
const pageList = Array.from(pages).sort((a, b) => a - b);
assert.deepEqual(pageList, [1, 2]);

const nonFormulaEdits = [
  { location: [0, 10], value: makeTextCell("123") }
];
const emptyPages = collectFormulaPages(nonFormulaEdits, 100);
assert.deepEqual(Array.from(emptyPages), []);

console.log("collectFormulaPages tests passed");
```

**Step 2: Run test to verify it fails**

Run:
```bash
npx tsc tests/collectFormulaPages.test.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
```
Expected: FAIL (找不到模块 `../src/utils/collectFormulaPages`)

---

### Task 2: 创建最小实现（让测试可运行并失败断言）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/utils/collectFormulaPages.ts`

**Step 1: Write minimal implementation**

```ts
import { GridCellKind, type EditListItem } from "@glideapps/glide-data-grid";

// **[2026-03-14]** 变更原因：需要统一计算批量公式涉及页
// **[2026-03-14]** 变更目的：为批量公式回显提供可测试的纯函数入口
export const collectFormulaPages = (
  edits: readonly EditListItem[],
  pageSize: number
): Set<number> => {
  const pages = new Set<number>();
  for (const edit of edits) {
    const row = edit.location[1];
    if (row < 0) continue;
    if (edit.value.kind !== GridCellKind.Text) continue;
    const raw = edit.value.data;
    const text = typeof raw === "string" ? raw : String(raw ?? "");
    if (!text.trim().startsWith("=")) continue;
    pages.add(Math.floor(row / pageSize) + 1);
  }
  return pages;
};
```

**Step 2: Run test to verify it fails (assertion)**

Run:
```bash
npx tsc tests/collectFormulaPages.test.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/collectFormulaPages.test.js
```
Expected: FAIL (断言不通过，如果逻辑不完整)

---

### Task 3: 完成实现并让测试通过

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/utils/collectFormulaPages.ts`

**Step 1: Update implementation (如果断言失败，按最小修改修正)**

```ts
// 如果 Task 2 已满足断言，此处无需改动。
```

**Step 2: Run test to verify it passes**

Run:
```bash
npx tsc tests/collectFormulaPages.test.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/collectFormulaPages.test.js
```
Expected: PASS (输出 "collectFormulaPages tests passed")

---

### Task 4: 批量公式回显（按页去重刷新）

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/components/GlideGrid.tsx`

**Step 1: Add import**

```ts
import { collectFormulaPages } from "../utils/collectFormulaPages";
```

**Step 2: Use helper in onCellsEdited**

```ts
// **[2026-03-14]** 变更原因：批量公式不回显
// **[2026-03-14]** 变更目的：提交成功后按涉及页去重刷新
const formulaPages = collectFormulaPages(newValues, PAGE_SIZE);
```

**Step 3: After all chunks succeed, refresh pages**

```ts
// **[2026-03-14]** 变更原因：公式结果依赖后端计算
// **[2026-03-14]** 变更目的：仅刷新涉及页，避免全量刷新
if (formulaPages.size > 0) {
  for (const page of formulaPages) {
    cache.current.delete(page);
    fetchPage(page);
  }
}
```

**Step 4: Run test to verify it still passes**

Run:
```bash
npx tsc tests/collectFormulaPages.test.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/collectFormulaPages.test.js
```
Expected: PASS

---

### Task 5: 清理临时产物（可选）

**Step 1: Remove temp build output**

```bash
Remove-Item -Recurse -Force .tmp-test
```

---

### Task 6: Commit

```bash
git add tests/collectFormulaPages.test.ts src/utils/collectFormulaPages.ts src/components/GlideGrid.tsx
git commit -m "fix: refresh pages for batch formula edits"
```
