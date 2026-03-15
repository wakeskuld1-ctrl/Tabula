// ### 变更记录
// - 2026-03-15: 原因=结构化引用需支持列映射; 目的=TDD 先失败再实现
import { describe, expect, test } from "vitest";
import { shiftStructuredReferences } from "../formulaFill";

describe("shiftStructuredReferences", () => {
  // ### 变更记录
  // - 2026-03-15: 原因=列名需按 dx 映射; 目的=Table[Sales] -> Table[Profit]
  test("maps table column names by dx", () => {
    const columns = ["Sales", "Profit", "Cost"];
    const result = shiftStructuredReferences("=Table1[Sales]", 1, columns);
    expect(result).toBe("=Table1[Profit]");
  });

  // ### 变更记录
  // - 2026-03-15: 原因=@ 结构需保持; 目的=只替换列名
  test("maps current row references", () => {
    const columns = ["Sales", "Profit"];
    const result = shiftStructuredReferences("=[@Sales]", 1, columns);
    expect(result).toBe("=[@Profit]");
  });

  // ### 变更记录
  // - 2026-03-15: 原因=未知列名不应改写; 目的=保持原样
  test("keeps original when column missing", () => {
    const columns = ["Sales", "Profit"];
    const result = shiftStructuredReferences("=Table1[Unknown]", 1, columns);
    expect(result).toBe("=Table1[Unknown]");
  });
});
