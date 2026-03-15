// **[2026-02-26]** 变更原因：SPLIT 需自定义插件支持。
// **[2026-02-26]** 变更目的：引入自定义函数与数组返回能力。
import {
  ArraySize,
  CellError,
  ErrorType,
  FunctionArgumentType,
  FunctionPlugin,
  HyperFormula,
  SimpleRangeValue
} from 'hyperformula';

// **[2026-02-26]** 变更原因：SPLIT 默认参数需要统一出口。
// **[2026-02-26]** 变更目的：与常见 SPLIT 语义保持一致。
const DEFAULT_SPLIT_BY_EACH = true;
// **[2026-02-26]** 变更原因：空字符串拆分需要保留或丢弃策略。
// **[2026-02-26]** 变更目的：提供默认“移除空项”的行为。
const DEFAULT_REMOVE_EMPTY = true;

// **[2026-02-26]** 变更原因：需支持 SPLIT 返回数组/溢出。
// **[2026-02-26]** 变更目的：替换 HyperFormula 内置 SPLIT 的空格索引语义。
class SplitArrayPlugin extends FunctionPlugin {
  // **[2026-02-26]** 变更原因：注册 SPLIT 元数据与参数签名。
  // **[2026-02-26]** 变更目的：允许 text/delimiter/可选布尔参数。
  static implementedFunctions = {
    SPLIT: {
      method: 'splitArray',
      sizeOfResultArrayMethod: 'splitArraySize',
      parameters: [
        { argumentType: FunctionArgumentType.STRING },
        { argumentType: FunctionArgumentType.STRING },
        { argumentType: FunctionArgumentType.BOOLEAN, optionalArg: true, defaultValue: DEFAULT_SPLIT_BY_EACH },
        { argumentType: FunctionArgumentType.BOOLEAN, optionalArg: true, defaultValue: DEFAULT_REMOVE_EMPTY }
      ]
    }
  };

  // **[2026-02-26]** 变更原因：SPLIT 需要返回二维数组以触发溢出。
  // **[2026-02-26]** 变更目的：在单元格内渲染横向结果。
  splitArray(ast: any, state: any) {
    return this.runFunction(ast.args, state, this.metadata('SPLIT'), (text: string, delimiter: string, splitByEach: boolean, removeEmpty: boolean) => {
      const parts = this.splitTextParts(text, delimiter, splitByEach, removeEmpty);
      if (parts.length === 0) {
        return new CellError(ErrorType.NA, 'Empty range');
      }
      return SimpleRangeValue.onlyValues([parts]);
    });
  }

  // **[2026-02-26]** 变更原因：数组溢出需要预估输出尺寸。
  // **[2026-02-26]** 变更目的：提前计算列宽避免尺寸冲突。
  splitArraySize(ast: any, state: any) {
    if (!ast?.args || ast.args.length < 2) {
      return ArraySize.error();
    }
    const rawText = this.normalizeScalar(this.evaluateAst(ast.args[0], state));
    const rawDelimiter = this.normalizeScalar(this.evaluateAst(ast.args[1], state));
    if (rawText instanceof CellError || rawDelimiter instanceof CellError) {
      return ArraySize.error();
    }
    if (typeof rawText !== 'string' || typeof rawDelimiter !== 'string') {
      return ArraySize.scalar();
    }
    const rawSplitByEach = ast.args.length > 2 ? this.normalizeScalar(this.evaluateAst(ast.args[2], state)) : undefined;
    const rawRemoveEmpty = ast.args.length > 3 ? this.normalizeScalar(this.evaluateAst(ast.args[3], state)) : undefined;
    const splitByEach = typeof rawSplitByEach === 'boolean' ? rawSplitByEach : DEFAULT_SPLIT_BY_EACH;
    const removeEmpty = typeof rawRemoveEmpty === 'boolean' ? rawRemoveEmpty : DEFAULT_REMOVE_EMPTY;
    const parts = this.splitTextParts(rawText, rawDelimiter, splitByEach, removeEmpty);
    return new ArraySize(Math.max(1, parts.length), 1);
  }

