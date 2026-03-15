# Formula Column Dialog Samples Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在“公式列弹窗”中增加公式示例选择按钮与聚合函数不支持提示，并在用户输入聚合函数时给出明确错误提示。  

**Architecture:** 在 `GlideGrid.tsx` 的公式列弹窗区域新增本地状态与示例弹出层，示例按钮点击后填充算术表达式；通过 `formulaRange.js` 提供的聚合函数名单与检测函数，统一生成提示文案与输入校验。  

**Tech Stack:** React + TypeScript（前端）；现有工具函数 `formulaRange.js`；现有脚本测试 `scripts/formula_range.test.cjs`。  

---

### Task 1: 增加聚合函数名单与检测工具函数

**Files:**
- Modify: `d:/Rust/metadata/frontend/src/utils/formulaRange.js`
- Test: `d:/Rust/metadata/frontend/scripts/formula_range.test.cjs`

**Step 1: Write the failing test**

```javascript
// scripts/formula_range.test.cjs
// 验证聚合函数名单与检测函数
assert.deepStrictEqual(getAggregateFunctionNames(), [
  "SUM",
  "COUNT",
  "COUNTA",
  "AVG",
  "AVERAGE",
  "MAX",
  "MIN"
]);
assert.strictEqual(isAggregateFormulaFunction("SUM"), true);
assert.strictEqual(isAggregateFormulaFunction("sum"), true);
assert.strictEqual(isAggregateFormulaFunction("IF"), false);
```

**Step 2: Run test to verify it fails**

Run: `node scripts/formula_range.test.cjs`  
Expected: FAIL with "getAggregateFunctionNames is not a function" or similar.

**Step 3: Write minimal implementation**

```javascript
// formulaRange.js
export function getAggregateFunctionNames() {
  return ["SUM", "COUNT", "COUNTA", "AVG", "AVERAGE", "MAX", "MIN"];
}

export function isAggregateFormulaFunction(rawFunc) {
  const name = String(rawFunc ?? "").toUpperCase();
  return getAggregateFunctionNames().includes(name);
}
```

**Step 4: Run test to verify it passes**

Run: `node scripts/formula_range.test.cjs`  
Expected: PASS with "formula_range.test.cjs passed".

**Step 5: Commit**

Skip commit unless the user explicitly asks to commit.

---

### Task 2: 公式列弹窗增加“选择公式”与聚合函数提示

**Files:**
- Modify: `d:/Rust/metadata/frontend/src/components/GlideGrid.tsx`

**Step 1: Write the failing test**

说明：该 UI 变更目前无现成测试框架覆盖。这里新增手动验证步骤作为验收门槛。

**Step 2: Run test to verify it fails**

手动验证：打开“插入公式列”弹窗，未出现“选择公式”按钮与聚合提示，即失败。

**Step 3: Write minimal implementation**

```tsx
// GlideGrid.tsx
// 1) 新增示例列表与弹出层状态
// 2) 在公式输入框旁增加“选择公式”按钮，点击后展示示例列表
// 3) 点击示例后填充公式输入框
// 4) 输入中若检测到聚合函数，则提示“不支持 SUM/COUNT/COUNTA/AVG/AVERAGE/MAX/MIN”
// 5) 输入框下方展示一行小字提示
```

**Step 4: Run test to verify it passes**

手动验证：
- 点击“选择公式”，列表出现并可插入 `A+B` 等示例  
- 手动输入 `=SUM(A:B)` 时出现“不支持聚合函数”提示  
- 其他算术表达式仍可提交  

**Step 5: Commit**

Skip commit unless the user explicitly asks to commit.

---

### Task 3: 运行现有脚本与回归检查

**Files:**
- None

**Step 1: Run test**

Run: `node scripts/formula_range.test.cjs`  
Expected: PASS

**Step 2: Manual smoke check**

验证弹窗提示与示例选择功能，确保不影响其他输入。

**Step 3: Commit**

Skip commit unless the user explicitly asks to commit.
