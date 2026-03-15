# Formula Tips Focus Gate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 只有在输入框聚焦并输入公式或点击 fx 时显示公式提示，选中含公式单元格不显示。

**Architecture:** 通过扩展 `shouldShowFormulaHelp` 的条件参数与 `FormulaBar` 的聚焦状态实现，保持 UI 渲染逻辑集中。

**Tech Stack:** React 18 + TypeScript + vitest

---

### Task 1: 更新公式提示触发逻辑（TDD）

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\formulaHelp.ts`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaHelp.test.ts`

**Step 1: Write the failing test**

```ts
// shouldShowFormulaHelp
expect(shouldShowFormulaHelp({ text: "=SUM", isFxToggled: false, isFocused: false })).toBe(false);
expect(shouldShowFormulaHelp({ text: "=SUM", isFxToggled: false, isFocused: true })).toBe(true);
expect(shouldShowFormulaHelp({ text: "A1", isFxToggled: true, isFocused: false })).toBe(true);
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/formulaHelp.test.ts`
Expected: FAIL because `shouldShowFormulaHelp` ignores `isFocused`.

**Step 3: Write minimal implementation**

```ts
export function shouldShowFormulaHelp(params: { text: string; isFxToggled: boolean; isFocused: boolean }) {
  const trimmed = (params.text || "").trimStart();
  return params.isFxToggled || (params.isFocused && trimmed.startsWith("="));
}
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/formulaHelp.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/formulaHelp.ts src/utils/__tests__/formulaHelp.test.ts
git commit -m "fix: gate formula tips by focus"
```

---

### Task 2: FormulaBar 接入聚焦状态

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\layout\FormulaBar.tsx`

**Step 1: Write the failing test**

N/A (covered by helper tests, UI smoke test suggested)

**Step 2: Run test to verify it fails**

N/A

**Step 3: Write minimal implementation**

- 增加 `isFocused` state。
- input 的 `onFocus/onBlur` 控制 `isFocused`。
- `shouldShowFormulaHelp` 传入 `isFocused`。

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/formulaHelp.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/components/layout/FormulaBar.tsx
git commit -m "feat: show formula tips only while editing"
```

---

### Task 3: 任务日志

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\.trae\CHANGELOG_TASK.md`

**Step 1: 追加变更记录**

补充本次“公式提示聚焦门禁”的修改原因、潜在问题与待验证点。

**Step 2: Commit**

```bash
git add .trae/CHANGELOG_TASK.md
git commit -m "docs: log formula tips focus gate"
```

---

Plan complete and saved to `docs/plans/2026-03-15-formula-tip-focus-plan.md`.

Two execution options:
1. Subagent-Driven (this session)
2. Parallel Session (separate)

Which approach?
