export function shiftFormulaReferences(
  formula: unknown,
  dx: number,
  dy: number
): string;

// **[2026-03-15]** 变更原因：新增解析位移导出。
// **[2026-03-15]** 变更目的：补齐 TS 类型声明。
export function shiftFormulaReferencesWithParser(
  formula: unknown,
  dx: number,
  dy: number
): string;

export function inferFillValues(
  sourceValues: readonly unknown[] | null | undefined,
  targetLength: number
): string[];
