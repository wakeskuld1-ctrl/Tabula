// ### 变更记录
// - 2026-03-15: 原因=新增解析位移能力需先锁定行为; 目的=确保整列引用可位移
// - 2026-03-15: 原因=TDD 红灯覆盖; 目的=先失败再实现
import { describe, expect, test } from "vitest";
// ### 变更记录
// - 2026-03-15: 原因=明确测试入口; 目的=覆盖解析位移函数
import { shiftFormulaReferencesWithParser } from "../formulaFill";

describe("shiftFormulaReferencesWithParser", () => {
  // ### 变更记录
  // - 2026-03-15: 原因=整列引用目前无法位移; 目的=锁定目标行为
  test("shifts whole-column references", () => {
    // ### 变更记录
    // - 2026-03-15: 原因=填充向右 1 列; 目的=验证 F:F -> G:G
    const result = shiftFormulaReferencesWithParser("=SUM(F:F)", 1, 0);
    // ### 变更记录
    // - 2026-03-15: 原因=明确期望; 目的=保证位移正确
    expect(result).toBe("=SUM(G:G)");
  });

  // ### 变更记录
  // - 2026-03-15: 原因=绝对列不应位移; 目的=避免破坏锁定引用
  test("respects absolute column", () => {
    // ### 变更记录
    // - 2026-03-15: 原因=列绝对锁定; 目的=验证位移无效
    const result = shiftFormulaReferencesWithParser("=$F1", 1, 0);
    // ### 变更记录
    // - 2026-03-15: 原因=锁定列不变; 目的=保持原始引用
    expect(result).toBe("=$F1");
  });

  // ### 变更记录
  // - 2026-03-15: 原因=相对行需位移; 目的=保证纵向填充正确
  test("shifts relative row", () => {
    // ### 变更记录
    // - 2026-03-15: 原因=向下填充 2 行; 目的=验证 A1 -> A3
    const result = shiftFormulaReferencesWithParser("=A1", 0, 2);
    // ### 变更记录
    // - 2026-03-15: 原因=行位移生效; 目的=保持相对引用一致
    expect(result).toBe("=A3");
  });

  // ### 变更记录
  // - 2026-03-15: 原因=整行引用需位移; 目的=覆盖 3:3 -> 5:5
  test("shifts whole-row references", () => {
    // ### 变更记录
    // - 2026-03-15: 原因=向下填充 2 行; 目的=验证整行位移
    const result = shiftFormulaReferencesWithParser("=SUM(3:3)", 0, 2);
    // ### 变更记录
    // - 2026-03-15: 原因=整行位移生效; 目的=保持序列一致
    expect(result).toBe("=SUM(5:5)");
  });
});
