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
});
