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

// **[2026-03-15]** 变更原因：相邻列选择需遵循左侧优先规则。
// **[2026-03-15]** 变更目的：保证双击填充行为与 Excel 一致。
export function chooseAdjacentColumnIndex(params: {
  selection: Rectangle | null | undefined;
  columnCount: number;
  hasDataAtColumn: (col: number) => boolean;
}): number | null {
  const { selection, columnCount, hasDataAtColumn } = params;
  if (!selection) return null;
  if (!Number.isFinite(columnCount) || columnCount <= 0) return null;
  const leftIndex = selection.x - 1;
  const rightIndex = selection.x + selection.width;
  if (leftIndex >= 0 && leftIndex < columnCount && hasDataAtColumn(leftIndex)) {
    return leftIndex;
  }
  if (rightIndex >= 0 && rightIndex < columnCount && hasDataAtColumn(rightIndex)) {
    return rightIndex;
  }
  return null;
}

// **[2026-03-15]** 变更原因：双击命中需随行高变化。
// **[2026-03-15]** 变更目的：避免缩放后误触或难以触发。
export function isFillHandleHit(params: {
  bounds: { x: number; y: number; width: number; height: number } | null | undefined;
  point: { x: number; y: number };
  tolerance?: number;
}): boolean {
  const { bounds, point, tolerance } = params;
  if (!bounds) return false;
  const safeTolerance = Number.isFinite(tolerance) ? Math.max(0, tolerance ?? 0) : 0;
  const handleSize = Math.min(10, Math.max(2, bounds.height * 0.25));
  const handleX = bounds.x + bounds.width - handleSize;
  const handleY = bounds.y + bounds.height - handleSize;
  const minX = handleX - safeTolerance;
  const maxX = handleX + handleSize + safeTolerance;
  const minY = handleY - safeTolerance;
  const maxY = handleY + handleSize + safeTolerance;
  return point.x >= minX && point.x <= maxX && point.y >= minY && point.y <= maxY;
}

// **[2026-03-15]** 变更原因：空值判定需统一。
// **[2026-03-15]** 变更目的：避免把空字符串当有效数据。
function isEmptyFillValue(value: unknown) {
  if (value === null || value === undefined) return true;
  const normalized = typeof value === "string" ? value : String(value);
  return normalized.trim().length === 0;
}
