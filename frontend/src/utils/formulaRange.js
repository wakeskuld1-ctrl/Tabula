// ### 变更记录
// - 2026-02-15: 原因=统一列名转换; 目的=依赖列解析与提示一致
// - 2026-02-15: 原因=与表格列索引一致; 目的=避免列名错位
export function getExcelColumnName(colIndex) {
  // ### 变更记录
  // - 2026-02-15: 原因=从 0 开始计算; 目的=与网格列索引对齐
  // - 2026-02-15: 原因=需要 Excel 风格; 目的=支持 A/AA/AB 形式
  let temp = colIndex + 1;
  let letter = "";
  while (temp > 0) {
    let t = (temp - 1) % 26;
    letter = String.fromCharCode(65 + t) + letter;
    temp = (temp - t - 1) / 26;
  }
  return letter;
}

// ### 变更记录
// - 2026-02-15: 原因=列名反向转换; 目的=用于公式解析依赖列
// - 2026-02-15: 原因=避免重复实现; 目的=集中管理索引逻辑
export function getExcelColumnIndex(colName) {
  // ### 变更记录
  // - 2026-02-15: 原因=与 Excel 规则一致; 目的=支持 A/AA/AB 等形式
  // - 2026-02-15: 原因=统一大写处理; 目的=减少大小写差异
  let index = 0;
  const name = colName.toUpperCase();
  for (let i = 0; i < name.length; i += 1) {
    index = index * 26 + (name.charCodeAt(i) - 64);
  }
  return index - 1;
}

// ### 变更记录
// - 2026-02-15: 原因=聚合公式识别; 目的=支持过期提示与刷新能力
// - 2026-02-15: 原因=轻量解析; 目的=避免引入额外依赖
export function parseAggregateFormula(input) {
  // ### 变更记录
  // - 2026-02-15: 原因=保证输入为字符串; 目的=避免非文本触发异常
  // - 2026-02-15: 原因=兼容大小写; 目的=与用户输入保持一致
  if (typeof input !== "string") return null;
  const regex =
    /^=\s*(SUM|COUNT|COUNTA|AVG|AVERAGE|MAX|MIN)\s*\(\s*([A-Z]+)(\d+)?\s*:\s*([A-Z]+)(\d+)?\s*\)\s*$/i;
  const match = input.match(regex);
  if (!match) return null;
  // ### 变更记录
  // - 2026-02-15: 原因=过滤 A1:B 这种不完整范围; 目的=避免错误依赖
  // - 2026-02-15: 原因=对齐范围语义; 目的=确保行号成对出现
  const hasStartRow = typeof match[3] === "string" && match[3].length > 0;
  const hasEndRow = typeof match[5] === "string" && match[5].length > 0;
  if (hasStartRow !== hasEndRow) return null;
  let func = match[1].toUpperCase();
  if (func === "AVERAGE") func = "AVG";
  if (func === "COUNTA") func = "COUNT";
  return {
    func,
    startCol: match[2].toUpperCase(),
    startRow: match[3] ? Number(match[3]) : null,
    endCol: match[4].toUpperCase(),
    endRow: match[5] ? Number(match[5]) : null,
  };
}

// ### 变更记录
// - 2026-02-17: 原因=集中聚合函数名单; 目的=弹窗提示与校验复用
// - 2026-02-17: 原因=避免散落硬编码; 目的=统一维护入口
export function getAggregateFunctionNames() {
  return ["SUM", "COUNT", "COUNTA", "AVG", "AVERAGE", "MAX", "MIN"];
}

// ### 变更记录
// - 2026-02-17: 原因=聚合函数识别; 目的=输入提示与拦截共用
// - 2026-02-17: 原因=统一大小写; 目的=降低输入差异
export function isAggregateFormulaFunction(rawFunc) {
  const name = String(rawFunc ?? "").toUpperCase();
  return getAggregateFunctionNames().includes(name);
}

// ### 变更记录
// - 2026-02-15: 原因=将解析结果转为范围信息; 目的=统一列依赖与范围类型
// - 2026-02-15: 原因=支持整列与指定行; 目的=覆盖 SUM(A:A) 与 SUM(A1:A5)
export function getRangeInfo(parsed) {
  // ### 变更记录
  // - 2026-02-15: 原因=处理空输入; 目的=提升健壮性
  // - 2026-02-15: 原因=避免无效解析; 目的=防止后续错误
  if (!parsed) return null;
  const hasStartRow = parsed.startRow !== null && !Number.isNaN(parsed.startRow);
  const hasEndRow = parsed.endRow !== null && !Number.isNaN(parsed.endRow);
  if (hasStartRow !== hasEndRow) return null;

  const startColIndex = getExcelColumnIndex(parsed.startCol);
  const endColIndex = getExcelColumnIndex(parsed.endCol);
  const minCol = Math.min(startColIndex, endColIndex);
  const maxCol = Math.max(startColIndex, endColIndex);
  const columns = [];
  for (let i = minCol; i <= maxCol; i += 1) {
    columns.push(getExcelColumnName(i));
  }

  if (!hasStartRow && !hasEndRow) {
    // ### 变更记录
    // - 2026-02-15: 原因=整列范围; 目的=标记为 column 类型
    // - 2026-02-15: 原因=无法确定行数; 目的=cellCount 设为 null
    return {
      type: "column",
      columns,
      cellCount: null,
      startRow: null,
      endRow: null,
    };
  }

  const minRow = Math.min(parsed.startRow, parsed.endRow);
  const maxRow = Math.max(parsed.startRow, parsed.endRow);
  const rowCount = maxRow - minRow + 1;
  const cellCount = rowCount * columns.length;
  // ### 变更记录
  // - 2026-02-15: 原因=指定行范围; 目的=标记为 cell 类型
  // - 2026-02-15: 原因=便于阈值判断; 目的=提供 cellCount
  return {
    type: "cell",
    columns,
    cellCount,
    startRow: minRow,
    endRow: maxRow,
  };
}

