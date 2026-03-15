# Header Single-Row + Brand Rename Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename brand to Tabula and render the entire top bar on a single row.

**Architecture:** Extract small, testable helpers for brand title and layout grouping; update App layout to use a single-row container; adjust CSS to prevent wrapping and ensure ellipsis.

**Tech Stack:** React + TypeScript, Vite, CSS, vitest (via npx).

---

### Task 1: Add failing tests for brand title + layout grouping

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\headerLayout.test.ts`
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\headerLayout.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { getBrandTitle, getHeaderGroups } from "../headerLayout";

describe("header layout", () => {
  it("uses Tabula brand title", () => {
    expect(getBrandTitle()).toBe("Tabula");
  });

  it("groups table selector with pivot", () => {
    const groups = getHeaderGroups();
    expect(groups.left.includes("table-selector")).toBe(true);
    expect(groups.left.includes("pivot")).toBe(true);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\headerLayout.test.ts`
Expected: FAIL (module missing)

**Step 3: Write minimal implementation**

```ts
export function getBrandTitle() {
  return "Tabula";
}

export function getHeaderGroups() {
  return {
    left: ["brand", "table-selector", "pivot"],
    right: ["status-label", "status-chip", "status-debug"],
  };
}
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\headerLayout.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\headerLayout.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\headerLayout.test.ts
git commit -m "test: add header layout helpers"
```

---

### Task 2: Update App layout to single-row header

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`

**Step 1: Write the failing test**
- Re-run headerLayout tests; App uses helpers after implementation.

**Step 2: Run test to verify it fails**
- `npx vitest run ...` (tests should already be green)

**Step 3: Write minimal implementation**
- Replace brand title with `getBrandTitle()`.
- Move table selector + pivot into header left group.
- Place status labels on header right group.

**Step 4: Run test to verify it passes**
- `npx vitest run ...` (ensure still green)

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx
git commit -m "feat: merge header into single row"
```

---

### Task 3: Update CSS for single-row behavior

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css`

**Step 1: Write the failing test**
- Manual visual check note (CSS visual change).

**Step 2: Run check to verify it fails**
- Before change, status bar wraps.

**Step 3: Write minimal implementation**
- Add `status-header` flex rules to prevent wrap.
- Ensure text overflow ellipsis on debug/status label.

**Step 4: Run check to verify it passes**
- Visual: top area is a single row.

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css
git commit -m "style: keep header in one row"
```

---

### Task 4: Task journal update

**Files:**
- Append to `D:\Rust\metadata\.trae\CHANGELOG_TASK.md`

**Step 1: Add entry**
- Summarize changes, reason, risks, and remaining items.

**Step 2: No history edits**
- Only append new entry.

---

Plan complete and saved to `docs/plans/2026-03-15-header-single-row-plan.md`. Two execution options:

1. Subagent-Driven (this session) - I dispatch fresh subagent per task, review between tasks, fast iteration
2. Parallel Session (separate) - Open new session with executing-plans, batch execution with checkpoints

Which approach?
