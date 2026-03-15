# Formula Tips Popup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make formula help appear only on `=` input or `fx` click, default collapsed to one-line summaries, and keep table selector + Pivot on one row.

**Architecture:** Add small, testable helper functions for visibility and summary formatting; UI consumes helpers and maintains per-item expansion state. CSS updates provide single-row top bar and compact tips.

**Tech Stack:** React + TypeScript, Vite, CSS, vitest (via npx).

---

### Task 1: Add failing tests for formula help visibility + summary

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaHelp.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, expect, it } from 'vitest';
import { shouldShowFormulaHelp, formatFormulaTipSummary } from '../formulaHelp';

const baseItem = {
  name: 'ABS',
  syntax: 'ABS(number)',
  example: '=ABS(1)',
  paramNotes: 'number/数值',
  purpose: '数学计算与取整 / Math and rounding',
  note: '—'
};

describe('formula help visibility', () => {
  it('shows when input starts with =', () => {
    expect(shouldShowFormulaHelp({ text: '=A', isFxToggled: false })).toBe(true);
  });

  it('shows when fx is toggled', () => {
    expect(shouldShowFormulaHelp({ text: 'A1', isFxToggled: true })).toBe(true);
  });

  it('hides when no trigger', () => {
    expect(shouldShowFormulaHelp({ text: 'A1', isFxToggled: false })).toBe(false);
  });
});

describe('formula help summary', () => {
  it('formats purpose + syntax', () => {
    expect(formatFormulaTipSummary(baseItem)).toBe('数学计算与取整 / Math and rounding =ABS(number)');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaHelp.test.ts`
Expected: FAIL (functions missing)

**Step 3: Write minimal implementation**
- Add `shouldShowFormulaHelp` + `formatFormulaTipSummary` to `formulaHelp.ts`.

**Step 4: Run test to verify it passes**

Run: `npx vitest run D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaHelp.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\formulaHelp.ts D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaHelp.test.ts
git commit -m "test: add formula help visibility and summary helpers"
```

---

### Task 2: Update FormulaBar to use conditional, collapsible tips

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\layout\FormulaBar.tsx`

**Step 1: Write the failing test**
- Add a small render test if feasible; if not, rely on helper tests (document reason).

**Step 2: Run test to verify it fails**
- Same vitest command as Task 1.

**Step 3: Write minimal implementation**
- Show tips only when `shouldShowFormulaHelp(...)` is true.
- Default collapsed list; click toggles `expandedTips` by name.
- Keep a minimal line layout using `formatFormulaTipSummary(...)`.

**Step 4: Run test to verify it passes**
- `npx vitest run ...`

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\layout\FormulaBar.tsx
git commit -m "feat: show formula tips on trigger with collapsible rows"
```

---

### Task 3: Update CSS for compact tips + one-line status bar

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`

**Step 1: Write the failing test**
- Manual visual check (documented) since CSS is not unit-testable here.

**Step 2: Run check to verify it fails**
- Before change, status bar wraps and tips occupy full height.

**Step 3: Write minimal implementation**
- Add `.status-bar-left` wrapper in JSX for select + pivot.
- CSS: keep `status-bar` in a single line and clamp tips height.

**Step 4: Run check to verify it passes**
- Visual: table selector + pivot on same row.
- Tips panel only appears on trigger and shows collapsed rows.

**Step 5: Commit**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx
git commit -m "style: keep status bar in one row and compact formula tips"
```

---

### Task 4: Verify API error causes (no code)

**Files:**
- None

**Step 1: Verify backend responses**
- `GET http://localhost:3000/api/versions?...` returns 404
- `POST http://localhost:3000/api/update_style_range` returns 405

**Step 2: Record finding**
- Note in final summary that these are backend route gaps (not front-end proxy issues).

---

### Task 5: Task journal update

**Files:**
- Append to `D:\Rust\metadata\.trae\CHANGELOG_TASK.md`

**Step 1: Add entry**
- Summarize changes, reason, risks, and remaining items.

**Step 2: No history edits**
- Only append new entry.

---

Plan complete and saved to `docs/plans/2026-03-15-formula-tips-popup.md`. Two execution options:

1. Subagent-Driven (this session) - I dispatch fresh subagent per task, review between tasks, fast iteration
2. Parallel Session (separate) - Open new session with executing-plans, batch execution with checkpoints

Which approach?
