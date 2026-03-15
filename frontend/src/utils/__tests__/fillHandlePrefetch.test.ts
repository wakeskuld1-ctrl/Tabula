// ### 变更记录
// - 2026-03-15: 原因=未缓存补抓需要 TDD 锁定; 目的=先失败再实现
import { describe, expect, test } from "vitest";
import { buildPrefetchPlan } from "../fillHandle";

describe("buildPrefetchPlan", () => {
  // ### 变更记录
  // - 2026-03-15: 原因=补抓需受页数限制; 目的=避免过度请求
  test("limits pages by maxPages", () => {
    const plan = buildPrefetchPlan({
      startRow: 0,
      rowCount: 1000,
      pageSize: 100,
      maxPages: 3,
      maxRows: 1000
    });
    expect(plan.length).toBe(3);
  });

  // ### 变更记录
  // - 2026-03-15: 原因=补抓需受行数限制; 目的=避免无限扩展
  test("clips by maxRows", () => {
    const plan = buildPrefetchPlan({
      startRow: 50,
      rowCount: 1000,
      pageSize: 100,
      maxPages: 10,
      maxRows: 120
    });
    expect(plan).toEqual([1, 2]);
  });
});
