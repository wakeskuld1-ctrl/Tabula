// ### Change Log
// - 2026-03-15: Reason=TDD for system table filtering; Purpose=prevent selecting invalid system tables

import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Keep table filtering centralized; Purpose=lock denylist behavior
import { filterUserVisibleTables } from "../tableList";

// ### Change Log
// - 2026-03-15: Reason=sys_metadata causes backend errors; Purpose=hide from selection list
describe("filterUserVisibleTables", () => {
  // ### Change Log
  // - 2026-03-15: Reason=System table should be hidden; Purpose=avoid user selecting invalid table
  it("filters sys_metadata from table list", () => {
    const result = filterUserVisibleTables(["users", "sys_metadata", "orders"]);
    expect(result).toEqual(["users", "orders"]);
  });
});
