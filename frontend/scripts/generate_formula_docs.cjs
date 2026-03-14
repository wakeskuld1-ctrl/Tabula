// ### 变更记录
// - 2026-03-14 22:05: 原因=需要读取 HyperFormula 全量函数; 目的=生成 README 全量公式表格
// - 2026-03-14 22:05: 原因=保留生成器可扩展性; 目的=将语法/示例生成逻辑拆分为函数
// - 2026-03-14 22:05: 原因=满足双语输出要求; 目的=表头与备注同时提供中英文
const { HyperFormula, FunctionArgumentType } = require("hyperformula");

// ### 变更记录
// - 2026-03-14 22:05: 原因=统一注入标记; 目的=避免 README 替换错位
// - 2026-03-14 22:05: 原因=多文件复用标记; 目的=根 README 与 frontend README 一致
const FORMULA_DOCS_START = "<!-- FORMULA_DOCS_START -->";
const FORMULA_DOCS_END = "<!-- FORMULA_DOCS_END -->";

// ### 变更记录
// - 2026-03-14 22:15: 原因=静态函数需要语言代码; 目的=与 HyperFormula API 对齐
// - 2026-03-14 22:15: 原因=保持函数名为英文; 目的=与公式实际输入一致
const FUNCTION_LANGUAGE_CODE = "enGB";

// ### 变更记录
// - 2026-03-14 22:05: 原因=参数类型需映射为人类可读文本; 目的=生成双语语法提示
// - 2026-03-14 22:05: 原因=示例需要稳定占位; 目的=避免不同平台输出不一致
const ARG_TYPE_LABELS = {
  NUMBER: "number",
  INTEGER: "integer",
  STRING: "text",
  BOOLEAN: "boolean",
  RANGE: "range",
  ANY: "value",
  SCALAR: "value",
  DATE: "date",
  TIME: "time",
  COMPLEX: "complex",
  CELL_REFERENCE: "cell",
  CELL_RANGE: "range",
  MATRIX: "array",
  ERROR: "error",
};

// ### 变更记录
// - 2026-03-14 22:05: 原因=示例需跟随类型; 目的=让每个函数拥有可读示例
// - 2026-03-14 22:05: 原因=保持模板一致; 目的=便于批量清理与替换
const ARG_TYPE_EXAMPLES = {
  number: "1",
  integer: "1",
  text: "\"text\"",
  boolean: "TRUE",
  range: "A1:A5",
  value: "1",
  date: "\"2026-03-14\"",
  time: "\"12:00\"",
  complex: "\"1+2i\"",
  cell: "A1",
  array: "{1,2,3}",
  error: "#VALUE!",
};

// ### 变更记录
// - 2026-03-14 22:05: 原因=Markdown 表格需要转义; 目的=避免 | 破坏布局
function escapeTableCell(value) {
  return String(value ?? "").replace(/\|/g, "\\|");
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=HyperFormula 类型可能为枚举或字符串; 目的=统一转换为文本标签
// - 2026-03-14 22:05: 原因=缺失类型时需兜底; 目的=避免生成 undefined
function normalizeArgumentType(argumentType) {
  if (argumentType === undefined || argumentType === null) {
    return "value";
  }
  if (typeof argumentType === "number") {
    const enumName = FunctionArgumentType[argumentType];
    return (ARG_TYPE_LABELS[enumName] || enumName || "value").toLowerCase();
  }
  if (typeof argumentType === "string") {
    const upper = argumentType.toUpperCase();
    return (ARG_TYPE_LABELS[upper] || upper || "value").toLowerCase();
  }
  return "value";
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=README 需要全量函数清单; 目的=稳定输出排序后的名称列表
// - 2026-03-14 22:05: 原因=避免空数组导致后续异常; 目的=统一返回空列表
function getRegisteredFunctions() {
  let raw = [];
  try {
    raw = HyperFormula.getRegisteredFunctionNames?.(FUNCTION_LANGUAGE_CODE) || [];
  } catch (error) {
    raw = [];
  }
  if (!Array.isArray(raw)) {
    return [];
  }
  return raw.slice().sort((a, b) => String(a).localeCompare(String(b)));
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=语法生成依赖元数据; 目的=读取插件静态定义
// - 2026-03-14 22:05: 原因=不同插件可能缺失元数据; 目的=返回 null 以触发降级
function getFunctionParameters(functionName) {
  const plugin = HyperFormula.getFunctionPlugin?.(functionName);
  if (!plugin || !plugin.implementedFunctions) {
    return null;
  }
  const meta =
    plugin.implementedFunctions[functionName] ||
    plugin.implementedFunctions[String(functionName).toUpperCase()];
  if (!meta || !Array.isArray(meta.parameters)) {
    return null;
  }
  return meta.parameters;
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=语法与示例需保持同步; 目的=统一由一处生成
// - 2026-03-14 22:05: 原因=参数不可用时不得提供用法; 目的=符合“不可用不提供”要求
function buildFormulaUsage(functionName, params) {
  if (!params) {
    return {
      syntax: "—",
      example: "—",
      note: "参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup",
    };
  }

  const required = params.filter((param) => !param?.optionalArg);
  const syntaxParts = params.map((param, index) => {
    const label = normalizeArgumentType(param?.argumentType);
    const base = `${label}${index + 1}`;
    return param?.optionalArg ? `[${base}]` : base;
  });
  const exampleParts = required.map((param) => {
    const label = normalizeArgumentType(param?.argumentType);
    return ARG_TYPE_EXAMPLES[label] ?? ARG_TYPE_EXAMPLES.value;
  });

  return {
    syntax: `${functionName}(${syntaxParts.join(", ")})`,
    example: `=${functionName}(${exampleParts.join(", ")})`,
    note: "—",
  };
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=README 表格需要双语表头; 目的=满足国际化阅读
// - 2026-03-14 22:05: 原因=输出需稳定可 diff; 目的=固定表头与分隔行
function buildFormulaDocsTable(functionNames) {
  const header = "| 函数名 / Function | 语法 / Syntax | 示例 / Example | 备注 / Notes |";
  const divider = "| --- | --- | --- | --- |";
  const rows = functionNames.map((name) => {
    const params = getFunctionParameters(name);
    const usage = buildFormulaUsage(name, params);
    return `| ${escapeTableCell(name)} | ${escapeTableCell(usage.syntax)} | ${escapeTableCell(usage.example)} | ${escapeTableCell(usage.note)} |`;
  });
  return [header, divider, ...rows].join("\n");
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=README 注入需要完整区块; 目的=集中输出标记块内容
// - 2026-03-14 22:05: 原因=函数列表为空时需提示; 目的=避免 README 空白
function buildFormulaDocsSection() {
  const functionNames = getRegisteredFunctions();
  const table =
    functionNames.length > 0
      ? buildFormulaDocsTable(functionNames)
      : "| 函数名 / Function | 语法 / Syntax | 示例 / Example | 备注 / Notes |\n| --- | --- | --- | --- |\n| 无 / None | — | — | 未读取到函数列表 / Function list empty |";
  return `${FORMULA_DOCS_START}\n${table}\n${FORMULA_DOCS_END}\n`;
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=测试与脚本复用; 目的=统一导出入口
module.exports = {
  buildFormulaDocsSection,
  getRegisteredFunctions,
  buildFormulaDocsTable,
  FORMULA_DOCS_START,
  FORMULA_DOCS_END,
};
