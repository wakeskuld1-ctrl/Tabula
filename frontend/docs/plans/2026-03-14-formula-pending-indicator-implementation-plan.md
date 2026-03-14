# 公式回显等待态 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 公式更新等待期间在单元格内显示 `⏳ 计算中…`，刷新完成后自动恢复真实数值。

**Architecture:** 使用纯函数 `collectFormulaPendingKeys` 计算公式单元格 key 集合，组件内维护 pending 状态并在 `getCellContent` 中覆盖显示。

**Tech Stack:** React + TypeScript + @glideapps/glide-data-grid + Node (脚本式测试)

---

### Task 1: 新增失败用例（TDD）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/tests/collectFormulaPendingKeys.test.ts`

**Step 1: Write the failing test**

```ts
import assert from "node:assert/strict";
import { GridCellKind, type Item, type TextCell } from "@glideapps/glide-data-grid";
import { collectFormulaPendingKeys } from "../src/utils/collectFormulaPendingKeys.js";

const makeTextCell = (data: string): TextCell => ({
  kind: GridCellKind.Text,
  data,
  displayData: data,
  allowOverlay: true,
  readonly: false
});

const edits = [
  { location: [0, 0] as Item, value: makeTextCell("=SUM(A1:A2)") },
  { location: [1, 150] as Item, value: makeTextCell("=A1+1") },
  { location: [2, 150] as Item, value: makeTextCell("=A1+2") }
];

const pendingKeys = collectFormulaPendingKeys(edits);
const list = Array.from(pendingKeys).sort();
assert.deepEqual(list, ["0,0", "150,1", "150,2"]);

const nonFormulaEdits = [
  { location: [0, 10] as Item, value: makeTextCell("123") }
];
const emptyKeys = collectFormulaPendingKeys(nonFormulaEdits);
assert.deepEqual(Array.from(emptyKeys), []);

console.log("collectFormulaPendingKeys tests passed");
```

**Step 2: Run test to verify it fails**

Run:
```bash
npx tsc tests/collectFormulaPendingKeys.test.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
```
Expected: FAIL (找不到模块 `../src/utils/collectFormulaPendingKeys.js`)

---

### Task 2: 创建最小实现（让测试可运行并失败断言）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/utils/collectFormulaPendingKeys.ts`

**Step 1: Write minimal implementation**

```ts
import { GridCellKind, type EditListItem } from "@glideapps/glide-data-grid";

// **[2026-03-14]** 变更原因：批量公式等待态需要定位单元格
// **[2026-03-14]** 变更目的：提供可测试的 pending key 计算入口
export const collectFormulaPendingKeys = (
  edits: readonly EditListItem[]
): Set<string> => {
  const keys = new Set<string>();
  for (const edit of edits) {
    const row = edit.location[1];
    const col = edit.location[0];
    if (row < 0 || col < 0) continue;
    if (edit.value.kind !== GridCellKind.Text) continue;
    const raw = edit.value.data;
    const text = typeof raw === "string" ? raw : String(raw ?? "");
    if (!text.trim().startsWith("=")) continue;
    keys.add(`${row},${col}`);
  }
  return keys;
};
```

**Step 2: Run test to verify it fails (assertion)**

Run:
```bash
npx tsc tests/collectFormulaPendingKeys.test.ts src/utils/collectFormulaPendingKeys.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/collectFormulaPendingKeys.test.js
```
Expected: FAIL (断言不通过，如果逻辑不完整)

---

### Task 3: 完成实现并让测试通过

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/utils/collectFormulaPendingKeys.ts`

**Step 1: Update implementation (如果断言失败，按最小修改修正)**

```ts
// 如果 Task 2 已满足断言，此处无需改动。
```

**Step 2: Run test to verify it passes**

Run:
```bash
npx tsc tests/collectFormulaPendingKeys.test.ts src/utils/collectFormulaPendingKeys.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/collectFormulaPendingKeys.test.js
```
Expected: PASS (输出 "collectFormulaPendingKeys tests passed")

---

### Task 4: 单元格内等待态渲染与清理

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/components/GlideGrid.tsx`

**Step 1: Add import**

```ts
import { collectFormulaPendingKeys } from "../utils/collectFormulaPendingKeys";
```

**Step 2: Add pending state helpers**

```ts
const [pendingFormulaKeys, setPendingFormulaKeys] = useState<Set<string>>(new Set());

const addPendingFormulaKeys = useCallback((keys: Set<string>) => {
  setPendingFormulaKeys((prev) => {
    const next = new Set(prev);
    keys.forEach((key) => next.add(key));
    return next;
  });
}, []);

const clearPendingFormulaKeys = useCallback((keys: Set<string>) => {
  setPendingFormulaKeys((prev) => {
    const next = new Set(prev);
    keys.forEach((key) => next.delete(key));
    return next;
  });
}, []);
```

**Step 3: Mark pending before submit; clear after refresh/failed**

```ts
const pendingKeys = collectFormulaPendingKeys(newValues);
if (pendingKeys.size > 0) {
  addPendingFormulaKeys(pendingKeys);
}

try {
  // existing batch update
} finally {
  if (pendingKeys.size > 0) {
    clearPendingFormulaKeys(pendingKeys);
  }
}
```

**Step 4: Render waiting text in getCellContent**

```ts
const cellKey = `${row},${col}`;
const waitingDisplay = pendingFormulaKeys.has(cellKey) ? "⏳ 计算中…" : formattedDisplay;

return {
  ...,
  displayData: waitingDisplay,
  copyData: waitingDisplay,
  ...
};
```

**Step 5: Run tests to verify**

Run:
```bash
npx tsc tests/collectFormulaPendingKeys.test.ts src/utils/collectFormulaPendingKeys.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/collectFormulaPendingKeys.test.js
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
git add tests/collectFormulaPendingKeys.test.ts src/utils/collectFormulaPendingKeys.ts src/components/GlideGrid.tsx docs/plans/2026-03-14-formula-pending-indicator-implementation-plan.md .trae/CHANGELOG_TASK.md
git commit -m "feat: show formula pending indicator"
```
