// ### Change Log
// - 2026-03-15: Reason=System tables can be invalid; Purpose=centralize table list filtering

// ### Change Log
// - 2026-03-15: Reason=Backend sys_metadata is not selectable; Purpose=prevent user selection
const SYSTEM_TABLE_DENYLIST = new Set(["sys_metadata"]);

// ### Change Log
// - 2026-03-15: Reason=Filter system tables; Purpose=return user-visible tables only
export const filterUserVisibleTables = (tables: string[]): string[] => {
  // ### Change Log
  // - 2026-03-15: Reason=Defensive copy; Purpose=avoid caller mutation surprises
  return tables.filter((name) => !SYSTEM_TABLE_DENYLIST.has(name));
};
