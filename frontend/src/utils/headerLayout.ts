// ### Change Log
// - 2026-03-15: Reason=Single-row header needs stable brand title; Purpose=centralize naming
// - 2026-03-15: Reason=TDD helper module; Purpose=keep layout rules testable

export type HeaderGroups = {
  // ### Change Log
  // - 2026-03-15: Reason=Left cluster contains primary controls; Purpose=order brand/table/pivot
  left: string[];
  // ### Change Log
  // - 2026-03-15: Reason=Right cluster contains status; Purpose=order label/chip/debug
  right: string[];
};

// ### Change Log
// - 2026-03-15: Reason=Brand rename to Tabula; Purpose=avoid duplicated literals
export function getBrandTitle() {
  return "Tabula";
}

// ### Change Log
// - 2026-03-15: Reason=Single-row layout needs grouping; Purpose=keep layout intent explicit
export function getHeaderGroups(): HeaderGroups {
  // ### Change Log
  // - 2026-03-15: Reason=Left side should keep selector + pivot together; Purpose=match UI requirement
  const left = ["brand", "table-selector", "pivot"];
  // ### Change Log
  // - 2026-03-15: Reason=Right side shows status info; Purpose=consistent ordering
  const right = ["status-label", "status-chip", "status-debug"];
  return { left, right };
}
