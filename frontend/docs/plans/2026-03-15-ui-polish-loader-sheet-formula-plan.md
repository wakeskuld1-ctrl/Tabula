# UI Polish (Loader Auto-Hide + Sheet Add Position + Formula Persistence) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 Loader 完成提示自动隐藏、Sheet “+” 贴近标签、公式保存原始表达式但显示计算结果。

**Architecture:** 通过一组可测试的纯函数辅助实现：debug overlay 自动隐藏判断、Sheet tab 列表构建、公式保存/显示决策。App/SheetBar/GlideGrid 只做“调用 + UI 绑定”。

**Tech Stack:** React 18 + TypeScript + Vite + vitest（已有测试文件使用）。

---

### Task 1: Debug Overlay 自动隐藏逻辑（纯函数）

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\debugOverlay.ts`
- Test: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\debugOverlay.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { shouldAutoHideDebugInfo } from "../debugOverlay";

describe("debug overlay", () => {
  it("auto-hides only for loaded rows when not loading", () => {
    expect(shouldAutoHideDebugInfo("Loaded orders: 100 rows", false)).toBe(true);
    expect(shouldAutoHideDebugInfo("Loaded orders: 100 rows", true)).toBe(false);
    expect(shouldAutoHideDebugInfo("Fetch failed: 500", false)).toBe(false);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/debugOverlay.test.ts`
Expected: FAIL with “shouldAutoHideDebugInfo is not defined” or module not found.

**Step 3: Write minimal implementation**

```ts
export const shouldAutoHideDebugInfo = (message: string, loading: boolean): boolean => {
  if (loading) return false;
  return /^Loaded\s.+:\s\d+\srows$/i.test(message.trim());
};
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/debugOverlay.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/debugOverlay.ts src/utils/__tests__/debugOverlay.test.ts
git commit -m "test: add debug overlay auto-hide helper"
```

---

### Task 2: Sheet Tab 列表构建（“+”贴近 tab 的纯函数）

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\sheetTabsModel.ts`
- Test: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\sheetTabsModel.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { buildSheetTabItems } from "../sheetTabsModel";

describe("sheet tabs model", () => {
  it("appends add button as last item", () => {
    const items = buildSheetTabItems([
      { sessionId: "s1", displayName: "Sheet1", isDefault: false }
    ]);
    expect(items[0].type).toBe("tab");
    expect(items[1].type).toBe("add");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/sheetTabsModel.test.ts`
Expected: FAIL with module not found.

**Step 3: Write minimal implementation**

```ts
export type SheetTabItem = { type: "tab"; sessionId: string; displayName: string; isDefault: boolean };
export type SheetAddItem = { type: "add" };

export const buildSheetTabItems = (sessions: SheetTabItem[]): Array<SheetTabItem | SheetAddItem> => {
  return [...sessions.map(s => ({ ...s, type: "tab" as const })), { type: "add" as const }];
};
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/sheetTabsModel.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/sheetTabsModel.ts src/utils/__tests__/sheetTabsModel.test.ts
git commit -m "test: add sheet tabs model helper"
```

---

### Task 3: 公式保存/显示决策（纯函数）

**Files:**
- Create: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\formulaPersistence.ts`
- Test: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\formulaPersistence.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { resolveFormulaStorage } from "../formulaPersistence";

describe("formula persistence", () => {
  it("keeps raw formula while exposing display override", () => {
    const result = resolveFormulaStorage("=SUM(A:A)", "123");
    expect(result.storedValue).toBe("=SUM(A:A)");
    expect(result.displayValue).toBe("123");
  });

  it("keeps normal input as stored and display", () => {
    const result = resolveFormulaStorage("42", "");
    expect(result.storedValue).toBe("42");
    expect(result.displayValue).toBe("42");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/__tests__/formulaPersistence.test.ts`
Expected: FAIL with module not found.

**Step 3: Write minimal implementation**

```ts
export const resolveFormulaStorage = (rawInput: string, computedDisplay?: string) => {
  const isFormula = typeof rawInput === "string" && rawInput.trim().startsWith("=");
  if (!isFormula) {
    return { storedValue: rawInput, displayValue: rawInput, isFormula };
  }
  return {
    storedValue: rawInput.trim(),
    displayValue: computedDisplay ?? rawInput.trim(),
    isFormula
  };
};
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/formulaPersistence.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/utils/formulaPersistence.ts src/utils/__tests__/formulaPersistence.test.ts
git commit -m "test: add formula persistence helper"
```

---

### Task 4: App.tsx 接入 Loader 自动隐藏

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`

**Step 1: Write the failing test**

Add a helper test if needed (optional). Since this is a hook-level change, rely on Task 1 helper and manual UI check.

**Step 2: Run test to verify it fails**

No new test in this task.

**Step 3: Write minimal implementation**

- 引入 `shouldAutoHideDebugInfo`。
- 新增 `useEffect`：当 debugInfo 命中“Loaded ... rows”且 loading=false 时启动 10 秒计时，超时清空 debugInfo。
- 新提示出现时取消旧计时器。

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/debugOverlay.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/App.tsx
git commit -m "feat: auto-hide loaded debug overlay"
```

---

### Task 5: SheetBar 渲染“+”贴近标签

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\layout\SheetBar.tsx`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css` (如需微调间距)

**Step 1: Write the failing test**

Run: `npx vitest run src/utils/__tests__/sheetTabsModel.test.ts`
Expected: PASS (helper already covered)

**Step 2: Run test to verify it fails**

N/A

**Step 3: Write minimal implementation**

- 使用 `buildSheetTabItems` 生成渲染列表。
- 遇到 `{ type: "add" }` 渲染“+”按钮。
- 按需调整 `.sheet-tabs` / `.sheet-add` 的 margin/padding。

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/sheetTabsModel.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/components/layout/SheetBar.tsx src/App.css
git commit -m "feat: move sheet add button next to tabs"
```

---

### Task 6: GlideGrid 公式持久化与显示覆盖

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\GlideGrid.tsx`

**Step 1: Write the failing test**

Run: `npx vitest run src/utils/__tests__/formulaPersistence.test.ts`
Expected: PASS (helper exists)

**Step 2: Run test to verify it fails**

N/A

**Step 3: Write minimal implementation**

- 使用 `resolveFormulaStorage` 将原始公式写入缓存与后端。
- 对聚合/lookup 拦截计算结果写入“显示覆盖 map”。
- `getCellContent` 渲染时优先显示覆盖值，但公式栏仍读取 raw。

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/__tests__/formulaPersistence.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/components/GlideGrid.tsx
git commit -m "fix: keep formula raw value while showing computed"
```

---

### Task 7: 变更记录与说明

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\.trae\CHANGELOG_TASK.md`

**Step 1: 追加任务记录**

补充本次 UI polish 变更摘要、风险点、待验证项。

**Step 2: Commit**

```bash
git add .trae/CHANGELOG_TASK.md
git commit -m "docs: log ui polish changes"
```

---

## 执行方式选择
Plan complete and saved to `docs/plans/2026-03-15-ui-polish-loader-sheet-formula-plan.md`.

Two execution options:
1. Subagent-Driven (this session)
2. Parallel Session (separate)

Which approach?
