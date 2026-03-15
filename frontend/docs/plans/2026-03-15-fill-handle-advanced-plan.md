# Fill Handle Advanced Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refine fill-handle hitbox, auto-prefetch missing pages on double-click, and add structured reference column mapping.

**Architecture:** Keep `fillRange` as the single writer. Add a hitbox helper, a prefetch loop over `fetchPage`, and a structured-reference mapper that uses current grid columns for name-to-name mapping with safe fallback.

**Tech Stack:** React, TypeScript, `@glideapps/glide-data-grid`, Vitest.

---

### Task 1: Hitbox helper (TDD)

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/__tests__/fillHandleHitbox.test.ts`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/fillHandle.ts`

**Step 1: Write the failing test**

```ts
import { describe, expect, test } from "vitest";
import { isFillHandleHit } from "../fillHandle";

describe("isFillHandleHit", () => {
  test("hits within dynamic handle size", () => {
    const hit = isFillHandleHit({
      bounds: { x: 100, y: 100, width: 80, height: 40 },
      point: { x: 175, y: 135 },
      tolerance: 2
    });
    expect(hit).toBe(true);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/fillHandleHitbox.test.ts`  
Expected: FAIL (function missing)

**Step 3: Write minimal implementation**

Add `isFillHandleHit` in `fillHandle.ts`:
- Compute `handleSize = Math.min(10, bounds.height * 0.25)`
- Compute handle rect at bottom-right
- Apply `tolerance` padding

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/fillHandleHitbox.test.ts`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/fillHandle.ts src/utils/__tests__/fillHandleHitbox.test.ts
git commit -m "feat: add fill handle hitbox helper"
```

---

### Task 2: Prefetch missing pages for double-click fill (TDD)

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/__tests__/fillHandlePrefetch.test.ts`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/fillHandle.ts`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/components/GlideGrid.tsx`

**Step 1: Write the failing test**

```ts
import { describe, expect, test } from "vitest";
import { buildPrefetchPlan } from "../fillHandle";

describe("buildPrefetchPlan", () => {
  test("limits pages by maxPages", () => {
    const plan = buildPrefetchPlan({ startRow: 0, rowCount: 1000, pageSize: 100, maxPages: 3 });
    expect(plan.length).toBe(3);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/fillHandlePrefetch.test.ts`  
Expected: FAIL (function missing)

**Step 3: Write minimal implementation**

Add `buildPrefetchPlan` in `fillHandle.ts`:
- Return a list of page numbers to fetch
- Respect `maxPages` and `maxRows` bounds

**Step 4: Wire into GlideGrid**

In `handleFillHandleDoubleClick`:
- When scanning reaches uncached row, call `fetchPage(page)`
- Continue scanning until blank or bound reached

**Step 5: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/fillHandlePrefetch.test.ts`  
Expected: PASS

**Step 6: Commit**

```bash
git add src/utils/fillHandle.ts src/utils/__tests__/fillHandlePrefetch.test.ts src/components/GlideGrid.tsx
git commit -m "feat: prefetch pages for double-click fill"
```

---

### Task 3: Structured reference mapping (TDD)

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/__tests__/structuredRefShift.test.ts`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/formulaFill.js`
- Modify: `D:/Rust/metadata/.worktrees/formula-tips/frontend/src/utils/formulaFill.d.ts`

**Step 1: Write the failing test**

```ts
import { describe, expect, test } from "vitest";
import { shiftStructuredReferences } from "../formulaFill";

describe("shiftStructuredReferences", () => {
  test("maps table column names by dx", () => {
    const columns = ["Sales", "Profit", "Cost"];
    const result = shiftStructuredReferences("=Table1[Sales]", 1, columns);
    expect(result).toBe("=Table1[Profit]");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/structuredRefShift.test.ts`  
Expected: FAIL (function missing)

**Step 3: Write minimal implementation**

In `formulaFill.js`:
- Add `shiftStructuredReferences(formula, dx, columns)`
- Parse `Table[Column]`, `[@Column]`, `Table[[#Headers],[Column]]`
- Map column names by index within `columns`
- If missing/out-of-range, keep original

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/structuredRefShift.test.ts`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/formulaFill.js src/utils/formulaFill.d.ts src/utils/__tests__/structuredRefShift.test.ts
git commit -m "feat: add structured reference mapping"
```

---

### Task 4: Final validation

**Files:**
- None

**Step 1: Run full tests**

Run: `npx vitest run`  
Expected: PASS

**Step 2: Run build**

Run: `npm run build`  
Expected: PASS

**Step 3: Commit (if needed)**

```bash
git add -A
git commit -m "chore: validate fill handle advanced changes"
```
