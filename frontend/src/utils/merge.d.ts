export interface MergeRange {
  start_row: number;
  start_col: number;
  end_row: number;
  end_col: number;
}

export function rebuildMergeIndex(
  merges: readonly MergeRange[] | null | undefined,
  globalMerges: Map<string, MergeRange>,
  cellToMergeMap: Map<string, string>
): void;

export function collectMergesFromCachePages(
  pages: readonly Array<{
    metadata?: {
      merges?: readonly Array<{
        start_row: number | string;
        start_col: number | string;
        end_row: number | string;
        end_col: number | string;
      }>;
    };
  }> | null | undefined
): MergeRange[];
