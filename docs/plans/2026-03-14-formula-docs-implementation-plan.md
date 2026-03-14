# Formula Docs Full List Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在 README 内提供 HyperFormula 注册的全量函数用法（语法 + 示例），并放入 `<details>` 折叠块中。

**Architecture:** 使用 Node 脚本从 HyperFormula 读取注册函数名与（可用时）参数元数据，生成表格并注入 README 指定标记区。若元数据缺失，语法与示例降级为占位形式，但保证每个函数都有用法条目。

**Tech Stack:** Node.js (CJS 脚本), HyperFormula, Markdown (README)

---

### Task 1: 建立 README 注入标记与最小失败测试

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/README.md`
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`

**Step 1: 写失败测试（缺少生成器应失败）**

```js
// D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs
const assert = require("assert");

// ### 变更记录
// - 2026-03-14: 原因=覆盖公式全量文档生成; 目的=确保 README 注入格式稳定
// - 2026-03-14: 原因=先写失败测试; 目的=遵循 TDD 流程

let generator;
try {
  generator = require("../generate_formula_docs.cjs");
} catch (e) {
  generator = null;
}

assert.ok(generator, "generator module should exist");
```

**Step 2: 运行测试确认失败**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: FAIL with "generator module should exist"

**Step 3: README 插入标记区（先放占位）**

在 README 中加入：

```
<!-- FORMULA_DOCS_START -->
<!-- FORMULA_DOCS_END -->
```

**Step 4: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/README.md D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs
git commit -m "test: add failing test and README markers for formula docs"
```

---

### Task 2: 实现公式文档生成器（最小可用）

**Files:**
- Create: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`

**Step 1: 最小实现（让测试能加载模块）**

```js
// D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs
// ### 变更记录
// - 2026-03-14: 原因=新增公式全量文档生成器; 目的=统一 README 自动化输出入口
// - 2026-03-14: 原因=便于后续扩展元数据读取; 目的=拆分为可复用函数

module.exports = {
  buildFormulaDocsSection() {
    return "<!-- FORMULA_DOCS_START -->\n<!-- FORMULA_DOCS_END -->\n";
  }
};
```

**Step 2: 运行测试确认通过**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: PASS

**Step 3: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs
git commit -m "feat: add minimal formula docs generator"
```

---

### Task 3: 读取 HyperFormula 注册函数并生成表格

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`

**Step 1: 失败测试（表格结构与行数）**

```js
// D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs
const { buildFormulaDocsSection, getRegisteredFunctions } = require("../generate_formula_docs.cjs");

const fnList = getRegisteredFunctions();
assert.ok(Array.isArray(fnList) && fnList.length > 0, "function list should not be empty");

const section = buildFormulaDocsSection();
assert.ok(section.includes("| 函数名 | 语法 | 示例 | 备注 |"), "table header should exist");
assert.ok(section.split("\n").length > fnList.length, "section should include rows");
```

**Step 2: 实现读取逻辑（含兜底）**

```js
// 伪代码示意：实际需根据 HyperFormula API 调整
const { HyperFormula } = require("hyperformula");

function getRegisteredFunctions() {
  return HyperFormula.getRegisteredFunctionNames();
}
```

**Step 3: 语法/示例生成（带兜底）**
- 如果能读到参数元数据，生成 `FUNC(text, number, [optional])`
- 否则生成 `FUNC(...)`
- 示例用 `=FUNC("text", 1, TRUE)` 等模板

**Step 4: 运行测试**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: PASS

**Step 5: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs
git commit -m "feat: generate full formula table from HyperFormula"
```

---

### Task 4: 注入 README 并完成输出

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`
- Modify: `D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/README.md`

**Step 1: 失败测试（README 注入）**

在测试里读取 README，断言标记区被替换为表格内容。

**Step 2: 实现注入逻辑**
- 读取 README
- 替换 `<!-- FORMULA_DOCS_START -->` 与 `<!-- FORMULA_DOCS_END -->` 之间的内容
- 写回 README

**Step 3: 运行测试**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/tests/generate_formula_docs.test.cjs`  
Expected: PASS

**Step 4: 提交**

```bash
git add D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/README.md D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs
git commit -m "docs: inject full formula docs into README"
```

---

### Task 5: 总体验证

**Step 1: 运行生成脚本**

Run: `node D:/Rust/metadata/.worktrees/formula-docs-plan/frontend/scripts/generate_formula_docs.cjs`  
Expected: README 中 `<details>` 块含全量函数表格

**Step 2: 目视检查**
- `<details>` 可展开  
- 每个函数有语法与示例（至少占位）  
- 表格行数与函数列表一致

**Step 3: 提交**

```bash
git status --short
```

