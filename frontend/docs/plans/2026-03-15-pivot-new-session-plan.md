# Pivot New Session Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 Pivot 结果写入新 Session 并自动切换到新 Sheet。

**Architecture:** 新增纯函数构建写入更新列表与分批逻辑；App 层调用 create_session + batch_update_cells 完成持久化。

**Tech Stack:** React 18 + TypeScript + vitest

---

### Task 1: Pivot 写入更新列表构建（TDD）

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\pivotSession.ts`
- Test: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\pivotSession.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { buildPivotUpdates, chunkPivotUpdates } from "../pivotSession";

describe("pivot session updates", () => {
  it("builds header row + data rows", () => {
    const result = buildPivotUpdates({
      headers: ["A", "B"],
      data: [[1, 2], [3, 4]],
      columnNames: ["col_a", "col_b"]
    });
    expect(result[0]).toEqual({ row: 0, col: "col_a", val: "A" });
    expect(result[1]).toEqual({ row: 0, col: "col_b", val: "B" });
    expect(result[2]).toEqual({ row: 1, col: "col_a", val: "1" });
  });

  it("chunks updates by size", () => {
    const updates = Array.from({ length: 5 }).map((_, i) => ({ row: i, col: "c", val: String(i) }));
    const chunks = chunkPivotUpdates(updates, 2);
    expect(chunks.length).toBe(3);
    expect(chunks[0].length).toBe(2);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/pivotSession.test.ts`
Expected: FAIL (module not found).

**Step 3: Write minimal implementation**

```ts
export const buildPivotUpdates = (...) => { ... };
export const chunkPivotUpdates = (...) => { ... };
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/pivotSession.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/pivotSession.ts src/utils/__tests__/pivotSession.test.ts
git commit -m "test: add pivot session update helpers"
```

---

### Task 2: App 持久化 Pivot 到新 Session

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`

**Step 1: Write the failing test**

N/A (helper tests cover core logic)

**Step 2: Run test to verify it fails**

N/A

**Step 3: Write minimal implementation**

- `handlePivotApply('new-sheet')`:
  - 调 `handleAddSheet()` 创建 session
  - 使用 `buildPivotUpdates` 构造 updates
  - 使用 `chunkPivotUpdates` 分批写入 `batch_update_cells`
  - 刷新 session 并切换到新 session
- 失败时显示 debugInfo / alert

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/pivotSession.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/App.tsx
git commit -m "feat: persist pivot to new session"
```

---

### Task 3: 任务日志

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\.trae\CHANGELOG_TASK.md`

**Step 1: 追加变更记录**

补充本次 Pivot 新 session 持久化的修改原因、潜在问题与待验证点。

**Step 2: Commit**

```bash
git add .trae/CHANGELOG_TASK.md
git commit -m "docs: log pivot new session persistence"
```

---

Plan complete and saved to `docs/plans/2026-03-15-pivot-new-session-plan.md`.

Two execution options:
1. Subagent-Driven (this session)
2. Parallel Session (separate)

Which approach?
