// ### 变更记录
// - 2026-03-15 00:10: 原因=新增公式帮助过滤测试; 目的=遵循TDD先失败
// - 2026-03-15 00:10: 原因=后续扩展查询逻辑; 目的=建立稳定断言基线
// - 2026-03-15 00:10: 原因=国际化要求; 目的=覆盖中英文关键字过滤

import { describe, it, expect } from "vitest";
import { filterFormulaHelpItems } from "../formulaHelp";

// ### 变更记录
// - 2026-03-15 00:10: 原因=测试需要固定样本; 目的=覆盖函数名/用途/参数说明字段
// - 2026-03-15 00:10: 原因=减少依赖真实数据; 目的=保证测试稳定
const sampleItems = [
  {
    name: "SUM",
    syntax: "SUM(range)",
    example: "=SUM(A1:A5)",
    paramNotes: "range/范围",
    purpose: "统计与汇总计算 / Statistical",
    note: "—",
  },
  {
    name: "VLOOKUP",
    syntax: "VLOOKUP(lookup_value, table)",
    example: "=VLOOKUP(A1, table)",
    paramNotes: "lookup_value/查找值",
    purpose: "查找与引用数据 / Lookup",
    note: "—",
  },
];

// ### 变更记录
// - 2026-03-15 00:10: 原因=验证空查询行为; 目的=确保默认返回全部
// - 2026-03-15 00:10: 原因=验证字段过滤; 目的=覆盖名称与用途检索
describe("filterFormulaHelpItems", () => {
  it("returns all items when query is empty", () => {
    expect(filterFormulaHelpItems(sampleItems, "").length).toBe(2);
  });

  it("filters by name", () => {
    expect(filterFormulaHelpItems(sampleItems, "sum")[0].name).toBe("SUM");
  });

  it("filters by purpose", () => {
    expect(filterFormulaHelpItems(sampleItems, "查找")[0].name).toBe("VLOOKUP");
  });
});
