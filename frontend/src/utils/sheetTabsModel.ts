// ### Change Log
// - 2026-03-15: Reason=Sheet add button must sit with tabs; Purpose=provide stable render model
// - 2026-03-15: Reason=Keep SheetBar simple; Purpose=centralize item ordering

export type SheetTabInput = {
  // ### Change Log
  // - 2026-03-15: Reason=Input mirrors session payload; Purpose=avoid extra typing in callers
  sessionId: string;
  displayName: string;
  isDefault: boolean;
};

export type SheetTabModel = SheetTabInput & {
  // ### Change Log
  // - 2026-03-15: Reason=Tab item needs stable shape; Purpose=render list safely
  type: "tab";
};

export type SheetAddModel = {
  // ### Change Log
  // - 2026-03-15: Reason=Add item is a sentinel; Purpose=render plus button inline
  type: "add";
};

// ### Change Log
// - 2026-03-15: Reason=Append add button after tabs; Purpose=keep plus next to last tab
// - 2026-03-15: Reason=Model should be deterministic; Purpose=avoid layout surprises
export const buildSheetTabItems = (sessions: SheetTabInput[]): Array<SheetTabModel | SheetAddModel> => {
  // ### Change Log
  // - 2026-03-15: Reason=Copy to avoid mutation; Purpose=preserve caller data
  // ### Change Log
  // - 2026-03-15: Reason=Union needs explicit type; Purpose=allow add sentinel without TS error
  const items: Array<SheetTabModel | SheetAddModel> = sessions.map(session => ({
    ...session,
    type: "tab" as const
  }));
  // ### Change Log
  // - 2026-03-15: Reason=Always include add entry; Purpose=consistent UI affordance
  items.push({ type: "add" });
  return items;
};