// ### 变更记录
// - 2026-02-16: 原因=新增公式列表达式解析; 目的=将列字母映射为真实列名
// - 2026-02-16: 原因=保障 SQL 生成安全; 目的=限制输入字符范围
export function buildFormulaColumnSql(rawExpression, columns) {
  // ### 变更记录
  // - 2026-02-16: 原因=兼容空输入; 目的=避免无效表达式进入后端
  // - 2026-02-16: 原因=保证列集合有效; 目的=避免访问越界
  if (typeof rawExpression !== "string" || !Array.isArray(columns)) return null;
  const trimmed = rawExpression.trim();
  if (!trimmed) return null;

  // ### 变更记录
  // - 2026-02-16: 原因=统一大小写; 目的=列名解析一致
  // - 2026-02-16: 原因=限制非法字符; 目的=降低 SQL 注入风险
  const normalized = trimmed.toUpperCase();
  const allowed = /^[A-Z0-9+\-*/().\s]+$/;
  if (!allowed.test(normalized)) return null;

  let invalid = false;
  // ### 变更记录
  // - 2026-02-16: 原因=定位列字母; 目的=根据索引映射列名
  // - 2026-02-16: 原因=防止越界访问; 目的=非法列返回 null
  const sql = normalized.replace(/[A-Z]+/g, (token) => {
    const colIndex = getExcelColumnIndex(token);
    if (colIndex < 0 || colIndex >= columns.length) {
      invalid = true;
      return token;
    }
    const escaped = String(columns[colIndex]).replace(/"/g, "\"\"");
    return `"${escaped}"`;
  });

  if (invalid) return null;
  return sql.replace(/\s+/g, "");
}

// ### 变更记录
// - 2026-02-16: 原因=新增公式列 marker 构造; 目的=统一前后端公式字段
// - 2026-02-16: 原因=集中校验入口; 目的=减少调用方重复判断
export function buildFormulaColumnMarker(rawExpression, columns) {
  // ### 变更记录
  // - 2026-02-16: 原因=兼容空输入; 目的=避免无效 marker 下发
  // - 2026-02-16: 原因=清洗空白; 目的=保持 raw 一致
  if (typeof rawExpression !== "string") return null;
  const trimmed = rawExpression.trim();
  if (!trimmed) return null;
  const sql = buildFormulaColumnSql(trimmed, columns);
  if (!sql) return null;
  return {
    kind: "formula",
    raw: trimmed,
    sql
  };
}

// ### 变更记录
// - 2026-02-17: 原因=算术公式需要标准化; 目的=统一列名与空白
// - 2026-02-17: 原因=拦截非算术表达式; 目的=避免误判 IF 等函数
export function normalizeArithmeticFormula(rawExpression) {
  // ### 变更记录
  // - 2026-02-17: 原因=保护非字符串输入; 目的=避免运行时异常
  // - 2026-02-17: 原因=空值直接忽略; 目的=不影响原有流程
  if (typeof rawExpression !== "string") return null;
  const trimmed = rawExpression.trim();
  if (!trimmed) return null;
  // ### 变更记录
  // - 2026-02-17: 原因=统一大写; 目的=保证列解析一致
  // - 2026-02-17: 原因=限制字符集合; 目的=仅允许算术表达式
  const normalized = trimmed.toUpperCase();
  const allowed = /^[A-Z0-9+\-*/().\s]+$/;
  if (!allowed.test(normalized)) return null;
  return normalized.replace(/\s+/g, "");
}

// ### 变更记录
// - 2026-02-17: 原因=提取算术公式列; 目的=支持数值型校验
// - 2026-02-17: 原因=保证顺序稳定; 目的=错误提示一致
export function extractArithmeticFormulaColumns(rawExpression) {
  // ### 变更记录
  // - 2026-02-17: 原因=复用标准化; 目的=统一判定入口
  // - 2026-02-17: 原因=非算术直接返回; 目的=跳过 IF 类公式
  const normalized = normalizeArithmeticFormula(rawExpression);
  if (!normalized) return null;
  const matches = normalized.match(/[A-Z]+/g) || [];
  const seen = new Set();
  const columns = [];
  for (const token of matches) {
    if (!seen.has(token)) {
      seen.add(token);
      columns.push(token);
    }
  }
  return columns;
}