  // **[2026-02-26]** 变更原因：分隔符可能包含正则特殊字符。
  // **[2026-02-26]** 变更目的：保证 split_by_each 正确拆分。
  private escapeRegExp(text: string) {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  // **[2026-02-26]** 变更原因：SPLIT 需要统一拆分逻辑。
  // **[2026-02-26]** 变更目的：复用逻辑以支持尺寸预测与执行。
  private splitTextParts(text: string, delimiter: string, splitByEach: boolean, removeEmpty: boolean) {
    if (delimiter === '') {
      return [];
    }
    let parts: string[];
    if (splitByEach) {
      const escaped = this.escapeRegExp(delimiter);
      const pattern = new RegExp(`[${escaped}]`, 'g');
      parts = text.split(pattern);
    } else {
      parts = text.split(delimiter);
    }
    if (removeEmpty) {
      parts = parts.filter(part => part !== '');
    }
    return parts.length === 0 ? [''] : parts;
  }

  // **[2026-02-26]** 变更原因：size 方法可能收到区间/数组。
  // **[2026-02-26]** 变更目的：统一提取首个标量用于估算。
  private normalizeScalar(value: any) {
    if (value instanceof SimpleRangeValue) {
      const [first] = value.valuesFromTopLeftCorner();
      return first;
    }
    return value;
  }
}

/**
 * Singleton wrapper for HyperFormula engine.
 * In a real app, this might need to manage multiple sheets/instances.
 */
export class FormulaEngine {
  private static instance: FormulaEngine;
  // **[2026-02-26]** 变更原因：避免重复注册 SPLIT 插件。
  // **[2026-02-26]** 变更目的：防止多次初始化抛错。
  private static splitPluginRegistered = false;
  private hf: HyperFormula;
  private sheetId: string = 'Sheet1';
  // **[2026-02-26]** 变更原因：支持 SPLIT 横向溢出显示。
  // **[2026-02-26]** 变更目的：缓存锚点右侧的溢出值映射。
  private spillMap: Map<string, Map<string, string>> = new Map();

  // **[2026-02-26]** 变更原因：插件需在实例化前注册。
  // **[2026-02-26]** 变更目的：保证 SPLIT 能被引擎识别。
  private static registerSplitPlugin() {
    if (FormulaEngine.splitPluginRegistered) {
      return;
    }
    const existing = HyperFormula.getFunctionPlugin('SPLIT');
    if (existing !== SplitArrayPlugin) {
      HyperFormula.registerFunctionPlugin(SplitArrayPlugin);
    }
    FormulaEngine.splitPluginRegistered = true;
  }

  private constructor() {
    // **[2026-02-26]** 变更原因：创建引擎前需注入 SPLIT 实现。
    // **[2026-02-26]** 变更目的：覆盖内置 SPLIT 的不兼容语义。
    FormulaEngine.registerSplitPlugin();
    this.hf = HyperFormula.buildEmpty({
      // **[2026-02-26]** 变更原因：保留 GPLv3 许可配置。
      // **[2026-02-26]** 变更目的：确保 HyperFormula 合规运行。
      licenseKey: 'gpl-v3',
      useArrayArithmetic: true,
    });
    // Add default sheet
    this.ensureSheet(this.sheetId);
  }

  public ensureSheet(sheetName: string): number {
    if (this.hf.doesSheetExist(sheetName)) {
      return this.hf.getSheetId(sheetName) as number;
    }
    try {
        const newSheetName = this.hf.addSheet(sheetName);
        return this.hf.getSheetId(newSheetName) as number;
    } catch (e) {
        console.error(`Failed to add sheet ${sheetName}`, e);
        return -1;
    }
  }

  public static getInstance(): FormulaEngine {
    if (!FormulaEngine.instance) {
      FormulaEngine.instance = new FormulaEngine();
    }
    return FormulaEngine.instance;
  }

