# Vitest Suite Exclusion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 `npx vitest run` 仅运行真正的 Vitest 单测，避免脚本型文件触发“无 suite”错误。

**Architecture:** 在 `vite.config.ts` 中配置 vitest `include/exclude` 规则，保持默认测试框架不变，仅调整扫描范围。

**Tech Stack:** Vite, Vitest, TypeScript

---

### Task 1: 约束 Vitest 扫描范围

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\vite.config.ts`

**Step 1: 复现失败（TDD - RED）**

Run: `npx vitest run`

Expected: FAIL with `No test suite found in file ...`

**Step 2: 写最小配置（TDD - GREEN）**

Add to `vite.config.ts`:

```ts
  // ### Change Log
  // - 2026-03-15: Reason=vitest should ignore script tests; Purpose=avoid "No test suite" failures
  test: {
    include: ["src/**/*.test.ts", "src/**/*.spec.ts"],
    exclude: ["tests/**", "scripts/**", "**/*.test.cjs", "**/*.spec.cjs"]
  }
```

**Step 3: 验证通过（TDD - GREEN）**

Run: `npx vitest run`

Expected: PASS (no “No test suite found”).

**Step 4: 轻量回归**

Run: `npm run build`

Expected: PASS (允许 Rollup 体积警告)。

**Step 5: Commit（可选）**

```bash
git add D:\Rust\metadata\.worktrees\formula-tips\frontend\vite.config.ts
```

