// ### Change Log
// - 2026-03-15: Reason=Pivot persistence needs deterministic updates; Purpose=build header + data writes
// - 2026-03-15: Reason=Large payloads require batching; Purpose=provide chunk helper

export type PivotUpdate = {
  // ### Change Log
  // - 2026-03-15: Reason=Row index required by API; Purpose=write correct coordinates
  row: number;
  // ### Change Log
  // - 2026-03-15: Reason=Column key uses backend schema; Purpose=match batch_update_cells payload
  col: string;
  // ### Change Log
  // - 2026-03-15: Reason=API expects string; Purpose=normalize values
  val: string;
};

export type PivotUpdateInput = {
  headers: string[];
  data: Array<Array<string | number | null>>;
  columnNames: string[];
};

export type PivotUpdateOffsetInput = PivotUpdateInput & {
  rowOffset: number;
  colOffset: number;
};

export type PivotColumnAdd = {
  // ### Change Log
  // - 2026-03-15: Reason=Backend needs column name; Purpose=ensure schema expansion
  name: string;
  // ### Change Log
  // - 2026-03-15: Reason=Backend may require type; Purpose=default to utf8
  type: string;
};

export type PivotColumnAddInput = {
  headers: string[];
  columnNames: string[];
  prefix?: string;
  colOffset?: number;
};

export type PivotPersistErrorInput = {
  step: "create_session" | "ensure_columns" | "batch_update" | "current_sheet";
  status?: number;
  message?: string;
};

// ### Change Log
// - 2026-03-15: Reason=Headers must be written to new session; Purpose=show pivot titles in row 1
// - 2026-03-15: Reason=Keep builder pure; Purpose=easy to test
export const buildPivotUpdates = (input: PivotUpdateInput): PivotUpdate[] => {
  const updates: PivotUpdate[] = [];
  const headerRow = 0;
  const dataOffset = 1;
  const headers = input.headers || [];
  const columnNames = input.columnNames || [];
  const dataRows = input.data || [];

  for (let colIndex = 0; colIndex < headers.length; colIndex += 1) {
    const colName = columnNames[colIndex] ?? `col_${colIndex}`;
    updates.push({
      row: headerRow,
      col: colName,
      val: String(headers[colIndex] ?? "")
    });
  }

  for (let r = 0; r < dataRows.length; r += 1) {
    const rowValues = dataRows[r] ?? [];
    for (let c = 0; c < headers.length; c += 1) {
      const colName = columnNames[c] ?? `col_${c}`;
      const cellValue = rowValues[c] ?? "";
      updates.push({
        row: r + dataOffset,
        col: colName,
        val: String(cellValue ?? "")
      });
    }
  }

  return updates;
};

// ### Change Log
// - 2026-03-15: Reason=current-sheet uses selection offset; Purpose=shift row/col positions
// - 2026-03-15: Reason=Keep helper pure; Purpose=easy to test
export const buildPivotUpdatesWithOffset = (input: PivotUpdateOffsetInput): PivotUpdate[] => {
  // ### Change Log
  // - 2026-03-15: Reason=Column offset maps to schema index; Purpose=use shifted names
  const headers = input.headers || [];
  const rowOffset = Math.max(0, input.rowOffset || 0);
  const colOffset = Math.max(0, input.colOffset || 0);
  const baseNames = input.columnNames || [];
  const effectiveNames = baseNames.slice(colOffset, colOffset + headers.length);
  const baseUpdates = buildPivotUpdates({
    headers: input.headers,
    data: input.data,
    columnNames: effectiveNames
  });
  return baseUpdates.map((update) => ({
    ...update,
    row: update.row + rowOffset
  }));
};

// ### Change Log
// - 2026-03-15: Reason=Pivot output may exceed base columns; Purpose=prepare add-column payload
// - 2026-03-15: Reason=Keep helper pure; Purpose=easy to test
export const buildPivotColumnAdds = (input: PivotColumnAddInput): PivotColumnAdd[] => {
  const headers = input.headers || [];
  const columnNames = input.columnNames || [];
  const prefix = input.prefix || "pivot_col_";
  const colOffset = Math.max(0, input.colOffset || 0);
  const requiredCount = colOffset + headers.length;
  const missingCount = Math.max(0, requiredCount - columnNames.length);
  const adds: PivotColumnAdd[] = [];
  for (let i = 0; i < missingCount; i += 1) {
    const nextIndex = columnNames.length + i + 1;
    adds.push({
      name: `${prefix}${nextIndex}`,
      type: "utf8"
    });
  }
  return adds;
};

// ### Change Log
// - 2026-03-15: Reason=Friendly error needed; Purpose=consistent Chinese message
export const formatPivotPersistError = (input: PivotPersistErrorInput): string => {
  const status = input.status ? `（状态码 ${input.status}）` : "";
  const message = input.message ? `：${input.message}` : "";
  switch (input.step) {
    case "create_session":
      return `新建 Sheet 失败${status}${message}`;
    case "ensure_columns":
      return `扩列失败${status}${message}`;
    case "batch_update":
      return `落库失败${status}${message}`;
    case "current_sheet":
      return `当前 Sheet 写入失败${status}${message}`;
    default:
      return `Pivot 落库失败${status}${message}`;
  }
};

// ### Change Log
// - 2026-03-15: Reason=Payload size can be large; Purpose=split into manageable chunks
// - 2026-03-15: Reason=Keep chunking pure; Purpose=easy to test
export const chunkPivotUpdates = (updates: PivotUpdate[], chunkSize: number): PivotUpdate[][] => {
  const safeSize = Math.max(1, chunkSize || 1);
  const result: PivotUpdate[][] = [];
  for (let i = 0; i < updates.length; i += safeSize) {
    result.push(updates.slice(i, i + safeSize));
  }
  return result;
};
