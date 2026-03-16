# Readonly Write Guard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Block all write actions in readonly/default sessions with an alert, and improve create_session reliability so Sheet1 is not skipped.

**Architecture:** Add a small write-guard utility for consistent checks, then wire it into App/Toolbar/FormulaBar/GlideGrid write entry points. Improve create_session parsing with a fallback to session list when response lacks session_id, and omit from_session_id when empty.

**Tech Stack:** React + TypeScript, Vite, Vitest

---

### Task 1: Add write-guard utility + tests (TDD)

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/utils/sessionWriteGuard.ts`
- Test: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/utils/__tests__/sessionWriteGuard.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, it, expect } from "vitest";
import { getWriteGuardState, READONLY_ALERT_MESSAGE } from "../sessionWriteGuard";

describe("sessionWriteGuard", () => {
  it("blocks write when sessionId is empty", () => {
    const state = getWriteGuardState({ sessionId: "", isReadOnly: false });
    expect(state.canWrite).toBe(false);
    expect(state.message).toBe(READONLY_ALERT_MESSAGE);
  });

  it("blocks write when readonly", () => {
    const state = getWriteGuardState({ sessionId: "s1", isReadOnly: true });
    expect(state.canWrite).toBe(false);
    expect(state.message).toBe(READONLY_ALERT_MESSAGE);
  });

  it("allows write when sessionId exists and not readonly", () => {
    const state = getWriteGuardState({ sessionId: "s1", isReadOnly: false });
    expect(state.canWrite).toBe(true);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/sessionWriteGuard.test.ts`  
Expected: FAIL (module not found / functions missing)

**Step 3: Write minimal implementation**

```typescript
export const READONLY_ALERT_MESSAGE = "请先创建新 Sheet（session）再编辑/保存";

export function getWriteGuardState(input: { sessionId?: string; isReadOnly?: boolean }) {
  const sessionId = (input.sessionId ?? "").trim();
  const isReadOnly = Boolean(input.isReadOnly);
  const canWrite = Boolean(sessionId) && !isReadOnly;
  return {
    canWrite,
    message: canWrite ? "" : READONLY_ALERT_MESSAGE,
  };
}
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/sessionWriteGuard.test.ts`  
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/utils/sessionWriteGuard.ts frontend/src/utils/__tests__/sessionWriteGuard.test.ts
git commit -m "test: add readonly write-guard utility"
```

---

### Task 2: Add create_session fallback parsing + tests (TDD)

**Files:**
- Create: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/utils/sessionCreateFallback.ts`
- Test: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/utils/__tests__/sessionCreateFallback.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, it, expect } from "vitest";
import { resolveCreatedSessionId } from "../sessionCreateFallback";

describe("resolveCreatedSessionId", () => {
  it("prefers response session_id", () => {
    const id = resolveCreatedSessionId({
      parsed: { data: { session_id: "s1" } },
      sessions: [],
      expectedName: "Sheet1",
    });
    expect(id).toBe("s1");
  });

  it("falls back to matching session name", () => {
    const id = resolveCreatedSessionId({
      parsed: { data: {} },
      sessions: [
        { sessionId: "a1", name: "Sheet1" },
        { sessionId: "b2", name: "Sheet2" },
      ],
      expectedName: "Sheet1",
    });
    expect(id).toBe("a1");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/sessionCreateFallback.test.ts`  
Expected: FAIL (module not found / functions missing)

**Step 3: Write minimal implementation**

```typescript
type SessionItemLite = { sessionId: string; name: string };

export function resolveCreatedSessionId(input: {
  parsed: any;
  sessions: SessionItemLite[];
  expectedName: string;
}) {
  const direct =
    input.parsed?.data?.session?.session_id ||
    input.parsed?.data?.session_id ||
    "";
  if (direct) return String(direct);
  const matched = input.sessions.find((item) => item.name === input.expectedName);
  return matched?.sessionId || "";
}
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/sessionCreateFallback.test.ts`  
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/utils/sessionCreateFallback.ts frontend/src/utils/__tests__/sessionCreateFallback.test.ts
git commit -m "test: add create-session fallback parser"
```

---

### Task 3: Wire write guard into App/Toolbar/FormulaBar/GlideGrid (TDD)

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/App.tsx`
- Modify: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/components/layout/Toolbar.tsx`
- Modify: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/components/layout/FormulaBar.tsx`
- Modify: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/components/GlideGrid.tsx`
- Test: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/utils/__tests__/readonlyGuardHooks.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, it, expect, vi } from "vitest";
import { getWriteGuardState, READONLY_ALERT_MESSAGE } from "../sessionWriteGuard";

describe("readonly guard usage", () => {
  it("alerts when cannot write", () => {
    const state = getWriteGuardState({ sessionId: "", isReadOnly: true });
    expect(state.canWrite).toBe(false);
    expect(state.message).toBe(READONLY_ALERT_MESSAGE);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/readonlyGuardHooks.test.ts`  
Expected: FAIL (file missing)

**Step 3: Write minimal implementation**

- Add a `guardWrite()` helper in `App.tsx` using `getWriteGuardState`.
- In `handleSave`, `handleStyleChange`, formula bar commit handler, and grid edit入口 before write, call `guardWrite()` and `alert` if blocked.
- Add `disabled` or `readOnly` props to Toolbar/FormulaBar where applicable to prevent input.
- In `GlideGrid.tsx`, wrap write-related callbacks (`onCellEdited`, `onPaste`, `fillRange`, style updates) with `guardWrite` or pass a prop to block before operations.
- Add change-log comments with date for every modified block and keep comment ratio >= 60%.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/readonlyGuardHooks.test.ts`  
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/App.tsx frontend/src/components/layout/Toolbar.tsx frontend/src/components/layout/FormulaBar.tsx frontend/src/components/GlideGrid.tsx frontend/src/utils/__tests__/readonlyGuardHooks.test.ts
git commit -m "feat: block writes in readonly session"
```

---

### Task 4: Apply create_session fallback + omit empty from_session_id (TDD)

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/App.tsx`
- Test: `D:/Rust/metadata/.worktrees/readonly-write-guard/frontend/src/utils/__tests__/sessionCreateFallback.test.ts`

**Step 1: Write the failing test**

Extend `sessionCreateFallback.test.ts`:

```typescript
it("returns empty when no match", () => {
  const id = resolveCreatedSessionId({
    parsed: { data: {} },
    sessions: [{ sessionId: "a1", name: "Sheet2" }],
    expectedName: "Sheet1",
  });
  expect(id).toBe("");
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/sessionCreateFallback.test.ts`  
Expected: FAIL (if behavior missing)

**Step 3: Write minimal implementation**

- In `handleAddSheet`, only include `from_session_id` when `sessionId` is truthy.
- After create_session response, use `resolveCreatedSessionId`:
  - If empty, fetch sessions and attempt to match `session_name`.
  - If still empty, show error and return null.
- Add change-log comments with date near the modified blocks.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/sessionCreateFallback.test.ts`  
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/App.tsx
git commit -m "fix: improve create-session fallback"
```

---

### Task 5: Full verification

**Files:**
- (no code changes)

**Step 1: Run targeted tests**

Run: `npx vitest run src/utils/__tests__/sessionWriteGuard.test.ts src/utils/__tests__/sessionCreateFallback.test.ts`  
Expected: PASS

**Step 2: Run build**

Run: `npm run build`  
Expected: PASS

**Step 3: Commit (if any small fixes were made)**

```bash
git add -A
git commit -m "chore: stabilize readonly guard behavior"
```

