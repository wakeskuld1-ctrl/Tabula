// ### 变更记录
// - 2026-03-14 22:10: 原因=扩展公式文档生成测试; 目的=校验函数列表与表格结构
// - 2026-03-14 22:10: 原因=保持TDD覆盖; 目的=确保README生成具备最小可靠性
// - 2026-03-14 22:10: 原因=统一测试风格; 目的=继续使用node:test
const test = require("node:test");
const assert = require("node:assert/strict");

// ### 变更记录
// - 2026-03-14 22:10: 原因=测试需要访问生成器输出; 目的=集中导入被测函数
const {
  buildFormulaDocsSection,
  getRegisteredFunctions,
} = require("../generate_formula_docs.cjs");

// ### 变更记录
// - 2026-03-14 22:10: 原因=确保 HyperFormula 已注册函数; 目的=避免生成空表格
// - 2026-03-14 22:10: 原因=输出需要覆盖全量函数; 目的=先验证数量基础

test("function list should not be empty", () => {
  // ### 变更记录
  // - 2026-03-14 22:10: 原因=保障全量覆盖; 目的=列表为空时立即失败
  const fnList = getRegisteredFunctions();
  assert.ok(Array.isArray(fnList), "function list should be an array");
  assert.ok(fnList.length > 0, "function list should not be empty");
});

// ### 变更记录
// - 2026-03-14 22:10: 原因=README 表格需要固定表头; 目的=避免格式断裂
// - 2026-03-14 22:10: 原因=双语输出要求; 目的=确认表头包含双语字段

test("table header should exist", () => {
  // ### 变更记录
  // - 2026-03-14 22:10: 原因=表头是最小结构保障; 目的=确保生成内容可读
  const section = buildFormulaDocsSection();
  assert.ok(
    section.includes("| 函数名 / Function | 语法 / Syntax | 示例 / Example | 备注 / Notes |"),
    "table header should exist"
  );
});

// ### 变更记录
// - 2026-03-14 22:10: 原因=每个函数需有一行输出; 目的=保证行数覆盖
// - 2026-03-14 22:10: 原因=README 输出必须可扩展; 目的=粗略校验行数

test("section should include rows", () => {
  // ### 变更记录
  // - 2026-03-14 22:10: 原因=行数是全量覆盖信号; 目的=避免丢失函数
  const fnList = getRegisteredFunctions();
  const section = buildFormulaDocsSection();
  const lineCount = section.split("\n").length;
  assert.ok(lineCount > fnList.length, "section should include rows");
});
