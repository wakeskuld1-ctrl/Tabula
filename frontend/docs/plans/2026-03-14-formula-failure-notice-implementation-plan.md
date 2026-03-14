# 公式更新失败提示条 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 公式更新/刷新失败时显示轻量提示条（3 秒自动隐藏），文案统一为“单元格XX更新失败，请重试”。

**Architecture:** 新增纯函数 `buildFormulaFailureNotice` 生成文案，组件内维护 notice 状态与 3 秒自动隐藏。

**Tech Stack:** React + TypeScript + @glideapps/glide-data-grid + Node (脚本式测试)

---

### Task 1: 新增失败用例（TDD）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/tests/buildFormulaFailureNotice.test.ts`

**Step 1: Write the failing test**

```ts
import assert from "node:assert/strict";
import { buildFormulaFailureNotice } from "../src/utils/buildFormulaFailureNotice.js";

const columns = [
  { title: "A" },
  { title: "金额" },
  { title: "C" }
];

const msg1 = buildFormulaFailureNotice(0, 0, columns);
assert.equal(msg1, "单元格 A1 更新失败，请重试");

const msg2 = buildFormulaFailureNotice(1, 2, columns);
assert.equal(msg2, "单元格 金额3 更新失败，请重试");

const msg3 = buildFormulaFailureNotice(5, 9, []);
assert.equal(msg3, "单元格 F10 更新失败，请重试");

console.log("buildFormulaFailureNotice tests passed");
```

**Step 2: Run test to verify it fails**

Run:
```bash
npx tsc tests/buildFormulaFailureNotice.test.ts src/utils/buildFormulaFailureNotice.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
```
Expected: FAIL (找不到模块 `../src/utils/buildFormulaFailureNotice.js`)

---

### Task 2: 创建最小实现（让测试可运行并失败断言）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/utils/buildFormulaFailureNotice.ts`

**Step 1: Write minimal implementation**

```ts
import { getExcelColumnName } from "./formulaRange";

// **[2026-03-14]** 变更原因：统一公式失败提示文案
// **[2026-03-14]** 变更目的：确保提示格式一致
export const buildFormulaFailureNotice = (
  col: number,
  row: number,
  columns: { title?: string }[]
): string => {
  const colTitle = columns[col]?.title || getExcelColumnName(col);
  const rowLabel = row + 1;
  return `单元格 ${colTitle}${rowLabel} 更新失败，请重试`;
};
```

**Step 2: Run test to verify it fails (assertion)**

Run:
```bash
npx tsc tests/buildFormulaFailureNotice.test.ts src/utils/buildFormulaFailureNotice.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/buildFormulaFailureNotice.test.js
```
Expected: FAIL (断言不通过，如果逻辑不完整)

---

### Task 3: 完成实现并让测试通过

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/utils/buildFormulaFailureNotice.ts`

**Step 1: Update implementation (如果断言失败，按最小修改修正)**

```ts
// 如果 Task 2 已满足断言，此处无需改动。
```

**Step 2: Run test to verify it passes**

Run:
```bash
npx tsc tests/buildFormulaFailureNotice.test.ts src/utils/buildFormulaFailureNotice.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/buildFormulaFailureNotice.test.js
```
Expected: PASS (输出 "buildFormulaFailureNotice tests passed")

---

### Task 4: 提示条状态与触发

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/src/components/GlideGrid.tsx`

**Step 1: Add import**

```ts
import { buildFormulaFailureNotice } from "../utils/buildFormulaFailureNotice";
```

**Step 2: Add notice state + helper**

```ts
const [formulaNoticeVisible, setFormulaNoticeVisible] = useState(false);
const [formulaNoticeMessage, setFormulaNoticeMessage] = useState("");
const formulaNoticeTimer = useRef<number | undefined>(undefined);

const showFormulaNotice = useCallback((message: string) => {
  setFormulaNoticeMessage(message);
  setFormulaNoticeVisible(true);
  if (formulaNoticeTimer.current !== undefined) {
    window.clearTimeout(formulaNoticeTimer.current);
  }
  formulaNoticeTimer.current = window.setTimeout(() => {
    setFormulaNoticeVisible(false);
    formulaNoticeTimer.current = undefined;
  }, 3000);
}, []);
```

**Step 3: Trigger on failure**

```ts
showFormulaNotice(buildFormulaFailureNotice(col, row, columns));
```

**Step 4: Render notice bar**

```tsx
{formulaNoticeVisible && (
  <div style={{ position: "absolute", top: 12, right: 12, ... }}>
    {formulaNoticeMessage}
  </div>
)}
```

**Step 5: Run tests to verify**

Run:
```bash
npx tsc tests/buildFormulaFailureNotice.test.ts src/utils/buildFormulaFailureNotice.ts --outDir .tmp-test --module ESNext --target ES2020 --moduleResolution bundler --jsx react-jsx --lib ES2020,DOM --skipLibCheck
node .tmp-test/tests/buildFormulaFailureNotice.test.js
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
git add tests/buildFormulaFailureNotice.test.ts src/utils/buildFormulaFailureNotice.ts src/components/GlideGrid.tsx docs/plans/2026-03-14-formula-failure-notice-implementation-plan.md .trae/CHANGELOG_TASK.md
git commit -m "feat: add formula failure notice"
```


