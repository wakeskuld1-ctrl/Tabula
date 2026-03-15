// ### Change Log
// - 2026-03-15: Reason=TDD for debug overlay; Purpose=lock auto-hide trigger behavior
// - 2026-03-15: Reason=Loader auto-hide is new; Purpose=prevent regressions

import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Test helper lives in utils; Purpose=keep UI logic minimal
import { shouldAutoHideDebugInfo } from "../debugOverlay";

// ### Change Log
// - 2026-03-15: Reason=Test should describe intent; Purpose=make auto-hide rules explicit
// - 2026-03-15: Reason=Small focused cases; Purpose=avoid overfitting to UI

describe("debug overlay", () => {
  // ### Change Log
  // - 2026-03-15: Reason=Only hide completed load; Purpose=keep errors visible
  it("auto-hides only for loaded rows when not loading", () => {
    expect(shouldAutoHideDebugInfo("Loaded orders: 100 rows", false)).toBe(true);
    expect(shouldAutoHideDebugInfo("Loaded orders: 100 rows", true)).toBe(false);
    expect(shouldAutoHideDebugInfo("Fetch failed: 500", false)).toBe(false);
  });
});
