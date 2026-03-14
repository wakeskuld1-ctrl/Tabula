// **[2026-03-14]** 变更原因：公式回显等待态缺少可复现用例
// **[2026-03-14]** 变更目的：用最小脚本验证 pending key 计算
// **[2026-03-14]** 变更原因：遵循 TDD 先红后绿
// **[2026-03-14]** 变更目的：确保实现前测试能失败
// **[2026-03-14]** 变更原因：无需引入完整测试框架
// **[2026-03-14]** 变更目的：使用 node + assert 保持轻量
import assert from "node:assert/strict";
import { GridCellKind, type Item, type TextCell } from "@glideapps/glide-data-grid";
import { collectFormulaPendingKeys } from "../src/utils/collectFormulaPendingKeys.js";

// **[2026-03-14]** 变更原因：构造最小 Text Cell 复用
// **[2026-03-14]** 变更目的：减少样板代码噪音
const makeTextCell = (data: string): TextCell => ({
  kind: GridCellKind.Text,
  data,
  displayData: data,
  allowOverlay: true,
  readonly: false
});

// **[2026-03-14]** 变更原因：覆盖跨行公式输入
// **[2026-03-14]** 变更目的：验证 key 组装逻辑
const edits = [
  { location: [0, 0] as Item, value: makeTextCell("=SUM(A1:A2)") },
  { location: [1, 150] as Item, value: makeTextCell("=A1+1") },
  { location: [2, 150] as Item, value: makeTextCell("=A1+2") }
];

const pendingKeys = collectFormulaPendingKeys(edits);
const list = Array.from(pendingKeys).sort();
assert.deepEqual(list, ["0,0", "150,1", "150,2"]);

// **[2026-03-14]** 变更原因：覆盖非公式输入
// **[2026-03-14]** 变更目的：确保不会触发等待态
const nonFormulaEdits = [
  { location: [0, 10] as Item, value: makeTextCell("123") }
];
const emptyKeys = collectFormulaPendingKeys(nonFormulaEdits);
assert.deepEqual(Array.from(emptyKeys), []);

console.log("collectFormulaPendingKeys tests passed");
