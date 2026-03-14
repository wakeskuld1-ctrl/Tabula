// **[2026-03-14]** 变更原因：测试脚本需要独立运行
// **[2026-03-14]** 变更目的：避免依赖 JS-only 模块导致编译失败
// **[2026-03-14]** 变更原因：提示文案需使用列字母
// **[2026-03-14]** 变更目的：内置最小列名转换逻辑
const getExcelColumnName = (index: number): string => {
  if (!Number.isFinite(index) || index < 0) return "";
  let n = index;
  let name = "";
  while (n >= 0) {
    name = String.fromCharCode((n % 26) + 65) + name;
    n = Math.floor(n / 26) - 1;
  }
  return name;
};

// **[2026-03-14]** 变更原因：统一公式失败提示文案
// **[2026-03-14]** 变更目的：确保提示格式一致
// **[2026-03-14]** 变更原因：列标题可能为空
// **[2026-03-14]** 变更目的：回退到 Excel 字母列名
export const buildFormulaFailureNotice = (
  col: number,
  row: number,
  columns: { title?: string }[]
): string => {
  const colTitle = columns[col]?.title || getExcelColumnName(col);
  const rowLabel = row + 1;
  return `单元格 ${colTitle}${rowLabel} 更新失败，请重试`;
};
