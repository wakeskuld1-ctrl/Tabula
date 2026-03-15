const assert = require("assert");

const run = async () => {
  const selectionModule = await import("../utils/selection.js");
  const mergeModule = await import("../utils/merge.js");
  const gridModule = await import("@glideapps/glide-data-grid");
  const { normalizeCompactSelection, normalizeGridSelection } = selectionModule;
  const { rebuildMergeIndex } = mergeModule;
  const { CompactSelection } = gridModule;

  const empty = normalizeCompactSelection();
  assert.strictEqual(typeof empty.offset, "function");
  assert.strictEqual(empty.length, 0);

  const fromItems = normalizeCompactSelection({ items: [1, [3, 5]] });
  assert.strictEqual(fromItems.hasIndex(1), true);
  assert.strictEqual(fromItems.hasIndex(2), false);
  assert.strictEqual(fromItems.hasAll([3, 5]), true);

  const cs = CompactSelection.empty().add(2);
  const preserved = normalizeCompactSelection(cs);
  assert.strictEqual(preserved, cs);

  const inputSelection = {
    current: {
      cell: [1, 2],
      range: { x: 1, y: 2, width: 2, height: 3 },
      rangeStack: [],
    },
    columns: { items: [] },
    rows: { items: [0] },
  };

  const normalized = normalizeGridSelection(inputSelection);
  assert.strictEqual(typeof normalized.columns.offset, "function");
  assert.strictEqual(typeof normalized.rows.offset, "function");
  assert.strictEqual(normalized.current.cell[0], 1);
  assert.strictEqual(normalized.current.range.width, 2);

  const globalMerges = new Map();
  const cellToMergeMap = new Map();
  rebuildMergeIndex(
    [
      { start_row: 0, start_col: 0, end_row: 0, end_col: 1 },
      { start_row: 2, start_col: 2, end_row: 3, end_col: 2 },
    ],
    globalMerges,
    cellToMergeMap
  );
  assert.strictEqual(globalMerges.size, 2);
  assert.strictEqual(cellToMergeMap.get("0,0"), "0,0");
  assert.strictEqual(cellToMergeMap.get("0,1"), "0,0");
  assert.strictEqual(cellToMergeMap.get("3,2"), "2,2");
};

run()
  .then(() => {
    console.log("selection_unit_test passed");
  })
  .catch((err) => {
    console.error("selection_unit_test failed", err);
    process.exit(1);
  });
