// ### Change Log
// - 2026-03-15: Reason=Formula must keep raw input; Purpose=separate storage vs display
// - 2026-03-15: Reason=UI uses computed display; Purpose=avoid overwriting raw formula

export type FormulaStorageResult = {
  // ### Change Log
  // - 2026-03-15: Reason=Store raw formula or normal input; Purpose=preserve dependency inputs
  storedValue: string;
  // ### Change Log
  // - 2026-03-15: Reason=Allow display override; Purpose=show computed value without losing raw
  displayValue: string;
  // ### Change Log
  // - 2026-03-15: Reason=Expose formula flag; Purpose=avoid string re-checks upstream
  isFormula: boolean;
};

// ### Change Log
// - 2026-03-15: Reason=Helper centralizes formula decision; Purpose=keep GlideGrid small
// - 2026-03-15: Reason=Avoid accidental trimming; Purpose=consistent stored values
export const resolveFormulaStorage = (rawInput: string, computedDisplay?: string): FormulaStorageResult => {
  // ### Change Log
  // - 2026-03-15: Reason=Normalize input safely; Purpose=avoid undefined handling in callers
  const normalized = typeof rawInput === "string" ? rawInput : String(rawInput ?? "");
  // ### Change Log
  // - 2026-03-15: Reason=Formula detection is shared; Purpose=single source of truth
  const isFormula = normalized.trim().startsWith("=");
  if (!isFormula) {
    return {
      storedValue: normalized,
      displayValue: normalized,
      isFormula
    };
  }
  return {
    storedValue: normalized.trim(),
    displayValue: computedDisplay ?? normalized.trim(),
    isFormula
  };
};
