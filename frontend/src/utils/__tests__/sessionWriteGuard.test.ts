// ### Change Log
// - 2026-03-16: Reason=Readonly sessions must block writes; Purpose=lock down guard behavior with tests.
// - 2026-03-16: Reason=User requested alert-first experience; Purpose=ensure message stays consistent.
// - 2026-03-16: Reason=TDD requirement; Purpose=fail before implementation.
import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-16: Reason=Guard will live in utils; Purpose=decouple UI logic from guard evaluation.
import { getWriteGuardState, READONLY_ALERT_MESSAGE } from "../sessionWriteGuard";

// ### Change Log
// - 2026-03-16: Reason=Readonly sessions must block any write intent; Purpose=verify canWrite=false.
// - 2026-03-16: Reason=Alert text must be stable; Purpose=ensure UX copy does not drift.
describe("sessionWriteGuard", () => {
  // ### Change Log
  // - 2026-03-16: Reason=sessionId empty is default session; Purpose=block all writes until new sheet.
  it("blocks write when sessionId is empty", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Empty sessionId should be treated as readonly; Purpose=consistent UX.
    const state = getWriteGuardState({ sessionId: "", isReadOnly: false });
    // ### Change Log
    // - 2026-03-16: Reason=Guard must block writes; Purpose=prevent accidental edits.
    expect(state.canWrite).toBe(false);
    // ### Change Log
    // - 2026-03-16: Reason=Alert copy must be stable; Purpose=avoid UX mismatch.
    expect(state.message).toBe(READONLY_ALERT_MESSAGE);
  });

  // ### Change Log
  // - 2026-03-16: Reason=Explicit readonly should always block; Purpose=protect default session.
  it("blocks write when readonly", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Readonly flag overrides sessionId presence; Purpose=keep rules strict.
    const state = getWriteGuardState({ sessionId: "s1", isReadOnly: true });
    // ### Change Log
    // - 2026-03-16: Reason=Guard must deny write; Purpose=prevent data loss.
    expect(state.canWrite).toBe(false);
    // ### Change Log
    // - 2026-03-16: Reason=Alert copy must be stable; Purpose=consistent messaging.
    expect(state.message).toBe(READONLY_ALERT_MESSAGE);
  });

  // ### Change Log
  // - 2026-03-16: Reason=Writable sessions should pass; Purpose=avoid blocking normal edits.
  it("allows write when sessionId exists and not readonly", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Valid session + writable flag; Purpose=should allow write.
    const state = getWriteGuardState({ sessionId: "s1", isReadOnly: false });
    // ### Change Log
    // - 2026-03-16: Reason=Writable sessions are allowed; Purpose=enable edits.
    expect(state.canWrite).toBe(true);
    // ### Change Log
    // - 2026-03-16: Reason=No message needed when writable; Purpose=avoid noise.
    expect(state.message).toBe("");
  });
});
