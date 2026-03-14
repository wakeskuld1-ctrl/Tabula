// ### 变更记录
// - 2026-03-14 22:40: 原因=扩展公式文档生成测试; 目的=校验函数列表与表格结构
// - 2026-03-14 22:40: 原因=保持TDD覆盖; 目的=确保README生成具备最小可靠性
// - 2026-03-14 22:40: 原因=统一测试风格; 目的=继续使用node:test
// - 2026-03-14 22:40: 原因=新增参数说明与用途列; 目的=覆盖别名与用途输出
const test = require("node:test");
const assert = require("node:assert/strict");

// ### 变更记录
// - 2026-03-14 22:40: 原因=测试需要访问生成器输出; 目的=集中导入被测函数
// - 2026-03-14 22:40: 原因=注入测试需要标记常量; 目的=验证替换区间
const {
  buildFormulaDocsSection,
  getRegisteredFunctions,
  buildInjectedContent,
  FORMULA_DOCS_START,
  FORMULA_DOCS_END,
} = require("../generate_formula_docs.cjs");

// ### 变更记录
// - 2026-03-14 22:40: 原因=确保 HyperFormula 已注册函数; 目的=避免生成空表格
// - 2026-03-14 22:40: 原因=输出需要覆盖全量函数; 目的=先验证数量基础

test("function list should not be empty", () => {
  // ### 变更记录
  // - 2026-03-14 22:40: 原因=保障全量覆盖; 目的=列表为空时立即失败
  const fnList = getRegisteredFunctions();
  assert.ok(Array.isArray(fnList), "function list should be an array");
  assert.ok(fnList.length > 0, "function list should not be empty");
});

// ### 变更记录
// - 2026-03-14 22:40: 原因=README 表格需要固定表头; 目的=避免格式断裂
// - 2026-03-14 22:40: 原因=双语输出要求; 目的=确认表头包含双语字段
// - 2026-03-14 22:40: 原因=新增参数说明与用途列; 目的=确保列同步更新

test("table header should include new columns", () => {
  // ### 变更记录
  // - 2026-03-14 22:40: 原因=表头是最小结构保障; 目的=确保生成内容可读
  const section = buildFormulaDocsSection();
  assert.ok(
    section.includes("| 函数名 / Function | 语法 / Syntax | 示例 / Example | 参数说明 / Parameter Notes | 用途 / Purpose | 备注 / Notes |"),
    "table header should include new columns"
  );
});

// ### 变更记录
// - 2026-03-14 22:40: 原因=每个函数需有一行输出; 目的=保证行数覆盖
// - 2026-03-14 22:40: 原因=README 输出必须可扩展; 目的=粗略校验行数

test("section should include rows", () => {
  // ### 变更记录
  // - 2026-03-14 22:40: 原因=行数是全量覆盖信号; 目的=避免丢失函数
  const fnList = getRegisteredFunctions();
  const section = buildFormulaDocsSection();
  const lineCount = section.split("\n").length;
  assert.ok(lineCount > fnList.length, "section should include rows");
});

// ### 变更记录
// - 2026-03-14 22:40: 原因=README 注入需要替换标记块; 目的=验证注入函数行为
// - 2026-03-14 22:40: 原因=避免误替换; 目的=确保旧内容被移除

test("inject should replace markers", () => {
  // ### 变更记录
  // - 2026-03-14 22:40: 原因=构造最小样例; 目的=确认注入区间生效
  const sample = `${FORMULA_DOCS_START}\nOLD\n${FORMULA_DOCS_END}`;
  const injected = buildInjectedContent(sample);
  assert.ok(!injected.includes("OLD"), "old content should be removed");
  assert.ok(
    injected.includes("| 函数名 / Function | 语法 / Syntax | 示例 / Example | 参数说明 / Parameter Notes | 用途 / Purpose | 备注 / Notes |"),
    "injected content should include new table header"
  );
});

// ### 变更记录
// - 2026-03-15 00:30: 原因=新增公式提示JSON输出; 目的=保障UI数据源完整
// - 2026-03-15 00:30: 原因=避免遗漏字段; 目的=断言关键属性存在
test("formula help data should be exported", () => {
  // ### 变更记录
  // - 2026-03-15 00:30: 原因=生成器需提供结构化数据; 目的=供前端抽屉使用
  const { buildFormulaHelpData } = require("../generate_formula_docs.cjs");
  const data = buildFormulaHelpData();
  if (!Array.isArray(data)) {
    throw new Error("formula help data should be an array");
  }
  if (data.length === 0) {
    throw new Error("formula help data should not be empty");
  }
  const sample = data[0];
  if (!sample || typeof sample !== "object") {
    throw new Error("formula help item should be an object");
  }
  if (!("name" in sample) || !("syntax" in sample) || !("example" in sample)) {
    throw new Error("formula help item should include name/syntax/example");
  }
});
