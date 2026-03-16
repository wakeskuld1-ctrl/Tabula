// ### Change Log
// - 2026-03-16: Reason=First Sheet creation may skip; Purpose=lock down fallback parser behavior.
// - 2026-03-16: Reason=TDD requirement; Purpose=fail before implementation exists.
import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-16: Reason=Fallback parser will live in utils; Purpose=decouple App from parsing rules.
import { resolveCreatedSessionId, buildCreateSessionPayload } from "../sessionCreateFallback";

// ### Change Log
// - 2026-03-16: Reason=Backend response may omit session_id; Purpose=ensure fallback works.
describe("resolveCreatedSessionId", () => {
  // ### Change Log
  // - 2026-03-16: Reason=Prefer backend session_id; Purpose=avoid unnecessary fallback.
  it("prefers response session_id", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Response may be nested; Purpose=match actual payload shape.
    const id = resolveCreatedSessionId({
      parsed: { data: { session_id: "s1" } },
      sessions: [],
      expectedName: "Sheet1",
    });
    // ### Change Log
    // - 2026-03-16: Reason=Should return provided session_id; Purpose=deterministic selection.
    expect(id).toBe("s1");
  });

  // ### Change Log
  // - 2026-03-16: Reason=Missing session_id should fallback; Purpose=prevent Sheet1 skip.
  it("falls back to matching session name", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Sessions list must be searched; Purpose=find newly created entry.
    const id = resolveCreatedSessionId({
      parsed: { data: {} },
      sessions: [
        { sessionId: "a1", name: "Sheet1" },
        { sessionId: "b2", name: "Sheet2" },
      ],
      expectedName: "Sheet1",
    });
    // ### Change Log
    // - 2026-03-16: Reason=Matching name should win; Purpose=correctly select Sheet1.
    expect(id).toBe("a1");
  });

  // ### Change Log
  // - 2026-03-16: Reason=Backend may reject null from_session_id; Purpose=omit when empty.
  it("omits from_session_id when empty", () => {
    // ### Change Log
    // - 2026-03-16: Reason=Payload should stay minimal; Purpose=avoid sending null fields.
    const payload = buildCreateSessionPayload({
      tableName: "t",
      sessionName: "Sheet1",
      fromSessionId: "",
    });
    // ### Change Log
    // - 2026-03-16: Reason=Empty from_session_id should be removed; Purpose=stabilize create_session.
    expect(payload).toEqual({
      table_name: "t",
      session_name: "Sheet1",
    });
  });
});
