// ### Change Log
// - 2026-03-16: Reason=Readonly sessions must block writes; Purpose=centralize guard logic.
// - 2026-03-16: Reason=User requested alert-first flow; Purpose=keep message consistent across UI.

// ### Change Log
// - 2026-03-16: Reason=UX copy should be stable; Purpose=single source for alert message.
export const READONLY_ALERT_MESSAGE = "请先创建新 Sheet（session）再编辑/保存";

// ### Change Log
// - 2026-03-16: Reason=Default session may lack id; Purpose=normalize guard inputs safely.
// - 2026-03-16: Reason=Avoid duplicated checks across components; Purpose=DRY guard evaluation.
export function getWriteGuardState(input: { sessionId?: string; isReadOnly?: boolean }) {
  // ### Change Log
  // - 2026-03-16: Reason=sessionId could be undefined/null; Purpose=normalize to trimmed string.
  const sessionId = (input.sessionId ?? "").trim();
  // ### Change Log
  // - 2026-03-16: Reason=isReadOnly can be undefined; Purpose=force boolean for guard rules.
  const isReadOnly = Boolean(input.isReadOnly);
  // ### Change Log
  // - 2026-03-16: Reason=Only sessions with id and writable flag can write; Purpose=block default session.
  const canWrite = Boolean(sessionId) && !isReadOnly;
  // ### Change Log
  // - 2026-03-16: Reason=Consumers need message for alerts; Purpose=centralize UI text.
  return {
    canWrite,
    message: canWrite ? "" : READONLY_ALERT_MESSAGE,
  };
}
