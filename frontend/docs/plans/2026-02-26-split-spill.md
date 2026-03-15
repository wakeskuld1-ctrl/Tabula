# SPLIT Spill Implementation Plan
 
> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
 
**Goal:** 在不写回后端数据的前提下，让 SPLIT 结果横向溢出到相邻单元格显示。
 
**Architecture:** 在 FormulaEngine 内维护溢出映射表，calculate 在锚点单元格计算时记录溢出范围；GlideGrid 在渲染空单元格时查询溢出映射获取显示值。
 
**Tech Stack:** TypeScript, HyperFormula, React, GlideGrid, Puppeteer
 
---
 
### Task 1: 记录 SPLIT 溢出映射
 
**Files:**
- Modify: `d:\Rust\metadata\frontend\src\utils\FormulaEngine.ts`
- Test: `d:\Rust\metadata\frontend\scripts\smoke_test.js`
 
**Step 1: Write the failing test**
 
```js
// 追加在 smoke_test.js 网格准备完成后
const spillResult = await page.evaluate(() => {
  const engine = window.FormulaEngine?.getInstance?.();
  if (!engine) return { ok: false, error: 'FormulaEngine missing' };
  engine.setCellValue(0, 0, '=SPLIT("A,B", ",")', 'Sheet1');
  const anchor = engine.calculate('=SPLIT("A,B", ",")', 0, 0, 'Sheet1');
  const spill = engine.getSpillValue?.(1, 0, 'Sheet1');
  return { ok: true, anchor, spill };
});
if (!spillResult.ok || spillResult.anchor !== 'A' || spillResult.spill !== 'B') {
  console.error('❌ SPLIT spill test failed', spillResult);
  process.exit(1);
}
console.log('✅ SPLIT spill test passed.');
```
 
**Step 2: Run test to verify it fails**
 
Run: `node scripts/smoke_test.js`  
Expected: FAIL with "SPLIT spill test failed" or `getSpillValue` undefined.
 
**Step 3: Write minimal implementation**
 
```ts
// FormulaEngine.ts 增加溢出缓存结构与 API
private spillMap = new Map<string, Map<string, string>>();
public getSpillValue(col: number, row: number, sheetName: string): string | undefined { ... }
private recordSpill(col: number, row: number, sheetName: string, data: any[][]) { ... }
```
 
**Step 4: Run test to verify it passes**
 
Run: `node scripts/smoke_test.js`  
Expected: PASS with "SPLIT spill test passed."
 
**Step 5: Commit**
 
```bash
git add src/utils/FormulaEngine.ts scripts/smoke_test.js
git commit -m "feat: add split spill mapping for FormulaEngine"
```
 
---
 
### Task 2: 在网格渲染时读取溢出值
 
**Files:**
- Modify: `d:\Rust\metadata\frontend\src\components\GlideGrid.tsx:1740-1755`
- Test: `d:\Rust\metadata\frontend\scripts\smoke_test.js`
 
**Step 1: Write the failing test**
 
```js
// 在 smoke_test.js 中增加断言（依赖 Task 1 的 API）
const gridSpill = await page.evaluate(() => {
  const engine = window.FormulaEngine?.getInstance?.();
  if (!engine) return { ok: false };
  engine.setCellValue(0, 1, '=SPLIT("X,Y", ",")', 'Sheet1');
  const anchor = engine.calculate('=SPLIT("X,Y", ",")', 0, 1, 'Sheet1');
  const spill = engine.getSpillValue?.(1, 1, 'Sheet1');
  return { ok: true, anchor, spill };
});
if (!gridSpill.ok || gridSpill.anchor !== 'X' || gridSpill.spill !== 'Y') {
  console.error('❌ Grid spill check failed', gridSpill);
  process.exit(1);
}
```
 
**Step 2: Run test to verify it fails**
 
Run: `node scripts/smoke_test.js`  
Expected: FAIL with "Grid spill check failed"
 
**Step 3: Write minimal implementation**
 
```ts
// GlideGrid.tsx 中，在 displayStr 计算后追加：
if (!rawValue && !displayStr) {
  const spill = formulaEngine.current.getSpillValue(col, row, tableName);
  if (spill !== undefined) {
    displayStr = String(spill);
  }
}
```
 
**Step 4: Run test to verify it passes**
 
Run: `node scripts/smoke_test.js`  
Expected: PASS
 
**Step 5: Commit**
 
```bash
git add src/components/GlideGrid.tsx scripts/smoke_test.js
git commit -m "feat: render spilled split results in grid"
```
 
---
 
### Task 3: 回归验证与报告输出
 
**Files:**
- Modify: `d:\Rust\metadata\frontend\scripts\test_hf.cjs`
- Test: `d:\Rust\metadata\frontend\scripts\test_hf.cjs`
 
**Step 1: Write the failing test**
 
```js
// test_hf.cjs 中补充 SPLIT 输出样例断言
const splitValue = hf.calculateFormula('SPLIT("A,B", ",")', sheetId);
if (!splitValue || !splitValue.data) {
  throw new Error('SPLIT result not array');
}
```
 
**Step 2: Run test to verify it fails**
 
Run: `node scripts/test_hf.cjs`  
Expected: FAIL before修复
 
**Step 3: Write minimal implementation**
 
```js
// 将断言移动到 SPLIT 通过后或用现有 formatResultValue 处理
```
 
**Step 4: Run test to verify it passes**
 
Run: `node scripts/test_hf.cjs`  
Expected: PASS
 
**Step 5: Commit**
 
```bash
git add scripts/test_hf.cjs docs/FORMULA_TEST_REPORT.md
git commit -m "test: verify split array output"
```
