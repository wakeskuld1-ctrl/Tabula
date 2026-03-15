export const rebuildMergeIndex = (merges, globalMerges, cellToMergeMap) => {
  globalMerges.clear();
  cellToMergeMap.clear();
  if (!Array.isArray(merges)) return;
  for (const merge of merges) {
    if (!merge) continue;
    const key = `${merge.start_row},${merge.start_col}`;
    globalMerges.set(key, merge);
    for (let r = merge.start_row; r <= merge.end_row; r++) {
      for (let c = merge.start_col; c <= merge.end_col; c++) {
        cellToMergeMap.set(`${r},${c}`, key);
      }
    }
  }
};

export const collectMergesFromCachePages = (pages) => {
  if (!Array.isArray(pages)) return [];
  const merges = [];
  for (const page of pages) {
    const pageMerges = page?.metadata?.merges;
    if (!Array.isArray(pageMerges)) continue;
    for (const merge of pageMerges) {
      if (!merge) continue;
      const start_row = Number(merge.start_row);
      const start_col = Number(merge.start_col);
      const end_row = Number(merge.end_row);
      const end_col = Number(merge.end_col);
      if (
        Number.isNaN(start_row) ||
        Number.isNaN(start_col) ||
        Number.isNaN(end_row) ||
        Number.isNaN(end_col)
      ) {
        continue;
      }
      merges.push({
        start_row,
        start_col,
        end_row,
        end_col
      });
    }
  }
  return merges;
};
