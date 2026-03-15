// ### Change Log
// - 2026-03-15: Reason=TDD for pivot persistence; Purpose=lock update list behavior
// - 2026-03-15: Reason=Chunking needed for large pivot; Purpose=verify batching

import { describe, it, expect } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Helper drives persistence; Purpose=avoid App coupling
import { buildPivotUpdates, chunkPivotUpdates, buildPivotColumnAdds, buildPivotUpdatesWithOffset, formatPivotPersistError } from "../pivotSession";

// ### Change Log
// - 2026-03-15: Reason=Pivot rows must include header; Purpose=ensure first row is titles

describe("pivot session updates", () => {
  // ### Change Log
  // - 2026-03-15: Reason=Write headers + data; Purpose=keep new sheet readable
  it("builds header row + data rows", () => {
    const result = buildPivotUpdates({
      headers: ["A", "B"],
      data: [[1, 2], [3, 4]],
      columnNames: ["col_a", "col_b"]
    });
    expect(result[0]).toEqual({ row: 0, col: "col_a", val: "A" });
    expect(result[1]).toEqual({ row: 0, col: "col_b", val: "B" });
    expect(result[2]).toEqual({ row: 1, col: "col_a", val: "1" });
  });

  // ### Change Log
  // - 2026-03-15: Reason=Large payloads need batches; Purpose=verify chunk sizes
  it("chunks updates by size", () => {
    const updates = Array.from({ length: 5 }).map((_, i) => ({ row: i, col: "c", val: String(i) }));
    const chunks = chunkPivotUpdates(updates, 2);
    expect(chunks.length).toBe(3);
    expect(chunks[0].length).toBe(2);
  });

  // ### Change Log
  // - 2026-03-15: Reason=Pivot columns may exceed base schema; Purpose=ensure column add payload built
  it("builds column add payload when headers exceed base columns", () => {
    const result = buildPivotColumnAdds({
      headers: ["A", "B", "C"],
      columnNames: ["col_a"],
      prefix: "pivot_col_"
    });
    expect(result.length).toBe(2);
    expect(result[0].name).toBe("pivot_col_2");
  });

  // ### Change Log
  // - 2026-03-15: Reason=current-sheet uses selection offset; Purpose=verify row/col shift
  it("applies row/col offsets for current-sheet writes", () => {
    const result = buildPivotUpdatesWithOffset({
      headers: ["A"],
      data: [[1]],
      columnNames: ["c0", "c1", "c2", "c3"],
      rowOffset: 2,
      colOffset: 3
    });
    expect(result[0].row).toBe(2);
    expect(result[0].col).toBe("c3");
  });

  // ### Change Log
  // - 2026-03-15: Reason=Friendly errors needed; Purpose=ensure human-readable messages
  it("formats friendly error for ensure_columns", () => {
    const message = formatPivotPersistError({ step: "ensure_columns", status: 405 });
    expect(message).toContain("扩列失败");
  });

  // ### Change Log
  // - 2026-03-15: Reason=Offset may exceed base columns; Purpose=add missing columns for offset
  it("builds column add payload when offset exceeds base length", () => {
    const result = buildPivotColumnAdds({
      headers: ["A", "B"],
      columnNames: ["c0", "c1", "c2"],
      colOffset: 2,
      prefix: "pivot_col_"
    });
    expect(result.length).toBe(1);
    expect(result[0].name).toBe("pivot_col_4");
  });
});
