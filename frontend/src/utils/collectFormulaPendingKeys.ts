import { GridCellKind, type EditListItem } from "@glideapps/glide-data-grid";

// **[2026-03-14]** 变更原因：公式回显等待态需要定位单元格
// **[2026-03-14]** 变更目的：提供可测试的 pending key 计算入口
// **[2026-03-14]** 变更原因：避免在 UI 逻辑中重复识别公式
// **[2026-03-14]** 变更目的：集中处理公式识别
// **[2026-03-14]** 变更原因：批量编辑可能跨多行
// **[2026-03-14]** 变更目的：确保每个公式单元都有 key
export const collectFormulaPendingKeys = (
  edits: readonly EditListItem[]
): Set<string> => {
  // **[2026-03-14]** 变更原因：返回纯 Set 便于去重
  // **[2026-03-14]** 变更目的：保证 pending 显示不重复
  const keys = new Set<string>();

  // **[2026-03-14]** 变更原因：批量编辑需逐条检测
  // **[2026-03-14]** 变更目的：仅对公式输入生成 key
  for (const edit of edits) {
    const row = edit.location[1];
    const col = edit.location[0];
    if (row < 0 || col < 0) continue;

    // **[2026-03-14]** 变更原因：仅 Text Cell 才可能是公式输入
    // **[2026-03-14]** 变更目的：排除非文本单元格干扰
    if (edit.value.kind !== GridCellKind.Text) continue;

    const raw = edit.value.data;
    const text = typeof raw === "string" ? raw : String(raw ?? "");

    // **[2026-03-14]** 变更原因：公式输入以“=”开头
    // **[2026-03-14]** 变更目的：避免普通文本触发等待态
    if (!text.trim().startsWith("=")) continue;

    keys.add(`${row},${col}`);
  }

  return keys;
};
