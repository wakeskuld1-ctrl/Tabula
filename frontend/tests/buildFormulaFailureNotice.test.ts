// **[2026-03-14]** 变更原因：公式失败提示缺少可复现用例
// **[2026-03-14]** 变更目的：验证提示文案生成逻辑
// **[2026-03-14]** 变更原因：遵循 TDD 先红后绿
// **[2026-03-14]** 变更目的：确保实现前测试能失败
// **[2026-03-14]** 变更原因：无需引入完整测试框架
// **[2026-03-14]** 变更目的：使用 node + assert 保持轻量
import assert from "node:assert/strict";
import { buildFormulaFailureNotice } from "../src/utils/buildFormulaFailureNotice.js";

const columns = [
  { title: "A" },
  { title: "金额" },
  { title: "C" }
];

const msg1 = buildFormulaFailureNotice(0, 0, columns);
assert.equal(msg1, "单元格 A1 更新失败，请重试");

const msg2 = buildFormulaFailureNotice(1, 2, columns);
assert.equal(msg2, "单元格 金额3 更新失败，请重试");

const msg3 = buildFormulaFailureNotice(5, 9, []);
assert.equal(msg3, "单元格 F10 更新失败，请重试");

console.log("buildFormulaFailureNotice tests passed");
