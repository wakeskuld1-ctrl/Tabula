// ### Change Log
// - 2026-03-16: Reason=Write guard needs a shared hook; Purpose=lock down block behavior.
// - 2026-03-16: Reason=TDD requirement; Purpose=fail before new helper exists.
import { describe, it, expect, vi } from "vitest";
// ### Change Log
// - 2026-03-16: Reason=Guard helper will live in utils; Purpose=ensure UI uses same logic.
import { guardWriteAction, READONLY_ALERT_MESSAGE } from "../sessionWriteGuard";

// ### Change Log
// - 2026-03-16: Reason=Readonly block must trigger alert; Purpose=protect users from silent failure.
describe("guardWriteAction", () => {
  // ### Change Log
  // - 2026-03-16: Reason=Blocked write must notify user; Purpose=ensure alert is fired.
  it("calls onBlocked with message when cannot write", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Need to observe blocking; Purpose=use a spy callback.
    const onBlocked = vi.fn();
    // ### Change Log
    // - 2026-03-16: Reason=Empty session should block; Purpose=simulate default session.
    const ok = guardWriteAction({ sessionId: "", isReadOnly: true, onBlocked });
    // ### Change Log
    // - 2026-03-16: Reason=Should be blocked; Purpose=return false when write is not allowed.
    expect(ok).toBe(false);
    // ### Change Log
    // - 2026-03-16: Reason=Alert message must be stable; Purpose=verify message is forwarded.
    expect(onBlocked).toHaveBeenCalledWith(READONLY_ALERT_MESSAGE);
  });
});
