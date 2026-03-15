# 相对引用位移与填充序列策略 Implementation Plan
  
 - **[2026-02-17]** 变更原因：用户要求输出实施计划; 变更目的：为后续开发提供可执行步骤
  
> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
  
**Goal:** 为下拉填充补齐相对引用位移与智能序列推断逻辑，并接入批量更新通道  
  
**Architecture:** 在前端 utils 层实现 A1 公式位移与序列推断纯函数，填充逻辑复用批量更新入口。  
  
**Tech Stack:** React 18、TypeScript/JavaScript、@glideapps/glide-data-grid、HyperFormula  
  
---
  
### Task 1: 新增公式相对引用位移工具
  
**Files:**  
- Create: `d:\Rust\metadata\frontend\src\utils\formulaFill.js`  
- Modify: `d:\Rust\metadata\frontend\src\utils\formulaRange.js`  
- Test: `d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs`  
  
**Step 1: Write the failing test**  
  
```javascript
const { shiftFormulaReferences } = require("../src/utils/formulaFill");

test("shiftFormulaReferences moves relative refs only", () => {
  expect(shiftFormulaReferences("=A1+$B$2+$C3+D$4", 1, 2))
    .toBe("=B3+$B$2+$C5+E$4");
});
```
  
**Step 2: Run test to verify it fails**  
  
Run: `node d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs`  
Expected: FAIL with "shiftFormulaReferences is not a function"  
  
**Step 3: Write minimal implementation**  
  
```javascript
export function shiftFormulaReferences(formula, dx, dy) {
  // 解析 A1/$A$1，并对相对列/行做位移
  // 超界回退为原引用
}
```
  
**Step 4: Run test to verify it passes**  
  
Run: `node d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs`  
Expected: PASS  
  
**Step 5: Commit**  
  
```bash
git add d:\Rust\metadata\frontend\src\utils\formulaFill.js d:\Rust\metadata\frontend\src\utils\formulaRange.js d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs
git commit -m "feat: add formula reference shifting utilities"
```
  
---
  
### Task 2: 新增智能序列推断工具
  
**Files:**  
- Modify: `d:\Rust\metadata\frontend\src\utils\formulaFill.js`  
- Test: `d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs`  
  
**Step 1: Write the failing test**  
  
```javascript
const { inferFillValues } = require("../src/utils/formulaFill");

test("inferFillValues handles numeric sequences", () => {
  const result = inferFillValues(["1", "3"], 4);
  expect(result).toEqual(["1", "3", "5", "7"]);
});
```
  
**Step 2: Run test to verify it fails**  
  
Run: `node d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs`  
Expected: FAIL with "inferFillValues is not a function"  
  
**Step 3: Write minimal implementation**  
  
```javascript
export function inferFillValues(sourceValues, targetLength) {
  // 识别数值/日期/文本+数字序列，无法识别则复制
}
```
  
**Step 4: Run test to verify it passes**  
  
Run: `node d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs`  
Expected: PASS  
  
**Step 5: Commit**  
  
```bash
git add d:\Rust\metadata\frontend\src\utils\formulaFill.js d:\Rust\metadata\frontend\src\scripts\formula_range.test.cjs
git commit -m "feat: add fill sequence inference"
```
  
---
  
### Task 3: 接入批量更新通道
  
**Files:**  
- Modify: `d:\Rust\metadata\frontend\src\components\GlideGrid.tsx`  
- Test: `d:\Rust\metadata\frontend\src\scripts\verify_state_integration.cjs`  
  
**Step 1: Write the failing test**  
  
```javascript
// 在 verify_state_integration.cjs 中新增：模拟填充范围并断言 batch_update_cells 被调用
```
  
**Step 2: Run test to verify it fails**  
  
Run: `node d:\Rust\metadata\frontend\src\scripts\verify_state_integration.cjs`  
Expected: FAIL with "fill operation not invoked"  
  
**Step 3: Write minimal implementation**  
  
```typescript
// 在 GlideGrid 内部新增 fill 入口，生成目标更新集合并复用 batch_update_cells
```
  
**Step 4: Run test to verify it passes**  
  
Run: `node d:\Rust\metadata\frontend\src\scripts\verify_state_integration.cjs`  
Expected: PASS  
  
**Step 5: Commit**  
  
```bash
git add d:\Rust\metadata\frontend\src\components\GlideGrid.tsx d:\Rust\metadata\frontend\src\scripts\verify_state_integration.cjs
git commit -m "feat: integrate fill logic with batch update"
```
  
