export function shiftFormulaReferences(
  formula: unknown,
  dx: number,
  dy: number
): string;

export function inferFillValues(
  sourceValues: readonly unknown[] | null | undefined,
  targetLength: number
): string[];
