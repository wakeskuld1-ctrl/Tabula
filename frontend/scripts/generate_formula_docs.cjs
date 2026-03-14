// ### 变更记录
// - 2026-03-14 22:05: 原因=需要读取 HyperFormula 全量函数; 目的=生成 README 全量公式表格
// - 2026-03-14 22:05: 原因=保留生成器可扩展性; 目的=将语法/示例生成逻辑拆分为函数
// - 2026-03-14 22:05: 原因=满足双语输出要求; 目的=表头与备注同时提供中英文
// - 2026-03-14 22:20: 原因=需要读写 README 文件; 目的=为注入逻辑准备 fs/path
// - 2026-03-14 23:05: 原因=参数别名与用途说明需外部配置; 目的=引入别名映射表
const fs = require("node:fs");
const path = require("node:path");
const { HyperFormula, FunctionArgumentType } = require("hyperformula");
const {
  typeAliases,
  functionAliases,
  functionPurposeOverrides,
  purposeRules,
} = require("./formula_alias_map.cjs");

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
// - 2026-03-14 23:05: 原因=类型标签需与别名保持一致; 目的=统一映射入口
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
// - 2026-03-14 23:05: 原因=与别名类型对齐; 目的=避免示例与参数不一致
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
// - 2026-03-14 23:05: 原因=别名依赖规范化类型; 目的=保证参数别名一致
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
// - 2026-03-14 23:05: 原因=需要从别名表读取中英文映射; 目的=生成双语参数说明
// - 2026-03-14 23:05: 原因=缺失映射时仍需输出; 目的=提供安全兜底
function getTypeAlias(typeKey) {
  const fallback = typeAliases?.value || { en: "value", cn: "值" };
  if (!typeKey) {
    return fallback;
  }
  return typeAliases?.[typeKey] || fallback;
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
// - 2026-03-14 23:05: 原因=新增参数别名与用途; 目的=输出业务化解释
function buildFormulaUsage(functionName, params) {
  if (!params) {
    return {
      syntax: "—",
      example: "—",
      paramNotes: "—",
      purpose: "不可用 / Unavailable",
      note: "参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup",
    };
  }

  const upperName = String(functionName).toUpperCase();
  const aliasOverride = functionAliases?.[upperName];
  const aliasList = params.map((param, index) => {
    const typeKey = normalizeArgumentType(param?.argumentType);
    const baseAlias = aliasOverride?.[index] || null;
    if (baseAlias) {
      return {
        en: baseAlias.en,
        cn: baseAlias.cn,
        optional: Boolean(param?.optionalArg),
        typeKey,
      };
    }
    const typeAlias = getTypeAlias(typeKey);
    const suffix = params.length > 1 ? String(index + 1) : "";
    return {
      en: `${typeAlias.en}${suffix}`,
      cn: `${typeAlias.cn}${suffix}`,
      optional: Boolean(param?.optionalArg),
      typeKey,
    };
  });

  const required = aliasList.filter((item) => !item.optional);
  const syntaxParts = aliasList.map((item) => {
    const label = item.en || "value";
    return item.optional ? `[${label}]` : label;
  });
  const exampleParts = required.map((item) => {
    const key = item.typeKey || "value";
    return ARG_TYPE_EXAMPLES[key] ?? ARG_TYPE_EXAMPLES.value;
  });

  let paramNotes = "无参数 / No parameters";
  if (aliasList.length > 0) {
    paramNotes = aliasList
      .map((item) => {
        const suffix = item.optional ? " (optional/可选)" : "";
        return `${item.en}/${item.cn}${suffix}`;
      })
      .join(", ");
  }

  const purposeOverride = functionPurposeOverrides?.[upperName];
  let purpose = purposeOverride
    ? `${purposeOverride.cn} / ${purposeOverride.en}`
    : "通用计算 / General calculation";
  if (!purposeOverride && Array.isArray(purposeRules)) {
    const matched = purposeRules.find((rule) => rule.pattern?.test(upperName));
    if (matched) {
      purpose = `${matched.cn} / ${matched.en}`;
    }
  }

  return {
    syntax: `${functionName}(${syntaxParts.join(", ")})`,
    example: `=${functionName}(${exampleParts.join(", ")})`,
    paramNotes,
    purpose,
    note: "—",
  };
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=README 表格需要双语表头; 目的=满足国际化阅读
// - 2026-03-14 22:05: 原因=输出需稳定可 diff; 目的=固定表头与分隔行
// - 2026-03-14 23:05: 原因=新增参数说明与用途列; 目的=补足业务化信息
function buildFormulaDocsTable(functionNames) {
  const header = "| 函数名 / Function | 语法 / Syntax | 示例 / Example | 参数说明 / Parameter Notes | 用途 / Purpose | 备注 / Notes |";
  const divider = "| --- | --- | --- | --- | --- | --- |";
  const rows = functionNames.map((name) => {
    const params = getFunctionParameters(name);
    const usage = buildFormulaUsage(name, params);
    return `| ${escapeTableCell(name)} | ${escapeTableCell(usage.syntax)} | ${escapeTableCell(usage.example)} | ${escapeTableCell(usage.paramNotes)} | ${escapeTableCell(usage.purpose)} | ${escapeTableCell(usage.note)} |`;
  });
  return [header, divider, ...rows].join("\n");
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=README 注入需要完整区块; 目的=集中输出标记块内容
// - 2026-03-14 22:05: 原因=函数列表为空时需提示; 目的=避免 README 空白
// - 2026-03-14 23:05: 原因=表头新增列; 目的=保持空表结构一致
function buildFormulaDocsSection() {
  const functionNames = getRegisteredFunctions();
  const table =
    functionNames.length > 0
      ? buildFormulaDocsTable(functionNames)
      : "| 函数名 / Function | 语法 / Syntax | 示例 / Example | 参数说明 / Parameter Notes | 用途 / Purpose | 备注 / Notes |\n| --- | --- | --- | --- | --- | --- |\n| 无 / None | — | — | — | — | 未读取到函数列表 / Function list empty |";
  return `${FORMULA_DOCS_START}\n${table}\n${FORMULA_DOCS_END}\n`;
}

// ### 变更记录
// - 2026-03-14 22:20: 原因=注入逻辑需要纯函数便于测试; 目的=避免直接写文件导致测试副作用
// - 2026-03-14 22:20: 原因=标记块替换需安全; 目的=缺失标记时直接抛错
function buildInjectedContent(rawContent) {
  const startIndex = rawContent.indexOf(FORMULA_DOCS_START);
  const endIndex = rawContent.indexOf(FORMULA_DOCS_END);
  if (startIndex < 0 || endIndex < 0 || endIndex < startIndex) {
    throw new Error("Formula docs markers not found");
  }
  const before = rawContent.slice(0, startIndex);
  const after = rawContent.slice(endIndex + FORMULA_DOCS_END.length);
  const section = buildFormulaDocsSection();
  return `${before}${section}${after}`;
}

// ### 变更记录
// - 2026-03-14 22:20: 原因=脚本需要写入 README; 目的=集中处理文件读写
// - 2026-03-14 22:20: 原因=保持编码一致; 目的=统一使用 UTF-8
function writeFormulaDocsToFile(filePath) {
  const raw = fs.readFileSync(filePath, "utf8");
  const next = buildInjectedContent(raw);
  fs.writeFileSync(filePath, next, "utf8");
}

// ### 变更记录
// - 2026-03-14 22:20: 原因=两份 README 需要同步; 目的=提供默认路径列表
// - 2026-03-14 22:20: 原因=脚本应可独立运行; 目的=自动定位根与前端 README
function getDefaultReadmePaths() {
  const frontendReadme = path.resolve(__dirname, "..", "README.md");
  const rootReadme = path.resolve(__dirname, "..", "..", "README.md");
  return [rootReadme, frontendReadme];
}

// ### 变更记录
// - 2026-03-14 22:20: 原因=便于命令行执行; 目的=支持传入路径或使用默认路径
// - 2026-03-14 22:20: 原因=避免静默失败; 目的=明确输出处理结果
function runCli() {
  const args = process.argv.slice(2);
  const targets = args.length > 0 ? args : getDefaultReadmePaths();
  targets.forEach((filePath) => {
    writeFormulaDocsToFile(filePath);
    // eslint-disable-next-line no-console
    console.log(`[formula-docs] updated ${filePath}`);
  });
}

// ### 变更记录
// - 2026-03-14 22:05: 原因=测试与脚本复用; 目的=统一导出入口
module.exports = {
  buildFormulaDocsSection,
  getRegisteredFunctions,
  buildFormulaDocsTable,
  FORMULA_DOCS_START,
  FORMULA_DOCS_END,
  buildInjectedContent,
  writeFormulaDocsToFile,
  getDefaultReadmePaths,
  runCli,
};

// ### 变更记录
// - 2026-03-14 22:20: 原因=支持直接运行脚本; 目的=简化 README 更新流程
if (require.main === module) {
  runCli();
}
