import type { CompactSelection, GridSelection } from "@glideapps/glide-data-grid";

export function normalizeCompactSelection(input: unknown): CompactSelection;

export function normalizeGridSelection(
  selection: GridSelection | null | undefined
): GridSelection | undefined;
