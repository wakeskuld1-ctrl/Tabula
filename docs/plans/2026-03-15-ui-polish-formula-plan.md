# UI Polish + Formula Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Auto-hide Loaded overlay, move Sheet add button next to tabs, make Pivot toggle reliable, and preserve formula raw strings while showing computed values.

**Architecture:** Introduce small pure helpers (debug overlay timing, sheet bar model, formula store/override rules, pivot prefetch rules) with TDD. Wire helpers into App/SheetBar/GlideGrid and adjust CSS.

**Tech Stack:** React + TypeScript, Vite, CSS, vitest (via npx).

---

### Task 1: Add failing tests for debug overlay auto-hide

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\debugOverlay.ts`
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\debugOverlay.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { getAutoHideDelayMs, shouldAutoHideDebugInfo } from "../debugOverlay";

describe("debug overlay auto hide", () => {
  it("auto hides loaded rows message", () => {
    const msg = "Loaded foo: 90 rows";
    expect(shouldAutoHideDebugInfo({ message: msg, loading: false })).toBe(true);
    expect(getAutoHideDelayMs(msg)).toBe(10000);
  });

  it("does not auto hide non-load messages", () => {
    expect(shouldAutoHideDebugInfo({ message: "Save completed", loading: false })).toBe(false);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\debugOverlay.test.ts`
Expected: FAIL (module missing)

**Step 3: Write minimal implementation**
- Implement helpers in `debugOverlay.ts`.

**Step 4: Run test to verify it passes**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\debugOverlay.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\debugOverlay.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\debugOverlay.test.ts
git commit -m "test: add debug overlay auto-hide helpers"
```

---

### Task 2: Wire debug overlay auto-hide into App

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`

**Step 1: Write the failing test**
- Reuse helper tests (already red/green). Document UI effect in notes.

**Step 2: Implement**
- Add `useEffect` to clear `debugInfo` 10s after load completion.
- Use helpers from `debugOverlay.ts`.

**Step 3: Run test to verify it passes**
- `npx vitest run ...debugOverlay.test.ts`

**Step 4: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx
git commit -m "feat: auto-hide loaded overlay after delay"
```

---

### Task 3: Add failing tests for SheetBar add placement

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\sheetBarModel.ts`
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\sheetBarModel.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { buildSheetItems } from "../sheetBarModel";

describe("sheet bar model", () => {
  it("places add button after last tab", () => {
    const items = buildSheetItems([{ sessionId: "s1", displayName: "S1", isDefault: false }]);
    expect(items[items.length - 1].type).toBe("add");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\sheetBarModel.test.ts`
Expected: FAIL (module missing)

**Step 3: Write minimal implementation**
- Implement `buildSheetItems` and use in `SheetBar.tsx` to render tabs + add button inline.

**Step 4: Run test to verify it passes**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\sheetBarModel.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\sheetBarModel.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\sheetBarModel.test.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\layout\SheetBar.tsx
git commit -m "feat: place sheet add button next to tabs"
```

---

### Task 4: Add failing tests for Pivot prefetch decision

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\pivotToggle.ts`
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\pivotToggle.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { shouldPrefetchPivotFields } from "../pivotToggle";

describe("pivot toggle", () => {
  it("prefetches when table exists but fields are empty", () => {
    expect(shouldPrefetchPivotFields({ tableName: "t", fieldsCount: 0 })).toBe(true);
  });

  it("does not prefetch when fields already loaded", () => {
    expect(shouldPrefetchPivotFields({ tableName: "t", fieldsCount: 2 })).toBe(false);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\pivotToggle.test.ts`
Expected: FAIL

**Step 3: Implement helper + App update**
- Implement `shouldPrefetchPivotFields`.
- Update `handlePivotToggle` to prefetch if needed.
- Fix incorrect debug message after pivot apply.

**Step 4: Run test to verify it passes**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\pivotToggle.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\pivotToggle.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\pivotToggle.test.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx
git commit -m "feat: stabilize pivot toggle and messaging"
```

---

### Task 5: Add failing tests for formula persistence

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\formulaPersistence.ts`
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaPersistence.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { resolveFormulaPersistence } from "../formulaPersistence";

describe("formula persistence", () => {
  it("keeps formula as stored value and returns display override", () => {
    const result = resolveFormulaPersistence({ input: "=SUM(A1:A2)", computed: "3" });
    expect(result.storedValue).toBe("=SUM(A1:A2)");
    expect(result.displayOverride).toBe("3");
  });

  it("passes through non-formula values", () => {
    const result = resolveFormulaPersistence({ input: "123", computed: "123" });
    expect(result.storedValue).toBe("123");
    expect(result.displayOverride).toBeNull();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaPersistence.test.ts`
Expected: FAIL

**Step 3: Implement helper + GlideGrid update**
- Use helper in `onCellEdited` to store formula raw value.
- Add `formulaDisplayOverrides` map and consult it in `getCellContent`.

**Step 4: Run test to verify it passes**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaPersistence.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\formulaPersistence.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaPersistence.test.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\GlideGrid.tsx
git commit -m "fix: preserve formula raw value with display overrides"
```

---

### Task 6: Update CSS for SheetBar add placement

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css`

**Step 1: Manual visual check**
- Confirm add button sits immediately after tabs.

**Step 2: Implement minimal CSS**
- Adjust `.sheet-add` margins and align with tabs.

**Step 3: Verify visually**

**Step 4: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css
git commit -m "style: align sheet add with tabs"
```

---

### Task 7: Task journal update

**Files:**
- Append to `D:\Rust\metadata\.trae\CHANGELOG_TASK.md`

**Step 1: Add entry**
- Summarize changes, reason, risks, and remaining items.

**Step 2: No history edits**
- Only append new entry.

---

Plan complete and saved to `docs/plans/2026-03-15-ui-polish-formula-plan.md`. Two execution options:

1. Subagent-Driven (this session) - I dispatch fresh subagent per task, review between tasks, fast iteration
2. Parallel Session (separate) - Open new session with executing-plans, batch execution with checkpoints

Which approach?
