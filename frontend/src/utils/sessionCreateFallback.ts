// ### Change Log
// - 2026-03-16: Reason=First Sheet creation may skip; Purpose=provide fallback parser for session id.
// - 2026-03-16: Reason=Keep App lean; Purpose=centralize parsing rules in utils.

// ### Change Log
// - 2026-03-16: Reason=We only need id + name for fallback; Purpose=avoid App-level imports here.
type SessionItemLite = { sessionId: string; name: string };

// ### Change Log
// - 2026-03-16: Reason=create_session response can be incomplete; Purpose=resolve best session id.
// - 2026-03-16: Reason=Fallback should match session_name; Purpose=avoid Sheet1 skip.
export function resolveCreatedSessionId(input: {
  parsed: any;
  sessions: SessionItemLite[];
  expectedName: string;
}) {
  // ### Change Log
  // - 2026-03-16: Reason=Backend may nest session in data.session; Purpose=read both shapes.
  const direct =
    input.parsed?.data?.session?.session_id ||
    input.parsed?.data?.session_id ||
    "";
  // ### Change Log
  // - 2026-03-16: Reason=Prefer direct session id; Purpose=avoid unnecessary fallback.
  if (direct) return String(direct);
  // ### Change Log
  // - 2026-03-16: Reason=Session list can include new item; Purpose=match by expected name.
  const matched = input.sessions.find((item) => item.name === input.expectedName);
  // ### Change Log
  // - 2026-03-16: Reason=No match should return empty; Purpose=let caller handle error.
  return matched?.sessionId || "";
}

// ### Change Log
// - 2026-03-16: Reason=Empty from_session_id should be omitted; Purpose=avoid backend rejects.
// - 2026-03-16: Reason=Keep payload construction consistent; Purpose=single source for create_session body.
export function buildCreateSessionPayload(input: {
  tableName: string;
  sessionName: string;
  fromSessionId?: string;
}) {
  // ### Change Log
  // - 2026-03-16: Reason=Always include required fields; Purpose=keep backend contract stable.
  const payload: { table_name: string; session_name: string; from_session_id?: string } = {
    table_name: input.tableName,
    session_name: input.sessionName,
  };
  // ### Change Log
  // - 2026-03-16: Reason=Null/empty from_session_id can break backend; Purpose=omit when missing.
  if (input.fromSessionId && input.fromSessionId.trim().length > 0) {
    payload.from_session_id = input.fromSessionId;
  }
  // ### Change Log
  // - 2026-03-16: Reason=Callers need final body; Purpose=return sanitized payload.
  return payload;
}
