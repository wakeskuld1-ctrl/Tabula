# Formula Failure Retest Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 补齐 215 个失败公式的参数样例，重新测试并生成 MD 报告。

**Architecture:** 扩展现有 `scripts/test_hf.cjs`，为失败函数添加明确的参数模板与数据区准备逻辑，运行后输出结构化统计并生成 `docs/FORMULA_TEST_REPORT.md`。全流程以脚本驱动并保持单入口。

**Tech Stack:** Node.js, HyperFormula, 现有测试脚本（CommonJS）

---

### Task 1: 扩展失败函数参数模板

**Files:**
- Modify: `d:\Rust\metadata\frontend\scripts\test_hf.cjs`

**Step 1: Write the failing test**

```bash
node scripts/test_hf.cjs
```

Expected: 报告中仍存在大量 “Wrong number of arguments” 失败项。

**Step 2: Implement minimal parameter templates**

```js
// 在 test_hf.cjs 中新增失败函数参数模板表
// 覆盖统计/分布/日期/金融/文本/查找/数组/工程等函数
```

**Step 3: Run test to verify it improves**

```bash
node scripts/test_hf.cjs
```

Expected: 失败数量显著下降，错误以语义类错误为主而非参数数量错误。

**Step 4: Commit**

```bash
git add scripts/test_hf.cjs
git commit -m "test: expand HyperFormula failure samples"
```

> 注意：当前约束不允许自动提交，如需提交需明确授权。

---

### Task 2: 生成失败函数补充测试结果与统计

**Files:**
- Modify: `d:\Rust\metadata\frontend\scripts\test_hf.cjs`

**Step 1: Write the failing test**

```bash
node scripts/test_hf.cjs
```

Expected: 输出仅为控制台日志，缺少结构化 JSON/MD 数据。

**Step 2: Implement report output**

```js
// 在 test_hf.cjs 中新增结构化结果汇总
// 输出到 docs/FORMULA_TEST_REPORT.md
```

**Step 3: Run test to verify it passes**

```bash
node scripts/test_hf.cjs
```

Expected: 生成 docs/FORMULA_TEST_REPORT.md 并包含失败样例与统计。

**Step 4: Commit**

```bash
git add scripts/test_hf.cjs docs/FORMULA_TEST_REPORT.md
git commit -m "test: add formula failure report output"
```

> 注意：当前约束不允许自动提交，如需提交需明确授权。

---

### Task 3: 回归验证与构建

**Files:**
- Test: `d:\Rust\metadata\frontend\package.json`

**Step 1: Run build as typecheck**

```bash
npm run build
```

Expected: 构建成功，产物生成，无 TypeScript 错误。

**Step 2: Commit**

```bash
git add .
git commit -m "test: update formula test report"
```

> 注意：当前约束不允许自动提交，如需提交需明确授权。
