// **[2026-03-14]** 变更原因：批量公式回显缺少可复现用例
// **[2026-03-14]** 变更目的：用最小脚本复现“批量公式涉及页”逻辑
// **[2026-03-14]** 变更原因：遵循 TDD 先红后绿
// **[2026-03-14]** 变更目的：确保后续实现有明确失败起点
// **[2026-03-14]** 变更原因：无需引入完整测试框架
// **[2026-03-14]** 变更目的：使用 node + assert 保持测试轻量
import assert from "node:assert/strict";
import { GridCellKind, type Item, type TextCell } from "@glideapps/glide-data-grid";
// **[2026-03-14]** 变更原因：Node ESM 默认不解析无后缀模块
// **[2026-03-14]** 变更目的：让编译产物可直接被 node 执行
import { collectFormulaPages } from "../src/utils/collectFormulaPages.js";

// **[2026-03-14]** 变更原因：构造最小 Text Cell 复用
// **[2026-03-14]** 变更目的：减少样板代码噪音
const makeTextCell = (data: string): TextCell => ({
  kind: GridCellKind.Text,
  data,
  displayData: data,
  allowOverlay: true,
  readonly: false
});

// **[2026-03-14]** 变更原因：覆盖跨页公式输入
// **[2026-03-14]** 变更目的：验证页集合去重逻辑
// **[2026-03-14]** 变更原因：Item 需要固定为 2 元组
// **[2026-03-14]** 变更目的：让测试编译通过并符合类型约束
const edits = [
  { location: [0, 0] as Item, value: makeTextCell("=SUM(A1:A2)") },
  { location: [1, 150] as Item, value: makeTextCell("=A1+1") },
  { location: [2, 150] as Item, value: makeTextCell("=A1+2") }
];

const pages = collectFormulaPages(edits, 100);
const pageList = Array.from(pages).sort((a, b) => a - b);
assert.deepEqual(pageList, [1, 2]);

// **[2026-03-14]** 变更原因：覆盖非公式输入
// **[2026-03-14]** 变更目的：确保不会触发刷新
const nonFormulaEdits = [
  { location: [0, 10] as Item, value: makeTextCell("123") }
];
const emptyPages = collectFormulaPages(nonFormulaEdits, 100);
assert.deepEqual(Array.from(emptyPages), []);

console.log("collectFormulaPages tests passed");
