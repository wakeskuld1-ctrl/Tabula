// ### Change Log
// - 2026-03-15: Reason=Header needs single-row layout; Purpose=define testable helpers for branding and grouping
// - 2026-03-15: Reason=TDD requirement; Purpose=encode expectations before implementation

import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Test new helpers; Purpose=verify brand title and layout grouping
import { getBrandTitle, getHeaderGroups } from "../headerLayout";

// ### Change Log
// - 2026-03-15: Reason=Validate brand rename; Purpose=avoid regression to old name
// - 2026-03-15: Reason=Keep tests minimal; Purpose=single assertion per behavior
describe("header layout", () => {
  it("uses Tabula brand title", () => {
    expect(getBrandTitle()).toBe("Tabula");
  });

  it("groups table selector with pivot", () => {
    const groups = getHeaderGroups();
    expect(groups.left.includes("table-selector")).toBe(true);
    expect(groups.left.includes("pivot")).toBe(true);
  });
});