  /**
   * Calculate a formula or return the value if it's not a formula.
   * Note: This is a simplified "stateless" calculation for now,
   * assuming the formula doesn't depend on other cells (or we haven't loaded them).
   * 
   * @param input The raw string (e.g., "=1+1" or "Hello")
   * @returns The calculated result or the original string
   */
  public calculate(input: string, col?: number, row?: number, sheetName: string = 'Sheet1'): string {
    if (!input.startsWith('=')) {
      return input;
    }

    try {
        const sheetId = this.ensureSheet(sheetName);
        // If col/row provided, we assume the formula is ALREADY set in the engine
        // (via setCellValue during data fetch or edit).
        // We just read the calculated value.
        if (col !== undefined && row !== undefined) {
            const val = this.hf.getCellValue({ sheet: sheetId, col, row });
            // **[2026-02-26]** 变更原因：数组函数返回 SimpleRangeValue。
            // **[2026-02-26]** 变更目的：避免将数组结果误判为错误。
            if (val instanceof CellError) {
                return val.message || "#ERROR";
            }
            if (val instanceof SimpleRangeValue) {
                // **[2026-02-26]** 变更原因：数组结果需要落到锚点与溢出映射。
                // **[2026-02-26]** 变更目的：锚点显示首项并触发溢出缓存。
                const arr = val.data;
                // **[2026-02-26]** 变更原因：锚点单元格必须返回单值。
                // **[2026-02-26]** 变更目的：保持网格单元格展示一致性。
                const anchor = String(arr[0]?.[0] ?? '');
                // **[2026-02-26]** 变更原因：记录横向溢出值。
                // **[2026-02-26]** 变更目的：供网格渲染读取相邻单元格显示。
                this.recordSpill(col, row, sheetName, arr);
                return anchor;
            }
            const formulaInCell = this.hf.getCellFormula({ sheet: sheetId, col, row });
            if (formulaInCell) {
                const arrayResult = this.hf.calculateFormula(formulaInCell, sheetId);
                if (Array.isArray(arrayResult) && Array.isArray(arrayResult[0])) {
                    this.recordSpill(col, row, sheetName, arrayResult as any[][]);
                    return String(arrayResult[0]?.[0] ?? '');
                }
            }
            return String(val);
        }

        // Fallback for standalone calculation (no context)
        const formula = input.substring(1);
        const result = this.hf.calculateFormula(formula, sheetId);
        // **[2026-02-26]** 变更原因：SPLIT 可能返回数组。
        // **[2026-02-26]** 变更目的：输出可读的数组内容。
        if (result instanceof CellError) {
          return result.message || "#ERROR";
        }
        if (result instanceof SimpleRangeValue) {
          // **[2026-02-26]** 变更原因：无位置信息无法记录映射。
          // **[2026-02-26]** 变更目的：保留首元素作为纯计算结果。
          const arr = result.data;
          return String(arr[0]?.[0] ?? '');
        }
        return result?.toString() ?? input;
    } catch (e) {
        console.error("Formula Error:", e);
        return "#ERROR";
    }
  }

  /**
   * Clear all data in the engine
   */
  public clear() {
      // Keep for backward compatibility, clear default sheet
      this.hf.clearSheet(0);
  }
  
  public clearSheet(sheetName: string) {
      if (this.hf.doesSheetExist(sheetName)) {
          const id = this.hf.getSheetId(sheetName) as number;
          this.hf.clearSheet(id);
      }
  }

  /**
   * Set cell value in the engine (to support dependencies later)
   */
  public setCellValue(col: number, row: number, value: string | number, sheetName: string = 'Sheet1') {
      let val = value;
      // Try to convert string to number if it looks like a number
      // This ensures SUM and other math functions work correctly with user input
      if (typeof val === 'string' && !val.startsWith('=')) {
          const trimmed = val.trim();
          if (trimmed !== '') {
            const num = Number(trimmed);
            if (!isNaN(num) && isFinite(num)) {
                val = num;
            }
          }
      }
      const sheetId = this.ensureSheet(sheetName);
      this.hf.setCellContents({ sheet: sheetId, col, row }, [[val]]);
      // **[2026-02-26]** 变更原因：单元格内容变化需清理溢出映射。
      // **[2026-02-26]** 变更目的：防止过期溢出值干扰显示。
      this.clearSpillForAnchor(col, row, sheetName);
  }

