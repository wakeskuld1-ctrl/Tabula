# Formula Aliases & Purpose Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为全量公式表格补充业务化参数别名与用途说明，并接入 CI 校验 README 一致性。

**Architecture:** 在生成器中引入别名/用途规则，输出双语列；支持 `--check` 模式并在 CI 中调用。

**Tech Stack:** Node.js (CJS), HyperFormula, Markdown, GitHub Actions

---

### Task 1: 为新列写失败测试

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`

**Step 1: 写失败测试**

```js
// 期望新增列：参数说明 / Parameter Notes、用途 / Purpose
```

**Step 2: 运行测试确认失败**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: FAIL（缺少新列/新输出）

**Step 3: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs
git commit -m "test: require parameter notes and purpose columns"
```

---

### Task 2: 实现参数别名 + 用途描述生成

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/formula_alias_map.cjs`

**Step 1: 实现别名映射与用途规则**
- typeAliases（通用）
- functionAliases（常见函数覆盖）
- functionPurposeOverrides（常见用途）
- purposeRules（按命名规则归类）

**Step 2: 更新表格列**
- 新增“参数说明 / Parameter Notes”
- 新增“用途 / Purpose”

**Step 3: 运行测试确认通过**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: PASS

**Step 4: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/formula_alias_map.cjs
git commit -m "feat: add parameter aliases and purpose descriptions"
```

---

### Task 3: 增加 README 检查模式与 CI

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/package.json`
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/.github/workflows/formula-docs.yml`

**Step 1: 实现 --check 模式**
- 仅比对内容，不写文件
- 若不一致则退出码 1

**Step 2: 新增 npm scripts**
- `docs:formulas`
- `docs:formulas:check`

**Step 3: 添加 GitHub Actions**
- 拉取代码
- 安装前端依赖
- 执行 `npm run docs:formulas:check`

**Step 4: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/package.json D:/Rust/metadata/.worktrees/formula-docs-plan/.github/workflows/formula-docs.yml
git commit -m "ci: verify formula docs consistency"
```

---

### Task 4: 生成 README 与最终验证

**Step 1: 运行生成脚本**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`

**Step 2: 运行测试**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`

**Step 3: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/README.md D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/README.md
git commit -m "docs: add parameter notes and purpose columns"
```
