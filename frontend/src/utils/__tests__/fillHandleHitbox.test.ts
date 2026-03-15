// ### 变更记录
// - 2026-03-15: 原因=填充把手命中规则需先锁定; 目的=TDD 先失败再实现
import { describe, expect, test } from "vitest";
import { isFillHandleHit } from "../fillHandle";

describe("isFillHandleHit", () => {
  // ### 变更记录
  // - 2026-03-15: 原因=命中范围需随行高缩放; 目的=验证动态把手
  test("hits within dynamic handle size", () => {
    const hit = isFillHandleHit({
      bounds: { x: 100, y: 100, width: 80, height: 40 },
      point: { x: 176, y: 136 },
      tolerance: 2
    });
    expect(hit).toBe(true);
  });

  // ### 变更记录
  // - 2026-03-15: 原因=误触发要避免; 目的=命中范围外返回 false
  test("misses outside handle region", () => {
    const hit = isFillHandleHit({
      bounds: { x: 100, y: 100, width: 80, height: 40 },
      point: { x: 160, y: 120 },
      tolerance: 2
    });
    expect(hit).toBe(false);
  });
});
