// ### Change Log
// - 2026-03-15: Reason=TDD for formula persistence; Purpose=protect raw formula storage
// - 2026-03-15: Reason=Formula display is decoupled; Purpose=prevent raw overwrite

import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Helper drives storage rules; Purpose=keep GlideGrid logic minimal
import { resolveFormulaStorage } from "../formulaPersistence";

// ### Change Log
// - 2026-03-15: Reason=Ensure raw formula preserved; Purpose=keep dependency inputs stable

describe("formula persistence", () => {
  // ### Change Log
  // - 2026-03-15: Reason=Formula should store raw; Purpose=display can differ from storage
  it("keeps raw formula while exposing display override", () => {
    const result = resolveFormulaStorage("=SUM(A:A)", "123");
    expect(result.storedValue).toBe("=SUM(A:A)");
    expect(result.displayValue).toBe("123");
  });

  // ### Change Log
  // - 2026-03-15: Reason=Normal input keeps same value; Purpose=avoid behavior change
  it("keeps normal input as stored and display", () => {
    const result = resolveFormulaStorage("42", "");
    expect(result.storedValue).toBe("42");
    expect(result.displayValue).toBe("42");
  });
});
