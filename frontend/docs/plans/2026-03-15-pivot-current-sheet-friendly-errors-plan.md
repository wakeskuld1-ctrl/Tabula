# Pivot Current-Sheet Persistence + Friendly Errors Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** current-sheet 输出落库（从选中单元格写入）并提供友好中文失败提示。

**Architecture:** 新增纯函数生成带偏移的更新列表与错误文案格式化；App 中复用既有 pivot 写入流程。

**Tech Stack:** React 18 + TypeScript + vitest

---

### Task 1: 更新列表偏移与错误文案（TDD）

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\pivotSession.ts`
- Test: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\pivotSession.test.ts`

**Step 1: Write the failing test**

```ts
import { buildPivotUpdatesWithOffset, formatPivotPersistError } from "../pivotSession";

const updates = buildPivotUpdatesWithOffset({
  headers: ["A"],
  data: [[1]],
  columnNames: ["col_a"],
  rowOffset: 2,
  colOffset: 3
});
expect(updates[0].row).toBe(2);
expect(updates[0].col).toBe("col_a");

expect(formatPivotPersistError({ step: "ensure_columns", status: 405 })).toContain("扩列失败");
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/pivotSession.test.ts`
Expected: FAIL (function not found)

**Step 3: Write minimal implementation**

- 新增 `buildPivotUpdatesWithOffset`
- 新增 `formatPivotPersistError`

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/pivotSession.test.ts`
Expected: PASS

---

### Task 2: App current-sheet 落库 + 友好提示

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`

**Step 1: Write the failing test**

N/A (helper tests cover core logic)

**Step 2: Implement**

- current-sheet：从 `selectedPosition` 获取 offset
- 未选中时回退 A1 并 `setDebugInfo`
- 只读会话直接提示并返回
- 调用 `formatPivotPersistError` 统一报错文案

**Step 3: Run tests**

`npx vitest run src/utils/__tests__/pivotSession.test.ts`

---

### Task 3: 任务日志

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\.trae\CHANGELOG_TASK.md`

**Step 1: 追加变更记录**

---

Plan complete and saved to `docs/plans/2026-03-15-pivot-current-sheet-friendly-errors-plan.md`.

Two execution options:
1. Subagent-Driven (this session)
2. Parallel Session (separate)

Which approach?
