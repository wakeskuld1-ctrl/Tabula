// ### 变更记录
// - 2026-03-15: 原因=双击填充目标范围需 TDD 锁定; 目的=先失败再实现
import { describe, expect, test } from "vitest";
import { getAutoFillDestination, chooseAdjacentColumnIndex } from "../fillHandle";

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

describe("chooseAdjacentColumnIndex", () => {
  // ### 变更记录
  // - 2026-03-15: 原因=左侧优先规则需锁定; 目的=避免后续回归
  test("prefers left column when data exists", () => {
    const selection = { x: 2, y: 5, width: 1, height: 1 };
    const result = chooseAdjacentColumnIndex({
      selection,
      columnCount: 6,
      hasDataAtColumn: (col) => col === 1
    });
    expect(result).toBe(1);
  });

  // ### 变更记录
  // - 2026-03-15: 原因=左侧无数据时需回退右侧; 目的=保持可用性
  test("falls back to right column when left empty", () => {
    const selection = { x: 2, y: 5, width: 1, height: 1 };
    const result = chooseAdjacentColumnIndex({
      selection,
      columnCount: 6,
      hasDataAtColumn: (col) => col === 3
    });
    expect(result).toBe(3);
  });

  // ### 变更记录
  // - 2026-03-15: 原因=左右都无数据不应触发; 目的=避免误填
  test("returns null when no adjacent data", () => {
    const selection = { x: 2, y: 5, width: 1, height: 1 };
    const result = chooseAdjacentColumnIndex({
      selection,
      columnCount: 6,
      hasDataAtColumn: () => false
    });
    expect(result).toBeNull();
  });
});
