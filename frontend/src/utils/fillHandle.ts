import type { Rectangle } from "@glideapps/glide-data-grid";

// **[2026-03-15]** 变更原因：双击填充需要统一目标范围计算。
// **[2026-03-15]** 变更目的：复用相邻列连续区间作为终点。
export function getAutoFillDestination(params: {
  selection: Rectangle | null | undefined;
  rowCount: number;
  getAdjacentValue: (row: number) => unknown;
}): Rectangle | null {
  const { selection, rowCount, getAdjacentValue } = params;
  if (!selection) return null;
  if (!Number.isFinite(rowCount) || rowCount <= 0) return null;
  const { x, y, width, height } = selection;
  if (width <= 0 || height <= 0) return null;
  if (y < 0 || y >= rowCount) return null;
  if (isEmptyFillValue(getAdjacentValue(y))) return null;
  let lastRow = y;
  for (let row = y + 1; row < rowCount; row += 1) {
    if (isEmptyFillValue(getAdjacentValue(row))) break;
    lastRow = row;
  }
  if (lastRow <= y + height - 1) return null;
  return { x, y, width, height: lastRow - y + 1 };
}

// **[2026-03-15]** 变更原因：空值判定需统一。
// **[2026-03-15]** 变更目的：避免把空字符串当有效数据。
function isEmptyFillValue(value: unknown) {
  if (value === null || value === undefined) return true;
  const normalized = typeof value === "string" ? value : String(value);
  return normalized.trim().length === 0;
}
