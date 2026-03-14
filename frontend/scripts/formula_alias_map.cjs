// ### 变更记录
// - 2026-03-14 22:55: 原因=参数别名需要集中管理; 目的=提供可维护的业务术语映射
// - 2026-03-14 22:55: 原因=用途说明需要统一出口; 目的=保证中英文一致
// - 2026-03-14 22:55: 原因=后续扩展需要清晰结构; 目的=拆分类型/函数/规则三类配置

// ### 变更记录
// - 2026-03-14 22:55: 原因=类型别名是默认兜底; 目的=生成通用参数名
// - 2026-03-14 22:55: 原因=国际化要求; 目的=提供中英文成对别名
const typeAliases = {
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=数值参数最常见; 目的=提供标准中英文
  number: { en: "number", cn: "数值" },
  integer: { en: "integer", cn: "整数" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=文本参数多用于字符串函数; 目的=统一为 text/文本
  text: { en: "text", cn: "文本" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=布尔参数通常表示条件; 目的=贴近业务语义
  boolean: { en: "condition", cn: "条件" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=范围是表格常用输入; 目的=统一为 range/范围
  range: { en: "range", cn: "范围" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=通用值参数频繁出现; 目的=提供 value/值 兜底
  value: { en: "value", cn: "值" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=日期/时间有专用语义; 目的=保持可读性
  date: { en: "date", cn: "日期" },
  time: { en: "time", cn: "时间" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=数组/矩阵函数需要标识; 目的=提高理解度
  array: { en: "array", cn: "数组" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=单元格引用常见; 目的=用 cell/单元格 标识
  cell: { en: "cell", cn: "单元格" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=错误参数少见但存在; 目的=给出明确说明
  error: { en: "error", cn: "错误" },
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=复数/工程函数可能出现; 目的=统一为 complex/复数
  complex: { en: "complex", cn: "复数" },
};

// ### 变更记录
// - 2026-03-14 22:55: 原因=常用函数需要业务化别名; 目的=语法更贴近实际使用
// - 2026-03-14 22:55: 原因=参数顺序需与公式一致; 目的=避免误导
const functionAliases = {
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=IF 函数频繁使用; 目的=提供语义明确的参数名
  IF: [
    { en: "condition", cn: "条件" },
    { en: "value_if_true", cn: "真值" },
    { en: "value_if_false", cn: "假值" },
  ],
  IFS: [
    { en: "condition1", cn: "条件1" },
    { en: "value1", cn: "值1" },
    { en: "condition2", cn: "条件2" },
    { en: "value2", cn: "值2" },
  ],
  AND: [{ en: "condition1", cn: "条件1" }],
  OR: [{ en: "condition1", cn: "条件1" }],
  NOT: [{ en: "condition", cn: "条件" }],
  IFERROR: [
    { en: "value", cn: "值" },
    { en: "value_if_error", cn: "出错替代值" },
  ],
  IFNA: [
    { en: "value", cn: "值" },
    { en: "value_if_na", cn: "NA替代值" },
  ],
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=统计类函数常用; 目的=统一范围/条件参数
  SUM: [{ en: "range", cn: "范围" }],
  SUMIF: [
    { en: "range", cn: "范围" },
    { en: "criteria", cn: "条件" },
    { en: "sum_range", cn: "求和范围" },
  ],
  SUMIFS: [
    { en: "sum_range", cn: "求和范围" },
    { en: "criteria_range1", cn: "条件范围1" },
    { en: "criteria1", cn: "条件1" },
  ],
  COUNT: [{ en: "range", cn: "范围" }],
  COUNTIF: [
    { en: "range", cn: "范围" },
    { en: "criteria", cn: "条件" },
  ],
  COUNTIFS: [
    { en: "criteria_range1", cn: "条件范围1" },
    { en: "criteria1", cn: "条件1" },
  ],
  AVERAGE: [{ en: "range", cn: "范围" }],
  AVERAGEIF: [
    { en: "range", cn: "范围" },
    { en: "criteria", cn: "条件" },
    { en: "average_range", cn: "平均范围" },
  ],
  AVERAGEIFS: [
    { en: "average_range", cn: "平均范围" },
    { en: "criteria_range1", cn: "条件范围1" },
    { en: "criteria1", cn: "条件1" },
  ],
  MAX: [{ en: "range", cn: "范围" }],
  MIN: [{ en: "range", cn: "范围" }],
  MEDIAN: [{ en: "range", cn: "范围" }],
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=查找类函数使用场景明确; 目的=业务化列名
  VLOOKUP: [
    { en: "lookup_value", cn: "查找值" },
    { en: "table", cn: "表" },
    { en: "return_col", cn: "返回列" },
    { en: "join_col", cn: "匹配列" },
  ],
  HLOOKUP: [
    { en: "lookup_value", cn: "查找值" },
    { en: "table", cn: "表" },
    { en: "return_row", cn: "返回行" },
    { en: "join_row", cn: "匹配行" },
  ],
  XLOOKUP: [
    { en: "lookup_value", cn: "查找值" },
    { en: "table", cn: "表" },
    { en: "join_col", cn: "匹配列" },
    { en: "return_col", cn: "返回列" },
    { en: "if_not_found", cn: "未找到返回值" },
  ],
  INDEX: [
    { en: "range", cn: "范围" },
    { en: "row", cn: "行号" },
    { en: "column", cn: "列号" },
  ],
  MATCH: [
    { en: "lookup_value", cn: "查找值" },
    { en: "range", cn: "范围" },
    { en: "match_type", cn: "匹配方式" },
  ],
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=文本处理函数高频; 目的=明确输入与长度含义
  LEFT: [
    { en: "text", cn: "文本" },
    { en: "num_chars", cn: "字符数" },
  ],
  RIGHT: [
    { en: "text", cn: "文本" },
    { en: "num_chars", cn: "字符数" },
  ],
  MID: [
    { en: "text", cn: "文本" },
    { en: "start_num", cn: "起始位置" },
    { en: "num_chars", cn: "字符数" },
  ],
  LEN: [{ en: "text", cn: "文本" }],
  CONCAT: [{ en: "text1", cn: "文本1" }],
  CONCATENATE: [{ en: "text1", cn: "文本1" }],
  TEXTJOIN: [
    { en: "delimiter", cn: "分隔符" },
    { en: "ignore_empty", cn: "忽略空值" },
    { en: "text1", cn: "文本1" },
  ],
  SPLIT: [
    { en: "text", cn: "文本" },
    { en: "delimiter", cn: "分隔符" },
    { en: "split_by_each", cn: "按字符拆分" },
    { en: "remove_empty", cn: "移除空值" },
  ],
  SUBSTITUTE: [
    { en: "text", cn: "文本" },
    { en: "old_text", cn: "旧文本" },
    { en: "new_text", cn: "新文本" },
    { en: "instance_num", cn: "替换次数" },
  ],
  REPLACE: [
    { en: "old_text", cn: "旧文本" },
    { en: "start_num", cn: "起始位置" },
    { en: "num_chars", cn: "字符数" },
    { en: "new_text", cn: "新文本" },
  ],
  FIND: [
    { en: "find_text", cn: "查找文本" },
    { en: "within_text", cn: "范围文本" },
    { en: "start_num", cn: "起始位置" },
  ],
  SEARCH: [
    { en: "find_text", cn: "查找文本" },
    { en: "within_text", cn: "范围文本" },
    { en: "start_num", cn: "起始位置" },
  ],
  // ### 变更记录
  // - 2026-03-14 22:55: 原因=日期/时间函数语义明确; 目的=对齐业务输入
  DATE: [
    { en: "year", cn: "年" },
    { en: "month", cn: "月" },
    { en: "day", cn: "日" },
  ],
  TIME: [
    { en: "hour", cn: "时" },
    { en: "minute", cn: "分" },
    { en: "second", cn: "秒" },
  ],
  ROUND: [
    { en: "number", cn: "数值" },
    { en: "num_digits", cn: "小数位" },
  ],
  ROUNDUP: [
    { en: "number", cn: "数值" },
    { en: "num_digits", cn: "小数位" },
  ],
  ROUNDDOWN: [
    { en: "number", cn: "数值" },
    { en: "num_digits", cn: "小数位" },
  ],
  TEXT: [
    { en: "value", cn: "值" },
    { en: "format_text", cn: "格式" },
  ],
  VALUE: [{ en: "text", cn: "文本" }],
};

// ### 变更记录
// - 2026-03-14 22:55: 原因=常见函数需要明确用途; 目的=补足中文“用来做什么”描述
// - 2026-03-14 22:55: 原因=国际化要求; 目的=提供中英文用途
const functionPurposeOverrides = {
  SUM: { cn: "对范围内数值求和", en: "Sum numbers in a range" },
  AVERAGE: { cn: "计算范围内数值平均值", en: "Average values in a range" },
  COUNT: { cn: "统计范围内数值个数", en: "Count numeric values in a range" },
  IF: { cn: "按条件返回不同结果", en: "Return different results based on a condition" },
  VLOOKUP: { cn: "按匹配列查找并返回指定列值", en: "Lookup by key column and return a target column value" },
  XLOOKUP: { cn: "按匹配列查找并返回指定列值", en: "Lookup by key column and return a target column value" },
  INDEX: { cn: "返回范围内指定行列的值", en: "Return the value at a specific row/column" },
  MATCH: { cn: "返回查找值在范围内的位置", en: "Return position of a lookup value" },
  LEFT: { cn: "从左侧截取文本", en: "Extract text from the left" },
  RIGHT: { cn: "从右侧截取文本", en: "Extract text from the right" },
  MID: { cn: "从中间截取文本", en: "Extract text from the middle" },
  CONCAT: { cn: "拼接多个文本", en: "Concatenate multiple text values" },
  SPLIT: { cn: "按分隔符拆分文本", en: "Split text by a delimiter" },
  DATE: { cn: "生成日期值", en: "Create a date value" },
  TODAY: { cn: "返回当前日期", en: "Return current date" },
  NOW: { cn: "返回当前日期时间", en: "Return current date and time" },
  ROUND: { cn: "按指定位数四舍五入", en: "Round to a specified number of digits" },
};

// ### 变更记录
// - 2026-03-14 22:55: 原因=全量函数难以逐一维护; 目的=按名称模式提供用途说明
// - 2026-03-14 22:55: 原因=保证中文用途完整; 目的=提供中英文双语默认描述
const purposeRules = [
  { pattern: /LOOKUP|MATCH|INDEX/i, cn: "查找与引用数据", en: "Lookup and reference data" },
  { pattern: /SUM|AVERAGE|COUNT|MAX|MIN|MEDIAN|MODE|STDEV|VAR|PERCENTILE|QUARTILE|RANK|CORREL|COVAR/i, cn: "统计与汇总计算", en: "Statistical and aggregate calculations" },
  { pattern: /DATE|TIME|DAY|MONTH|YEAR|HOUR|MINUTE|SECOND|WEEK|TODAY|NOW/i, cn: "日期与时间处理", en: "Date and time handling" },
  { pattern: /TEXT|LEFT|RIGHT|MID|LEN|CONCAT|JOIN|SPLIT|SUBSTITUTE|REPLACE|TRIM|UPPER|LOWER|PROPER|FIND|SEARCH|VALUE/i, cn: "文本处理", en: "Text processing" },
  { pattern: /IF|AND|OR|NOT|XOR|SWITCH|CHOOSE|IFS/i, cn: "逻辑判断与条件处理", en: "Logical conditions" },
  { pattern: /ROUND|CEILING|FLOOR|INT|ABS|SIGN|SQRT|POWER|EXP|LN|LOG|LOG10|MOD|RAND|RANDBETWEEN/i, cn: "数学计算与取整", en: "Math and rounding" },
  { pattern: /SIN|COS|TAN|ASIN|ACOS|ATAN|COT|SEC|CSC|DEGREES|RADIANS|PI/i, cn: "三角函数与角度转换", en: "Trigonometry" },
  { pattern: /BESSEL|ERF|GAMMA|BETA|NORM|LOGNORM|POISSON|BINOM|CHISQ|F\.DIST|T\.DIST|WEIBULL|HYPGEOM|NEGBINOM|EXPON|PARETO|Z\.TEST|ZTEST/i, cn: "统计分布函数", en: "Statistical distributions" },
  { pattern: /PMT|NPV|IRR|RATE|PV|FV|NPER|SLN|SYD|DB|DDB|IPMT|PPMT|CUMIPMT|CUMPRINC/i, cn: "财务计算", en: "Financial calculations" },
  { pattern: /ARRAY|FILTER|SORT|UNIQUE|SEQUENCE|TRANSPOSE|MMULT|MDETERM|MINVERSE/i, cn: "数组与矩阵处理", en: "Array and matrix operations" },
  { pattern: /ERROR|ISERROR|ISNA|IFERROR|IFNA/i, cn: "错误检测与处理", en: "Error handling" },
  { pattern: /BIT|BIN|HEX|OCT|BASE|DECIMAL/i, cn: "进制与位运算", en: "Base and bitwise operations" },
];

// ### 变更记录
// - 2026-03-14 22:55: 原因=集中输出配置; 目的=供生成器复用
module.exports = {
  typeAliases,
  functionAliases,
  functionPurposeOverrides,
  purposeRules,
};