// ### 变更记录
// - 2026-02-17: 原因=将列名映射索引; 目的=校验列类型
// - 2026-02-17: 原因=越界即失败; 目的=避免错误提示遗漏
export function getArithmeticFormulaColumnIndexes(rawExpression, columnCount) {
  // ### 变更记录
  // - 2026-02-17: 原因=列数非法时终止; 目的=避免误判
  // - 2026-02-17: 原因=非算术公式返回 null; 目的=保持调用方简洁
  if (!Number.isFinite(columnCount) || columnCount <= 0) return null;
  const columns = extractArithmeticFormulaColumns(rawExpression);
  if (!columns) return null;
  const indexes = [];
  for (const name of columns) {
    const idx = getExcelColumnIndex(name);
    if (idx < 0 || idx >= columnCount) return null;
    indexes.push(idx);
  }
  return indexes;
}

// ### 变更记录
// - 2026-02-16: 原因=公式列需要列名输入; 目的=统一校验空值与空白
// - 2026-02-16: 原因=避免重复逻辑; 目的=集中处理列名规范化
export function validateFormulaColumnName(name) {
  if (typeof name !== "string") return null;
  const trimmed = name.trim();
  return trimmed.length > 0 ? trimmed : null;
}

// ### 变更记录
// - 2026-02-16: 原因=新增公式列索引判断; 目的=前端只读逻辑复用
// - 2026-02-16: 原因=与后端字段对齐; 目的=减少字段名误用
export function isFormulaColumnIndex(colIndex, formulaColumns) {
  // ### 变更记录
  // - 2026-02-16: 原因=空集合快速返回; 目的=避免额外遍历
  // - 2026-02-16: 原因=类型保护; 目的=避免运行时异常
  if (!Array.isArray(formulaColumns)) return false;
  return formulaColumns.some((col) => col && col.index === colIndex);
}

// ### 变更记录
// - 2026-02-16: 原因=公式栏需要显示 raw; 目的=选中公式列展示原始表达式
// - 2026-02-16: 原因=非公式列回退; 目的=保持原值显示
export function getFormulaColumnDisplayValue(colIndex, formulaColumns, fallback) {
  // ### 变更记录
  // - 2026-02-16: 原因=容错处理; 目的=避免空引用
  // - 2026-02-16: 原因=优先 raw_expression; 目的=显示用户输入
  if (!Array.isArray(formulaColumns)) return fallback;
  const target = formulaColumns.find((col) => col && col.index === colIndex);
  if (!target || typeof target.raw_expression !== "string") return fallback;
  return target.raw_expression;
}

// ### 变更记录
// - 2026-02-16: 原因=新增单元格格式化入口; 目的=统一格式化逻辑
// - 2026-02-16: 原因=避免 UI 直接处理; 目的=集中管理可测试
export function formatCellValue(rawValue, format) {
  // ### 变更记录
  // - 2026-02-16: 原因=支持空值; 目的=避免 undefined 展示
  // - 2026-02-16: 原因=保证字符串回退; 目的=避免渲染异常
  if (rawValue === null || rawValue === undefined) return "";
  const rawText = String(rawValue);
  if (!format) return rawText;

  // ### 变更记录
  // - 2026-02-16: 原因=数值解析; 目的=统一校验合法性
  // - 2026-02-16: 原因=非法值回退; 目的=保持原始展示
  const parseNumber = () => {
    const num = Number(rawText);
    return Number.isFinite(num) ? num : null;
  };

  if (format === "number") {
    const num = parseNumber();
    if (num === null) return rawText;
    return new Intl.NumberFormat("en-US").format(num);
  }

  if (format === "percent") {
    const num = parseNumber();
    if (num === null) return rawText;
    const percent = num * 100;
    return `${new Intl.NumberFormat("en-US").format(percent)}%`;
  }

  if (format === "currency") {
    const num = parseNumber();
    if (num === null) return rawText;
    return new Intl.NumberFormat("zh-CN", {
      style: "currency",
      currency: "CNY",
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(num);
  }

  if (format === "date") {
    const time = Date.parse(rawText);
    if (Number.isNaN(time)) return rawText;
    const date = new Date(time);
    const yyyy = date.getFullYear();
    const mm = String(date.getMonth() + 1).padStart(2, "0");
    const dd = String(date.getDate()).padStart(2, "0");
    return `${yyyy}-${mm}-${dd}`;
  }

  return rawText;
}

// ### 变更记录
// - 2026-02-15: 原因=保持默认导出; 目的=兼容不同导入方式
// - 2026-02-15: 原因=集中导出; 目的=调用侧可读性提升
export default {
  parseAggregateFormula,
  getAggregateFunctionNames,
  isAggregateFormulaFunction,
  getRangeInfo,
  getExcelColumnIndex,
  getExcelColumnName,
  buildFormulaColumnSql,
  buildFormulaColumnMarker,
  validateFormulaColumnName,
  isFormulaColumnIndex,
  getFormulaColumnDisplayValue,
  formatCellValue,
};