  public getRawValue(col: number, row: number, sheetName: string): any {
      if (!this.hf.doesSheetExist(sheetName)) return "SHEET_NOT_FOUND";
      const sheetId = this.hf.getSheetId(sheetName) as number;
      return this.hf.getCellValue({ sheet: sheetId, col, row });
  }

  public getSheetNames(): string[] {
      return this.hf.getSheetNames();
  }

  /**
   * Get list of all supported function names
   */
  public getSupportedFunctions(): string[] {
      return this.hf.getRegisteredFunctionNames();
  }

  // **[2026-02-26]** 变更原因：新增溢出读写接口。
  // **[2026-02-26]** 变更目的：供 GlideGrid 渲染时读取。
  public getSpillValue(col: number, row: number, sheetName: string): string | undefined {
      const key = this.sheetKey(sheetName);
      const map = this.spillMap.get(key);
      const cached = map?.get(this.cellKey(col, row));
      if (cached !== undefined) {
          return cached;
      }
      if (!this.hf.doesSheetExist(sheetName)) {
          return undefined;
      }
      const sheetId = this.hf.getSheetId(sheetName) as number;
      const val = this.hf.getCellValue({ sheet: sheetId, col, row });
      if (val instanceof CellError || val instanceof SimpleRangeValue) {
          return undefined;
      }
      if (val === null || val === undefined) {
          return undefined;
      }
      return String(val);
  }

  // **[2026-02-26]** 变更原因：记录锚点对应的溢出值。
  // **[2026-02-26]** 变更目的：为相邻单元格提供展示数据。
  private recordSpill(anchorCol: number, anchorRow: number, sheetName: string, data: any[][]) {
      // **[2026-02-26]** 变更原因：溢出范围可能随依赖变化。
      // **[2026-02-26]** 变更目的：先清理再写入避免残留旧值。
      this.clearSpillForAnchor(anchorCol, anchorRow, sheetName);
      const key = this.sheetKey(sheetName);
      let map = this.spillMap.get(key);
      if (!map) {
          map = new Map();
          this.spillMap.set(key, map);
      }
      // **[2026-02-26]** 变更原因：当前仅接入 SPLIT 横向溢出。
      // **[2026-02-26]** 变更目的：限制为单行输出避免误写。
      const row = data[0] || [];
      // **[2026-02-26]** 变更原因：锚点自身由 calculate 返回。
      // **[2026-02-26]** 变更目的：溢出缓存从右侧第一列开始。
      for (let i = 1; i < row.length; i += 1) {
          const value = row[i];
          map.set(this.cellKey(anchorCol + i, anchorRow), String(value ?? ''));
      }
  }

  // **[2026-02-26]** 变更原因：锚点公式变化需清理旧溢出。
  // **[2026-02-26]** 变更目的：避免过期值残留在显示层。
  private clearSpillForAnchor(anchorCol: number, anchorRow: number, sheetName: string) {
      const key = this.sheetKey(sheetName);
      const map = this.spillMap.get(key);
      if (!map) return;
      // **[2026-02-26]** 变更原因：溢出长度未知需保守清理。
      // **[2026-02-26]** 变更目的：限制清理范围避免性能抖动。
      for (let i = 1; i <= 256; i += 1) {
          const k = this.cellKey(anchorCol + i, anchorRow);
          if (!map.has(k)) break;
          map.delete(k);
      }
  }

  // **[2026-02-26]** 变更原因：sheetName 可能为空或非字符串。
  // **[2026-02-26]** 变更目的：统一溢出缓存键格式。
  private sheetKey(sheetName: string) {
      return String(sheetName || '');
  }

  // **[2026-02-26]** 变更原因：单元格坐标需要统一索引。
  // **[2026-02-26]** 变更目的：避免 Map 键格式不一致。
  private cellKey(col: number, row: number) {
      return `${col},${row}`;
  }
}
