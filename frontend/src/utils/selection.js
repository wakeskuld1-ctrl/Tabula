import { CompactSelection } from "@glideapps/glide-data-grid";

const isCompactSelection = (value) =>
  value && typeof value.offset === "function" && typeof value.add === "function";

export const normalizeCompactSelection = (input) => {
  if (isCompactSelection(input)) return input;
  if (input && Array.isArray(input.items)) {
    let selection = CompactSelection.empty();
    for (const item of input.items) {
      selection = selection.add(item);
    }
    return selection;
  }
  return CompactSelection.empty();
};

export const normalizeGridSelection = (selection) => {
  if (!selection) return undefined;
  const current = selection.current
    ? {
        ...selection.current,
        range: { ...selection.current.range },
        rangeStack: Array.isArray(selection.current.rangeStack)
          ? selection.current.rangeStack
          : [],
      }
    : undefined;
  return {
    current,
    columns: normalizeCompactSelection(selection.columns),
    rows: normalizeCompactSelection(selection.rows),
  };
};
