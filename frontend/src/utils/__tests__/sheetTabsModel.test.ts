// ### Change Log
// - 2026-03-15: Reason=TDD for sheet tab model; Purpose=fix add button placement safely
// - 2026-03-15: Reason=UI layout regression risk; Purpose=lock output ordering

import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Render list helper is new; Purpose=keep SheetBar simple
import { buildSheetTabItems } from "../sheetTabsModel";

// ### Change Log
// - 2026-03-15: Reason=Ensure add button appended; Purpose=keep plus at end

describe("sheet tabs model", () => {
  // ### Change Log
  // - 2026-03-15: Reason=Add button must be last; Purpose=keep UI placement predictable
  it("appends add button as last item", () => {
    const items = buildSheetTabItems([
      { sessionId: "s1", displayName: "Sheet1", isDefault: false }
    ]);
    expect(items[0].type).toBe("tab");
    expect(items[1].type).toBe("add");
  });
});
