// ### 变更记录
// - 2026-02-15: 原因=增加公式解析测试; 目的=覆盖列范围与单元格范围
// - 2026-02-15: 原因=先写失败用例; 目的=符合“先测后改”流程
const assert = require("assert");

(async () => {
  // ### 变更记录
  // - 2026-02-15: 原因=使用 ESM 动态导入; 目的=兼容 type=module
  // - 2026-02-15: 原因=复用解析工具; 目的=测试与实现对齐
  const {
    parseAggregateFormula,
    getRangeInfo,
    buildFormulaColumnSql,
    buildFormulaColumnMarker,
    isFormulaColumnIndex,
    getFormulaColumnDisplayValue,
    validateFormulaColumnName,
    formatCellValue,
    normalizeArithmeticFormula,
    extractArithmeticFormulaColumns,
    getArithmeticFormulaColumnIndexes,
    // **[2026-02-17]** 变更原因：新增聚合函数提示测试依赖。
    // **[2026-02-17]** 变更目的：保证弹窗展示与检测一致。
    getAggregateFunctionNames,
    // **[2026-02-17]** 变更原因：新增聚合函数检测测试依赖。
    // **[2026-02-17]** 变更目的：覆盖大小写输入行为。
    isAggregateFormulaFunction
  } = await import(
    "../src/utils/formulaRange.js"
  );
  const { shiftFormulaReferences, inferFillValues } = await import(
    "../src/utils/formulaFill.js"
  );
  const { collectMergesFromCachePages } = await import(
    "../src/utils/merge.js"
  );

  // ### 变更记录
  // - 2026-02-15: 原因=验证整列范围解析; 目的=支持 SUM(A:B)
  // - 2026-02-15: 原因=覆盖无行号场景; 目的=避免错误判定
  const parsedColumn = parseAggregateFormula("=SUM(A:B)");
  assert.ok(parsedColumn);
  assert.strictEqual(parsedColumn.func, "SUM");
  assert.strictEqual(parsedColumn.startCol, "A");
  assert.strictEqual(parsedColumn.endCol, "B");
  assert.strictEqual(parsedColumn.startRow, null);
  assert.strictEqual(parsedColumn.endRow, null);

  // ### 变更记录
  // - 2026-02-15: 原因=验证列范围输出; 目的=确保 columns 与 cellCount 逻辑
  // - 2026-02-15: 原因=覆盖 null cellCount; 目的=区分整列范围
  const columnInfo = getRangeInfo(parsedColumn);
  assert.strictEqual(columnInfo.type, "column");
  assert.deepStrictEqual(columnInfo.columns, ["A", "B"]);
  assert.strictEqual(columnInfo.cellCount, null);

  // ### 变更记录
  // - 2026-02-15: 原因=验证指定行范围解析; 目的=支持 COUNT(A1:A5)
  // - 2026-02-15: 原因=覆盖行号解析; 目的=保证边界正确
  const parsedCell = parseAggregateFormula("=COUNT(A1:A5)");
  assert.ok(parsedCell);
  assert.strictEqual(parsedCell.func, "COUNT");
  assert.strictEqual(parsedCell.startCol, "A");
  assert.strictEqual(parsedCell.endCol, "A");
  assert.strictEqual(parsedCell.startRow, 1);
  assert.strictEqual(parsedCell.endRow, 5);

  // ### 变更记录
  // - 2026-02-15: 原因=验证单元格范围输出; 目的=确保 cellCount 计算
  // - 2026-02-15: 原因=覆盖列列表; 目的=确保依赖列准确
  const cellInfo = getRangeInfo(parsedCell);
  assert.strictEqual(cellInfo.type, "cell");
  assert.deepStrictEqual(cellInfo.columns, ["A"]);
  assert.strictEqual(cellInfo.cellCount, 5);

  // ### 变更记录
  // - 2026-02-15: 原因=验证不完整范围; 目的=阻止 A1:B 误判
  // - 2026-02-15: 原因=确保返回 null; 目的=防止产生错误元信息
  const invalid = parseAggregateFormula("=SUM(A1:B)");
  assert.strictEqual(invalid, null);

  // ### 变更记录
  // - 2026-02-16: 原因=新增公式列 SQL 生成测试; 目的=保证列映射正确
  // - 2026-02-16: 原因=覆盖基础算术表达式; 目的=确保列名替换安全
  const sql = buildFormulaColumnSql("B*C", ["id", "price", "qty"]);
  assert.strictEqual(sql, "\"price\"*\"qty\"");

  // ### 变更记录
  // - 2026-02-16: 原因=覆盖越界列; 目的=确保非法输入返回 null
  // - 2026-02-16: 原因=防止错误映射; 目的=保证错误可见
  const invalidSql = buildFormulaColumnSql("Z+1", ["id"]);
  assert.strictEqual(invalidSql, null);

  // ### 变更记录
  // - 2026-02-16: 原因=新增公式列 marker 构造测试; 目的=确保 raw 与 sql 同步
  // - 2026-02-16: 原因=验证 marker 结构; 目的=对齐后端解析字段
  const marker = buildFormulaColumnMarker("B*C", ["id", "price", "qty"]);
  assert.ok(marker);
  assert.strictEqual(marker.kind, "formula");
  assert.strictEqual(marker.raw, "B*C");
  assert.strictEqual(marker.sql, "\"price\"*\"qty\"");

  // ### 变更记录
  // - 2026-02-16: 原因=覆盖非法 marker; 目的=保证异常输入返回 null
  // - 2026-02-16: 原因=避免空公式写入; 目的=降低错误入库
  const invalidMarker = buildFormulaColumnMarker("Z+1", ["id"]);
  assert.strictEqual(invalidMarker, null);

  // ### 变更记录
  // - 2026-02-16: 原因=新增公式列索引判断; 目的=前端只读控制
  // - 2026-02-16: 原因=覆盖非公式列; 目的=避免误判
  const metaList = [
    { index: 1, raw_expression: "B*C", sql_expression: "\"price\"*\"qty\"", name: "calc" }
  ];
  assert.strictEqual(isFormulaColumnIndex(1, metaList), true);
  assert.strictEqual(isFormulaColumnIndex(0, metaList), false);

  // ### 变更记录
  // - 2026-02-16: 原因=公式栏显示需使用 raw; 目的=选中公式列展示原始表达式
  // - 2026-02-16: 原因=覆盖非公式列; 目的=回退显示原始值
  assert.strictEqual(
    getFormulaColumnDisplayValue(1, metaList, "123"),
    "B*C"
  );
  assert.strictEqual(
    getFormulaColumnDisplayValue(0, metaList, "123"),
    "123"
  );

  // ### 变更记录
  // - 2026-02-16: 原因=公式列列名校验; 目的=阻止空列名插入
  // - 2026-02-16: 原因=裁剪空白; 目的=保证列名一致
  assert.strictEqual(validateFormulaColumnName(""), null);
  assert.strictEqual(validateFormulaColumnName("   "), null);
  assert.strictEqual(validateFormulaColumnName(" 公式列 "), "公式列");

  // ### 变更记录
  // - 2026-02-16: 原因=新增单元格格式化; 目的=验证数值格式输出
  // - 2026-02-16: 原因=覆盖格式化兜底; 目的=避免 NaN 展示
  assert.strictEqual(formatCellValue("1234", "number"), "1,234");
  assert.strictEqual(formatCellValue("foo", "number"), "foo");

  // ### 变更记录
  // - 2026-02-16: 原因=新增百分比显示; 目的=符合格式化需求
  // - 2026-02-16: 原因=覆盖非法输入; 目的=保持原值
  assert.strictEqual(formatCellValue("0.12", "percent"), "12%");
  assert.strictEqual(formatCellValue("bar", "percent"), "bar");

  // ### 变更记录
  // - 2026-02-16: 原因=新增货币显示; 目的=保持两位小数
  // - 2026-02-16: 原因=使用 CNY; 目的=与本地默认一致
  assert.strictEqual(formatCellValue("56.7", "currency"), "¥56.70");

  // ### 变更记录
  // - 2026-02-16: 原因=新增日期显示; 目的=验证日期格式化
  // - 2026-02-16: 原因=覆盖无效日期; 目的=回退原值
  assert.strictEqual(formatCellValue("2025-01-02", "date"), "2025-01-02");
  assert.strictEqual(formatCellValue("not-date", "date"), "not-date");

  // ### 变更记录
  // - 2026-02-17: 原因=新增算术公式标准化测试; 目的=确保列名统一大写
  // - 2026-02-17: 原因=覆盖空白清理; 目的=保证表达式一致性
  assert.strictEqual(normalizeArithmeticFormula(" b + c "), "B+C");

  // ### 变更记录
  // - 2026-02-17: 原因=非算术表达式不应标准化; 目的=跳过 IF 类公式
  // - 2026-02-17: 原因=包含比较符不支持; 目的=避免误判为算术
  assert.strictEqual(normalizeArithmeticFormula("IF(A>0,1,0)"), null);

  // ### 变更记录
  // - 2026-02-17: 原因=新增算术列提取测试; 目的=用于数值型校验
  // - 2026-02-17: 原因=覆盖去重与顺序; 目的=错误提示稳定
  assert.deepStrictEqual(
    extractArithmeticFormulaColumns("A+B*C+A"),
    ["A", "B", "C"]
  );

  // ### 变更记录
  // - 2026-02-17: 原因=新增列索引映射测试; 目的=支持类型判断
  // - 2026-02-17: 原因=覆盖越界列; 目的=保证非法表达式拦截
  assert.deepStrictEqual(
    getArithmeticFormulaColumnIndexes("A+C", 3),
    [0, 2]
  );
  assert.strictEqual(
    getArithmeticFormulaColumnIndexes("Z+1", 2),
    null
  );

  // ### 变更记录
  // - 2026-02-17: 原因=聚合函数提示需要名单; 目的=避免文案遗漏
  // - 2026-02-17: 原因=聚合检测依赖名单; 目的=校验入口一致
  assert.deepStrictEqual(
    getAggregateFunctionNames(),
    ["SUM", "COUNT", "COUNTA", "AVG", "AVERAGE", "MAX", "MIN"]
  );
  // ### 变更记录
  // - 2026-02-17: 原因=聚合函数检测; 目的=覆盖大小写输入
  // - 2026-02-17: 原因=非聚合应返回 false; 目的=避免误判
  assert.strictEqual(isAggregateFormulaFunction("SUM"), true);
  assert.strictEqual(isAggregateFormulaFunction("sum"), true);
  assert.strictEqual(isAggregateFormulaFunction("IF"), false);

  // **[2026-02-17]** 变更原因：覆盖小写列名公式。
  // **[2026-02-17]** 变更目的：确保列名大小写不影响位移。
  assert.strictEqual(
    shiftFormulaReferences("=A1+$B$2+$C3+D$4", 1, 2),
    "=B3+$B$2+$C5+E$4"
  );
  // **[2026-02-17]** 变更原因：新增小写列名测试。
  // **[2026-02-17]** 变更目的：验证输出统一为大写列名。
  assert.strictEqual(
    shiftFormulaReferences("=a1+b2", 1, 0),
    "=B1+C2"
  );
  // **[2026-02-17]** 变更原因：验证数值递增序列。
  // **[2026-02-17]** 变更目的：保持基础步长推断一致。
  assert.deepStrictEqual(
    inferFillValues(["1", "3"], 4),
    ["1", "3", "5", "7"]
  );
  // **[2026-02-17]** 变更原因：覆盖负数步长序列。
  // **[2026-02-17]** 变更目的：确保递减序列正确扩展。
  assert.deepStrictEqual(
    inferFillValues(["1", "-1"], 4),
    ["1", "-1", "-3", "-5"]
  );
  // **[2026-02-17]** 变更原因：覆盖小数步长序列。
  // **[2026-02-17]** 变更目的：确保小数计算保持精度。
  assert.deepStrictEqual(
    inferFillValues(["1.5", "2.0"], 4),
    ["1.5", "2", "2.5", "3"]
  );
  // **[2026-02-17]** 变更原因：覆盖跨月/闰年日期序列。
  // **[2026-02-17]** 变更目的：验证日期步长稳定性。
  assert.deepStrictEqual(
    inferFillValues(["2024-02-28", "2024-02-29"], 4),
    ["2024-02-28", "2024-02-29", "2024-03-01", "2024-03-02"]
  );
  // **[2026-02-17]** 变更原因：文本前缀不一致时不应推断。
  // **[2026-02-17]** 变更目的：回退为重复首值策略。
  assert.deepStrictEqual(
    inferFillValues(["A01", "B02"], 4),
    ["A01", "B02", "A01", "A01"]
  );

  // ### 变更记录
  // - 2026-03-12: 原因=复现“合并单元格不可用”中的跨行合并丢失问题; 目的=确保收集逻辑不再只保留单行合并
  // - 2026-03-12: 原因=覆盖字符串数值场景; 目的=统一后续索引构建输入类型
  const collectedMerges = collectMergesFromCachePages([
    {
      metadata: {
        merges: [
          { start_row: 1, start_col: 0, end_row: 3, end_col: 1 },
          { start_row: "5", start_col: "2", end_row: "5", end_col: "4" }
        ]
      }
    }
  ]);
  assert.deepStrictEqual(collectedMerges, [
    { start_row: 1, start_col: 0, end_row: 3, end_col: 1 },
    { start_row: 5, start_col: 2, end_row: 5, end_col: 4 }
  ]);

  // ### 变更记录
  // - 2026-02-15: 原因=标记测试完成; 目的=便于脚本判断
  // - 2026-02-15: 原因=输出一致; 目的=可读性提升
  console.log("formula_range.test.cjs passed");
})();
