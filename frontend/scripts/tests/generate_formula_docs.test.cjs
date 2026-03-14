// ### 变更记录
// - 2026-03-14 21:30: 原因=新增公式全量文档生成测试; 目的=遵循TDD先失败
// - 2026-03-14 21:30: 原因=后续扩展需要结构化测试; 目的=采用node:test组织
// - 2026-03-14 21:30: 原因=保持可维护性; 目的=先验证生成器入口存在

const test = require("node:test");
const assert = require("node:assert/strict");

// ### 变更记录
// - 2026-03-14 21:30: 原因=确认生成器模块存在; 目的=为README注入建立最小保障
// - 2026-03-14 21:30: 原因=预留扩展空间; 目的=确保失败信息清晰可见
// - 2026-03-14 21:30: 原因=避免隐式失败; 目的=显式断言模块可加载

test("generator module should exist", () => {
  // ### 变更记录
  // - 2026-03-14 21:30: 原因=模块可能暂未创建; 目的=确保用例按TDD失败
  // - 2026-03-14 21:30: 原因=捕获require异常; 目的=输出稳定失败断言
  let generator = null;
  try {
    generator = require("../generate_formula_docs.cjs");
  } catch (error) {
    generator = null;
  }

  // ### 变更记录
  // - 2026-03-14 21:30: 原因=测试目标是入口存在; 目的=阻止无生成器继续后续流程
  assert.ok(generator, "generator module should exist");
});
