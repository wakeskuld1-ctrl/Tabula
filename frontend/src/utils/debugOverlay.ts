// ### Change Log
// - 2026-03-15: Reason=Auto-hide debug overlay needs shared logic; Purpose=centralize trigger rule
// - 2026-03-15: Reason=TDD helper extraction; Purpose=keep UI hooks small and testable

// ### Change Log
// - 2026-03-15: Reason=Only hide completed load messages; Purpose=avoid clearing errors
// - 2026-03-15: Reason=Loader rule must be strict; Purpose=prevent false positives
export const shouldAutoHideDebugInfo = (message: string, loading: boolean): boolean => {
  // ### Change Log
  // - 2026-03-15: Reason=Never hide while loading; Purpose=avoid flicker during fetch
  if (loading) return false;
  // ### Change Log
  // - 2026-03-15: Reason=Ignore empty messages; Purpose=avoid unnecessary timers
  if (!message || message.trim().length === 0) return false;
  // ### Change Log
  // - 2026-03-15: Reason=Match only "Loaded <table>: <n> rows"; Purpose=keep other notices visible
  const normalized = message.trim();
  // ### Change Log
  // - 2026-03-15: Reason=Stable regex; Purpose=avoid locale-dependent parsing
  return /^Loaded\s.+:\s\d+\srows$/i.test(normalized);
};
