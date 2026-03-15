export type AggregateFunctionName =
  | "SUM"
  | "COUNT"
  | "COUNTA"
  | "AVG"
  | "AVERAGE"
  | "MAX"
  | "MIN";

export interface ParsedAggregateFormula {
  func: Exclude<AggregateFunctionName, "AVERAGE" | "COUNTA">;
  startCol: string;
  startRow: number | null;
  endCol: string;
  endRow: number | null;
}

export interface ColumnRangeInfo {
  type: "column";
  columns: string[];
  cellCount: null;
  startRow: null;
  endRow: null;
}

export interface CellRangeInfo {
  type: "cell";
  columns: string[];
  cellCount: number;
  startRow: number;
  endRow: number;
}

export type RangeInfo = ColumnRangeInfo | CellRangeInfo;

export interface FormulaColumnMarker {
  kind: "formula";
  raw: string;
  sql: string;
}

export interface FormulaColumnMeta {
  index: number;
  raw_expression?: string;
}

export function getExcelColumnName(colIndex: number): string;
export function getExcelColumnIndex(colName: string): number;
export function parseAggregateFormula(input: unknown): ParsedAggregateFormula | null;
export function getAggregateFunctionNames(): AggregateFunctionName[];
export function isAggregateFormulaFunction(rawFunc: unknown): boolean;
export function getRangeInfo(parsed: ParsedAggregateFormula | null): RangeInfo | null;
export function buildFormulaColumnSql(rawExpression: unknown, columns: unknown): string | null;
export function buildFormulaColumnMarker(rawExpression: unknown, columns: unknown): FormulaColumnMarker | null;
export function normalizeArithmeticFormula(rawExpression: unknown): string | null;
export function extractArithmeticFormulaColumns(rawExpression: unknown): string[] | null;
export function getArithmeticFormulaColumnIndexes(
  rawExpression: unknown,
  columnCount: number
): number[] | null;
export function validateFormulaColumnName(name: unknown): string | null;
export function isFormulaColumnIndex(
  colIndex: number,
  formulaColumns: readonly FormulaColumnMeta[] | null | undefined
): boolean;
export function getFormulaColumnDisplayValue(
  colIndex: number,
  formulaColumns: readonly FormulaColumnMeta[] | null | undefined,
  fallback: string
): string;
export function formatCellValue(rawValue: unknown, format: string | null | undefined): string;

declare const _default: {
  parseAggregateFormula: typeof parseAggregateFormula;
  getAggregateFunctionNames: typeof getAggregateFunctionNames;
  isAggregateFormulaFunction: typeof isAggregateFormulaFunction;
  getRangeInfo: typeof getRangeInfo;
  getExcelColumnIndex: typeof getExcelColumnIndex;
  getExcelColumnName: typeof getExcelColumnName;
  buildFormulaColumnSql: typeof buildFormulaColumnSql;
  buildFormulaColumnMarker: typeof buildFormulaColumnMarker;
  validateFormulaColumnName: typeof validateFormulaColumnName;
  isFormulaColumnIndex: typeof isFormulaColumnIndex;
  getFormulaColumnDisplayValue: typeof getFormulaColumnDisplayValue;
  formatCellValue: typeof formatCellValue;
};

export default _default;
