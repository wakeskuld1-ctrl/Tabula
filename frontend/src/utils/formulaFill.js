import { getExcelColumnIndex, getExcelColumnName } from "./formulaRange.js";

export function shiftFormulaReferences(formula, dx, dy) {
  if (typeof formula !== "string") return formula;
  const colShift = Number.isFinite(dx) ? dx : 0;
  const rowShift = Number.isFinite(dy) ? dy : 0;
  // **[2026-02-17]** 变更原因：原实现仅匹配大写列名。
  // **[2026-02-17]** 变更目的：支持小写列名引用的位移处理。
  // **[2026-02-17]** 变更原因：避免大小写差异导致引用遗漏。
  // **[2026-02-17]** 变更目的：对齐 Excel 列名大小写不敏感行为。
  return formula.replace(/(\$?)([A-Z]+)(\$?)(\d+)/gi, (match, colAbs, colName, rowAbs, rowStr) => {
    // **[2026-02-17]** 变更原因：统一列名处理路径。
    // **[2026-02-17]** 变更目的：保证列索引计算稳定一致。
    const colIndex = getExcelColumnIndex(String(colName).toUpperCase());
    if (!Number.isFinite(colIndex) || colIndex < 0) {
      return match;
    }
    const rowNum = Number(rowStr);
    if (!Number.isFinite(rowNum) || rowNum < 1) {
      return match;
    }
    const nextColIndex = colAbs ? colIndex : colIndex + colShift;
    const nextRowNum = rowAbs ? rowNum : rowNum + rowShift;
    if (nextColIndex < 0 || nextRowNum < 1) {
      return match;
    }
    const nextColName = getExcelColumnName(nextColIndex);
    return `${colAbs}${nextColName}${rowAbs}${nextRowNum}`;
  });
}

export function inferFillValues(sourceValues, targetLength) {
  const values = Array.isArray(sourceValues) ? sourceValues.map((val) => String(val ?? "")) : [];
  const desiredLength = Number.isFinite(targetLength) ? Math.max(0, Math.floor(targetLength)) : values.length;
  if (desiredLength <= values.length) {
    return values.slice(0, desiredLength);
  }
  if (values.length === 0) {
    return Array.from({ length: desiredLength }, () => "");
  }
  if (values.length === 1) {
    return Array.from({ length: desiredLength }, () => values[0]);
  }
  const numericValues = values.map((val) => Number(val));
  const allNumeric = numericValues.every((val) => Number.isFinite(val));
  if (allNumeric) {
    const step = numericValues[1] - numericValues[0];
    const out = numericValues.slice(0, values.length).map((val) => String(val));
    for (let i = values.length; i < desiredLength; i += 1) {
      out.push(String(numericValues[0] + step * i));
    }
    return out;
  }
  const datePattern = /^(\d{4})([-/])(\d{2})\2(\d{2})$/;
  const dateMatches = values.map((val) => val.match(datePattern));
  if (dateMatches.every(Boolean)) {
    // **[2026-02-17]** 变更原因：日期字符串可能不合法但可匹配正则。
    // **[2026-02-17]** 变更目的：避免自动纠正日期造成错误序列。
    const timestamps = dateMatches.map((match) => {
      const year = Number(match[1]);
      const month = Number(match[3]);
      const day = Number(match[4]);
      if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) {
        return null;
      }
      const time = Date.UTC(year, month - 1, day);
      const date = new Date(time);
      if (
        date.getUTCFullYear() !== year ||
        date.getUTCMonth() + 1 !== month ||
        date.getUTCDate() !== day
      ) {
        return null;
      }
      return time;
    });
    if (timestamps.some((value) => value === null)) {
      // **[2026-02-17]** 变更原因：日期无效时继续推断会产生误差。
      // **[2026-02-17]** 变更目的：回退为重复首值保证可预测行为。
      const out = values.slice(0, values.length);
      for (let i = values.length; i < desiredLength; i += 1) {
        out.push(values[0]);
      }
      return out;
    }
    const step = timestamps[1] - timestamps[0];
    const separator = dateMatches[0][2];
    const out = values.slice(0, values.length);
    for (let i = values.length; i < desiredLength; i += 1) {
      const nextTime = timestamps[0] + step * i;
      const nextDate = new Date(nextTime);
      const yyyy = nextDate.getUTCFullYear();
      const mm = String(nextDate.getUTCMonth() + 1).padStart(2, "0");
      const dd = String(nextDate.getUTCDate()).padStart(2, "0");
      out.push(`${yyyy}${separator}${mm}${separator}${dd}`);
    }
    return out;
  }
  const textNumberPattern = /^(.*?)(\d+)$/;
  const textMatches = values.map((val) => val.match(textNumberPattern));
  if (textMatches.every(Boolean)) {
    const prefix = textMatches[0][1];
    const width = textMatches[0][2].length;
    if (textMatches.every((match) => match[1] === prefix)) {
      const numbers = textMatches.map((match) => Number(match[2]));
      if (numbers.every((val) => Number.isFinite(val))) {
        const step = numbers[1] - numbers[0];
        const out = values.slice(0, values.length);
        for (let i = values.length; i < desiredLength; i += 1) {
          const next = numbers[0] + step * i;
          const nextText = String(next).padStart(width, "0");
          out.push(`${prefix}${nextText}`);
        }
        return out;
      }
    }
  }
  const out = values.slice(0, values.length);
  for (let i = values.length; i < desiredLength; i += 1) {
    out.push(values[0]);
  }
  return out;
}
