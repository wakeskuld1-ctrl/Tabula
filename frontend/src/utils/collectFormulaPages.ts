import { GridCellKind, type EditListItem } from "@glideapps/glide-data-grid";

// **[2026-03-14]** 变更原因：批量公式回显依赖涉及页集合
// **[2026-03-14]** 变更目的：提供统一、可测试的页集合计算入口
// **[2026-03-14]** 变更原因：避免在 UI 逻辑中重复识别公式
// **[2026-03-14]** 变更目的：降低 onCellsEdited 复杂度
// **[2026-03-14]** 变更原因：批量下拉可能跨页
// **[2026-03-14]** 变更目的：去重刷新页，避免重复请求
export const collectFormulaPages = (
  edits: readonly EditListItem[],
  pageSize: number
): Set<number> => {
  // **[2026-03-14]** 变更原因：确保结果可复用且不依赖外部状态
  // **[2026-03-14]** 变更目的：返回纯数据结构，便于测试
  const pages = new Set<number>();

  // **[2026-03-14]** 变更原因：批量编辑需要逐条检测
  // **[2026-03-14]** 变更目的：只对公式输入生成刷新页
  for (const edit of edits) {
    const row = edit.location[1];
    if (row < 0) continue;

    // **[2026-03-14]** 变更原因：仅 Text Cell 才可能是公式输入
    // **[2026-03-14]** 变更目的：排除非文本单元格干扰
    if (edit.value.kind !== GridCellKind.Text) continue;

    const raw = edit.value.data;
    const text = typeof raw === "string" ? raw : String(raw ?? "");

    // **[2026-03-14]** 变更原因：仅对“= 开头”视作公式
    // **[2026-03-14]** 变更目的：避免普通文本触发刷新
    if (!text.trim().startsWith("=")) continue;

    pages.add(Math.floor(row / pageSize) + 1);
  }

  return pages;
};
