// ### 变更记录
// - 2026-03-15: 原因=双击填充目标范围需 TDD 锁定; 目的=先失败再实现
import { describe, expect, test } from "vitest";
import { getAutoFillDestination } from "../fillHandle";

describe("getAutoFillDestination", () => {
  // ### 变更记录
  // - 2026-03-15: 原因=连续数据决定填充终点; 目的=按相邻列连续范围扩展
  test("expands down to contiguous adjacent data", () => {
    const selection = { x: 2, y: 5, width: 1, height: 1 };
    const values = ["A", "B", "C", ""];
    const result = getAutoFillDestination({
      selection,
      rowCount: 10,
      getAdjacentValue: (row) => values[row - 5] ?? ""
    });
    expect(result).toEqual({ x: 2, y: 5, width: 1, height: 3 });
  });

  // ### 变更记录
  // - 2026-03-15: 原因=无额外数据不应触发; 目的=避免无意义填充
  test("returns null when no extra rows", () => {
    const selection = { x: 1, y: 3, width: 1, height: 1 };
    const values = ["A"];
    const result = getAutoFillDestination({
      selection,
      rowCount: 10,
      getAdjacentValue: (row) => values[row - 3] ?? ""
    });
    expect(result).toBeNull();
  });

  // ### 变更记录
  // - 2026-03-15: 原因=起始位置为空不应触发; 目的=避免误填
  test("returns null when start row is empty", () => {
    const selection = { x: 4, y: 2, width: 2, height: 1 };
    const values = ["", "A"];
    const result = getAutoFillDestination({
      selection,
      rowCount: 10,
      getAdjacentValue: (row) => values[row - 2] ?? ""
    });
    expect(result).toBeNull();
  });

  // ### 变更记录
  // - 2026-03-15: 原因=目标不应超过最大行; 目的=保证范围安全
  test("clips to rowCount", () => {
    const selection = { x: 0, y: 8, width: 1, height: 1 };
    const values = ["A", "B", "C"];
    const result = getAutoFillDestination({
      selection,
      rowCount: 10,
      getAdjacentValue: (row) => values[row - 8] ?? ""
    });
    expect(result).toEqual({ x: 0, y: 8, width: 1, height: 2 });
  });
});
