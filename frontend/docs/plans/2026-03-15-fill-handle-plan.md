# Fill Handle + Parser Formula Shift Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable Excel-like fill handle (drag + double-click) and parser-based formula reference shifting, while keeping backend unchanged.

**Architecture:** Keep `fillRange` as the single source of truth for fill logic; add a parser-based reference shifter with regex fallback; wire DataEditor fill handle and double-click auto-fill target calculation into `GlideGrid.tsx`.

**Tech Stack:** React, TypeScript, `@glideapps/glide-data-grid`, `excel-formula-parser`, Vitest.

---

### Task 1: Add parser dependency

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/package.json`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/package-lock.json`

**Step 1: Write the failing test**

Create a parser-driven test that fails because the parser is missing.

**File:** `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/__tests__/formulaShiftParser.test.ts`

```ts
import { shiftFormulaReferencesWithParser } from "../formulaFill";

test("shiftFormulaReferencesWithParser shifts whole-column references", () => {
  const result = shiftFormulaReferencesWithParser("=SUM(F:F)", 1, 0);
  expect(result).toBe("=SUM(G:G)");
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/formulaShiftParser.test.ts`
Expected: FAIL (module or function not found / parser missing)

**Step 3: Add dependency**

Run: `npm install excel-formula-parser`

**Step 4: Run test to verify it still fails**

Run: `npx vitest run src/utils/__tests__/formulaShiftParser.test.ts`
Expected: FAIL (function not implemented)

**Step 5: Commit**

```bash
git add package.json package-lock.json src/utils/__tests__/formulaShiftParser.test.ts
git commit -m "test: add failing parser shift test"
```

---

### Task 2: Implement parser-based formula reference shifting

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/formulaFill.js`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/__tests__/formulaShiftParser.test.ts`

**Step 1: Write the failing tests (extend)**

Add tests covering:

```ts
test("respects absolute column", () => {
  expect(shiftFormulaReferencesWithParser("=$F1", 1, 0)).toBe("=$F1");
});

test("shifts relative row", () => {
  expect(shiftFormulaReferencesWithParser("=A1", 0, 2)).toBe("=A3");
});

test("shifts whole-row references", () => {
  expect(shiftFormulaReferencesWithParser("=SUM(3:3)", 0, 2)).toBe("=SUM(5:5)");
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/formulaShiftParser.test.ts`
Expected: FAIL (function missing / wrong output)

**Step 3: Implement minimal logic**

In `formulaFill.js`:
- Add `shiftFormulaReferencesWithParser(formula, dx, dy)` using `excel-formula-parser`.
- Parse formula to tokens/AST and rewrite references:
  - A1 references (with `$` flags).
  - Whole-column references (`F:F`, `$F:$F`).
  - Whole-row references (`3:3`, `$3:$3`).
- On parse failure, fall back to existing `shiftFormulaReferences`.
- Ensure ASCII-only comments and add change-log notes with date/reason/purpose.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/formulaShiftParser.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/formulaFill.js src/utils/__tests__/formulaShiftParser.test.ts
git commit -m "feat: add parser-based formula shift with fallback"
```

---

### Task 3: Compute auto-fill destination (double-click)

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/fillHandle.ts`
- Create: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/__tests__/fillHandle.test.ts`

**Step 1: Write the failing test**

```ts
import { getAutoFillDestination } from "../fillHandle";

test("uses left column contiguous range first", () => {
  const dest = getAutoFillDestination({
    selection: { x: 2, y: 5, width: 1, height: 1 },
    adjacentColumnValues: ["", "A", "B", "C"],
    startRow: 5
  });
  expect(dest?.y).toBe(5);
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/fillHandle.test.ts`
Expected: FAIL (module missing)

**Step 3: Implement minimal logic**

In `fillHandle.ts` implement:
- Determine contiguous non-empty range downward from `startRow` in the chosen adjacent column.
- Return `Rectangle` destination or `null`.
- Only extend downward; clip to rowCount.
- Add change-log comments with date/reason/purpose.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/fillHandle.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/fillHandle.ts src/utils/__tests__/fillHandle.test.ts
git commit -m "feat: add auto-fill destination helper"
```

---

### Task 4: Wire fill handle + double-click into GlideGrid

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/components/GlideGrid.tsx`

**Step 1: Write the failing test**

Add a unit test (or component test if available) that verifies:
- `fillHandle` is enabled.
- `onFillPattern` calls `fillRange`.
- Double-click triggers auto-fill when adjacent column has data.

If no component test infra exists, add a unit test for a new pure helper used by `GlideGrid.tsx`.

**Step 2: Run test to verify it fails**

Run: `npx vitest run`
Expected: FAIL

**Step 3: Implement minimal code**

- Enable `fillHandle` in `DataEditor`.
- Add `onFillPattern` handler:
  - Call `fillRange(patternSource, fillDestination)`.
- Capture double-click on grid wrapper:
  - Use `getAutoFillDestination` with cache-backed adjacent column values.
  - Call `fillRange` with computed destination.
- Ensure comment ratio 6:4 with change-log notes (date/reason/purpose).

**Step 4: Run test to verify it passes**

Run: `npx vitest run`
Expected: PASS

**Step 5: Commit**

```bash
git add src/components/GlideGrid.tsx
git commit -m "feat: enable fill handle and auto-fill double click"
```

---

### Task 5: Final validation

**Files:**
- None

**Step 1: Run full build**

Run: `npm run build`
Expected: PASS

**Step 2: Run test suite**

Run: `npx vitest run`
Expected: PASS

**Step 3: Commit (if needed)**

```bash
git add -A
git commit -m "chore: validate fill handle changes"
```
