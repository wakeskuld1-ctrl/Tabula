import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
// **[2026-02-16]** 变更原因：新增行主题缓存依赖；变更目的：避免重复计算
import { createPortal } from "react-dom";
import {
  DataEditor,
  DataEditorProps,
  EditListItem,
  GridCell,
  GridCellKind,
  GridColumn,
  GridColumnIcon,
  Item,
  GridSelection,
  CompactSelection,
  ProvideEditorCallback,
  Theme,
  HeaderClickedEventArgs,
  CellClickedEventArgs,
  Rectangle,
  DrawCellCallback,
  SpriteMap,
} from "@glideapps/glide-data-grid";
import "@glideapps/glide-data-grid/dist/index.css";
import { FormulaEngine } from "../utils/FormulaEngine";
import { FormulaEditor } from "./FormulaEditor";
import { normalizeGridSelection } from "../utils/selection.js";
import { collectMergesFromCachePages, rebuildMergeIndex } from "../utils/merge.js";
import {
  getExcelColumnName,
  getExcelColumnIndex,
  parseAggregateFormula,
  getRangeInfo,
  buildFormulaColumnMarker,
  getAggregateFunctionNames,
  normalizeArithmeticFormula,
  extractArithmeticFormulaColumns,
  getArithmeticFormulaColumnIndexes,
  validateFormulaColumnName,
  isFormulaColumnIndex,
  getFormulaColumnDisplayValue,
  formatCellValue
} from "../utils/formulaRange.js";
import { inferFillValues, shiftFormulaReferences } from "../utils/formulaFill.js";
// **[2026-03-14]** 变更原因：批量公式需要统一计算涉及页
// **[2026-03-14]** 变更目的：为批量刷新提供可测试的纯函数支撑
import { collectFormulaPages } from "../utils/collectFormulaPages";
// **[2026-03-14]** 变更原因：单元格需展示公式等待态
// **[2026-03-14]** 变更目的：计算并维护 pending key 集合
import { collectFormulaPendingKeys } from "../utils/collectFormulaPendingKeys";
// **[2026-03-14]** 变更原因：公式失败需要统一提示文案
// **[2026-03-14]** 变更目的：保证失败提示一致与可测试
import { buildFormulaFailureNotice } from "../utils/buildFormulaFailureNotice";
import { fetchGridData, fetchFilterValues, updateCell, batchUpdateCells } from "../utils/GridAPI";

const buildFormulaKey = (col: number, row: number) => `${row},${col}`;
const FILTER_PAGE_SIZE = 200;

// Data Cleaner for Excel Paste
const cleanData = (val: string): string => {
    if (!val) return "";
    let cleaned = val.trim();
    // Remove commas from numbers (e.g., "1,234.56" -> "1234.56")
    if (/^-?[\d,]+(\.\d+)?$/.test(cleaned)) {
         cleaned = cleaned.replace(/,/g, '');
    }
    // Remove currency symbols if it looks like a price
    if (/^[$¥€£]/.test(cleaned)) {
        cleaned = cleaned.substring(1);
    }
    return cleaned;
}

const trimPasteData = (rows: readonly (readonly string[])[]): string[][] => {
    const data = rows.map(row => Array.from(row, v => v ?? ""));
    let lastRow = data.length - 1;
    while (lastRow >= 0 && data[lastRow].every(v => v.trim() === "")) {
        lastRow -= 1;
    }
    if (lastRow < 0) {
        return [];
    }
    const trimmedRows = data.slice(0, lastRow + 1);
    let maxCols = 0;
    for (const row of trimmedRows) {
        if (row.length > maxCols) {
            maxCols = row.length;
        }
    }
    let lastCol = maxCols - 1;
    while (lastCol >= 0) {
        let allEmpty = true;
        for (const row of trimmedRows) {
            const val = row[lastCol];
            if (val !== undefined && val.trim() !== "") {
                allEmpty = false;
                break;
            }
        }
        if (allEmpty) {
            lastCol -= 1;
        } else {
            break;
        }
    }
    const finalCols = lastCol + 1;
    if (finalCols <= 0) {
        return [];
    }
    return trimmedRows.map(row => row.slice(0, finalCols));
}

const isIntegerString = (val: string): boolean => {
    return /^[-+]?\d+$/.test(val);
}

const isNumberString = (val: string): boolean => {
    if (val.trim() === "") return false;
    const parsed = Number(val);
    return Number.isFinite(parsed);
}

const inferColumnType = (rows: readonly (readonly string[])[], colIndex: number): string => {
    let hasValue = false;
    let allInt = true;
    let allNumber = true;
    for (const row of rows) {
        const raw = row[colIndex] ?? "";
        const cleaned = cleanData(String(raw)).trim();
        if (cleaned === "") {
            continue;
        }
        hasValue = true;
        if (!isIntegerString(cleaned)) {
            allInt = false;
        }
        if (!isNumberString(cleaned)) {
            allNumber = false;
            break;
        }
    }
    if (!hasValue) return "utf8";
    if (allNumber) {
        return allInt ? "int64" : "float64";
    }
    return "utf8";
}

// ### 变更记录
// - 2026-02-17: 原因=算术公式需识别数值列; 目的=前端拦截类型错误
// - 2026-02-17: 原因=覆盖多种数值类 目的=与后端类型命名对
const isNumericColumnType = (rawType?: string): boolean => {
    const t = String(rawType ?? "").toLowerCase();
    return t === "int64"
        || t === "int32"
        || t === "float64"
        || t === "double"
        || t.startsWith("decimal");
};

const getColumnIcon = (rawType: string): GridColumnIcon => {
    const t = rawType.toLowerCase();
    return t === "int64" || t === "float64"
        ? GridColumnIcon.HeaderNumber
        : GridColumnIcon.HeaderString;
}

// ### 变更记录
// - 2026-03-11 21:45: 原因=后端在部分环境缺/api/grid-data 或返回非 JSON; 目的=统一提供安全 JSON 解析，避免页面卡Loading Grid Metadata
const parseJsonResponseSafely = async (res: Response): Promise<{ ok: true; data: any } | { ok: false; reason: string; rawPreview: string }> => {
    const raw = await res.text();
    if (!raw || raw.trim().length === 0) {
        return { ok: false, reason: `empty response (status ${res.status})`, rawPreview: "" };
    }
    try {
        return { ok: true, data: JSON.parse(raw) };
    } catch (error) {
        return {
            ok: false,
            reason: `invalid json (status ${res.status})`,
            rawPreview: raw.slice(0, 240)
        };
    }
};

const readJsonOrThrow = async (res: Response, context: string): Promise<any> => {
    const parsed = await parseJsonResponseSafely(res);
    if (!res.ok) {
        const preview = parsed.ok ? JSON.stringify(parsed.data).slice(0, 240) : parsed.rawPreview;
        throw new Error(`${context} HTTP ${res.status}: ${preview || res.statusText}`);
    }
    if (!parsed.ok) {
        throw new Error(`${context} parse failed: ${parsed.reason}`);
    }
    return parsed.data ?? {};
};

// ### 变更记录
// - 2026-03-11 21:45: 原因=表名可能包含特殊字符，直接拼 SQL 会失 目的=统一标识符转义，保证 fallback 查询稳定
const quoteSqlIdentifier = (identifier: string): string => `"${String(identifier).replace(/"/g, `""`)}"`;
const quoteSqlLiteral = (value: string): string => `'${String(value).replace(/'/g, "''")}'`;

export interface GlideGridHandle {
  getCell: (col: number, row: number) => string | null;
  updateCell: (col: number, row: number, value: string) => Promise<void>;
  updateStyle: (col: number, row: number, style: any) => Promise<void>;
  mergeSelection: () => Promise<void>;
  updateSelectionStyle: (style: any) => Promise<void>;
  setSelection: (selection: GridSelection) => void;
  refresh: () => void;
  pasteValues: (target: Item, values: readonly (readonly string[])[]) => boolean;
  fillRange: (source: Rectangle, target: Rectangle) => Promise<boolean>;
  toggleFreeze: () => void;
  toggleFilter: () => void;
  openFilterMenuAt: (x: number, y: number, colIndex?: number) => void;
  removeFilter: (colId: string) => void;
  undo: () => void;
  redo: () => void;
  // ### 变更记录
  // - 2026-02-16: 原因=补充行级主题能力; 目的=支持外部测试与调
  getRowThemeForRow: (row: number) => Partial<Theme> | undefined;
}

export interface ActiveFilter {
    colId: string;
    colName: string; // Display name (e.g., column title or A/B/C)
    desc: string; // Description (e.g., "3 items" or "Contains 'foo'")
}

interface FormulaMeta {
    formula: string;
    columns: string[];
    stale: boolean;
    lastUpdatedAt: string;
}

// ### 变更记录
// - 2026-02-16: 原因=新增公式列元信息结构; 目的=对齐后端返回字段
// - 2026-02-16: 原因=前端识别只读 目的=限制公式列单元格编辑
interface FormulaColumnMeta {
    index: number;
    name: string;
    raw_expression: string;
    sql_expression: string;
}

interface GlideGridProps {
  sessionId: string;
  tableName: string;
  // ### 变更记录
  // - 2026-03-14: 原因=默认会话需只读；目允许外部控制编辑权限
  readOnly?: boolean;
  onSessionChange?: (newSessionId: string) => void;
  onSelectionChange?: (col: number, row: number, value: string, colName: string) => void;
  onFilterApply?: (payload: FilterApplyPayload) => void;
  onFilterChange?: (activeFilters: ActiveFilter[]) => void;
  onStackChange?: (canUndo: boolean, canRedo: boolean) => void;
  onFormulaMetaChange?: (col: number, row: number, meta: FormulaMeta | null) => void;
  onInvalidateByColumn?: (colIndex: number, exclude?: { col: number, row: number }) => void;
  onRefreshFormula?: (target: { col: number, row: number }) => void;
  formulaMetaMap?: Map<string, FormulaMeta>;
  // **[2026-02-16]** 变更原因：补Glide Data Grid 缺失能力参数
  // **[2026-02-16]** 变更目的：允许外部控制滚表头/右侧区域
  // **[2026-02-16]** 变更原因：与官方 DataEditor props 对齐
  // **[2026-02-16]** 变更目的：减少后续二次改动成本
  // **[2026-02-16]** 变更原因：支持分组表头与缩放配置
  // **[2026-02-16]** 变更目的：提升表现一致性与可扩展性
  overscrollX?: DataEditorProps["overscrollX"];
  overscrollY?: DataEditorProps["overscrollY"];
  headerHeight?: DataEditorProps["headerHeight"];
  groupHeaderHeight?: DataEditorProps["groupHeaderHeight"];
  headerIcons?: DataEditorProps["headerIcons"];
  rightElement?: DataEditorProps["rightElement"];
  rightElementProps?: DataEditorProps["rightElementProps"];
  verticalBorder?: DataEditorProps["verticalBorder"];
  scaleToRem?: DataEditorProps["scaleToRem"];
  getGroupDetails?: DataEditorProps["getGroupDetails"];
}

interface CellStyle {
    bold?: boolean;
    italic?: boolean;
    underline?: boolean;
    align?: "left" | "center" | "right";
    color?: string;
    bg_color?: string;
    // ### 变更记录
    // - 2026-02-16: 原因=新增格式化字 目的=支持单元格格式显
    // - 2026-02-16: 原因=保持类型约束; 目的=避免非法格式写入
    format?: "number" | "percent" | "currency" | "date";
}

interface FilterApplyPayload {
    colIndex: number;
    columnId: string;
    selectedValues: string[];
    searchText: string;
}

interface MergeRange {
    start_row: number;
    start_col: number;
    end_row: number;
    end_col: number;
}

interface SheetMetadata {
    styles: Record<string, CellStyle>;
    merges: MergeRange[];
}

type HistoryAction =
  | { type: 'cell-update', row: number, col: number, oldValue: string, newValue: string, colName: string }
  | { type: 'batch-update', changes: { row: number, col: number, oldValue: string, newValue: string, colName: string }[] };

const PAGE_SIZE = 100;

interface PageData {
  data: any[][];
  columns: string[];
  total_rows: number;
  metadata?: Partial<SheetMetadata>;
  formula_columns?: FormulaColumnMeta[];
}

export const GlideGrid = React.forwardRef((props: GlideGridProps, ref: React.Ref<GlideGridHandle>) => {
  const {
    sessionId,
    tableName,
    // ### 变更记录
    // - 2026-03-14: 原因=默认会话需只读；目注入外部 readOnly 状态
    readOnly,
    onSessionChange,
    onSelectionChange,
    onFilterApply,
    onFilterChange,
    onStackChange,
    onFormulaMetaChange,
    onInvalidateByColumn,
    onRefreshFormula,
    formulaMetaMap,
    overscrollX,
    overscrollY,
    headerHeight,
    groupHeaderHeight,
    headerIcons,
    rightElement,
    rightElementProps,
    verticalBorder,
    scaleToRem,
    getGroupDetails,
  } = props;

  // ### 变更记录
  // - 2026-03-14: 原因=readOnly 可能undefined；目统一布尔化处理
  const isReadOnly = Boolean(readOnly);
  // ### 变更记录
  // - 2026-03-14: 原因=session_id 会导致后端报错；目的=统一归一化处理
  const normalizedSessionId = sessionId?.trim() ? sessionId : undefined;
  // Cache: Map<pageIndex, PageData>
  const cache = useRef<Map<number, PageData>>(new Map());
  const fetching = useRef<Set<number>>(new Set());
  const [rowCount, setRowCount] = useState<number>(0);
  const gridWrapperRef = useRef<HTMLDivElement>(null);
  const [columns, setColumns] = useState<GridColumn[]>([]);
  const [columnTypes, setColumnTypes] = useState<string[]>([]);
  const [realColCount, setRealColCount] = useState<number>(0);
  const [realRowCount, setRealRowCount] = useState<number>(0);
  // ### 变更记录
  // - 2026-02-16: 原因=缓存公式列元信息; 目的=只读控制与公式栏显示
  // - 2026-02-16: 原因=单页数据无法覆盖; 目的=跨页保持列级状
  const [formulaColumns, setFormulaColumns] = useState<FormulaColumnMeta[]>([]);
    const [version, setVersion] = useState<number>(0); // Force update version with timestamp
    // ### 变更记录
    // - 2026-03-14: 原因=公式回显存在等待; 目的=单元格内展示“计算中”
    // - 2026-03-14: 原因=批量/单格共用逻辑; 目的=统一 pending 状态管理
    // - 2026-03-14: 原因=需要去重渲染; 目的=Set 结构便于去重
    const [pendingFormulaKeys, setPendingFormulaKeys] = useState<Set<string>>(new Set());
    // ### 变更记录
    // - 2026-03-14: 原因=批量/单格公式需立即反馈; 目的=追加 pending key
    // - 2026-03-14: 原因=避免直接突变 Set; 目的=保持状态不可变
    const addPendingFormulaKeys = useCallback((keys: Set<string>) => {
        setPendingFormulaKeys((prev) => {
            const next = new Set(prev);
            keys.forEach((key) => next.add(key));
            return next;
        });
    }, []);
    // ### 变更记录
    // - 2026-03-14: 原因=刷新完成后需恢复显示; 目的=清理 pending key
    // - 2026-03-14: 原因=避免直接突变 Set; 目的=保持状态不可变
    const clearPendingFormulaKeys = useCallback((keys: Set<string>) => {
        setPendingFormulaKeys((prev) => {
            const next = new Set(prev);
            keys.forEach((key) => next.delete(key));
            return next;
        });
    }, []);
    const [selection, setSelection] = useState<GridSelection | undefined>(undefined);
    
    // UI States
    const [freezeColumns, setFreezeColumns] = useState<number>(0);
    const [showFilterHeaders, setShowFilterHeaders] = useState<boolean>(true);
    const [filterMenuOpen, setFilterMenuOpen] = useState(false);
    const [filterMenuTarget, setFilterMenuTarget] = useState<{ x: number, y: number, colIndex: number } | null>(null);
    const [contextMenuOpen, setContextMenuOpen] = useState(false);
    // ### 变更记录
    // - 2026-02-16: 原因=新增公式列弹 目的=集中输入并提示不可编
    // - 2026-02-16: 原因=避免误插 目的=支持取消与校验提
    const [formulaColumnDialogOpen, setFormulaColumnDialogOpen] = useState(false);
    // **[2026-02-16]** 变更原因：区分插入与编辑模式
    // **[2026-02-16]** 变更目的：复用弹窗并减少状态分散
    const [formulaColumnDialogMode, setFormulaColumnDialogMode] = useState<"insert" | "edit">("insert");
    // ### 变更记录
    // - 2026-02-16: 原因=公式列需要列 目的=允许用户输入自定义名
    // - 2026-02-16: 原因=避免空 目的=配合校验提示
    const [formulaColumnName, setFormulaColumnName] = useState("");
    const [formulaColumnInput, setFormulaColumnInput] = useState("");
    const [formulaColumnError, setFormulaColumnError] = useState("");
    const [formulaColumnTargetIndex, setFormulaColumnTargetIndex] = useState<number | null>(null);
    // ### 变更记录
    // - 2026-02-17: 原因=新增公式示例选择; 目的=提供快捷填充入口
    // - 2026-02-17: 原因=避免误触保留状 目的=弹窗开关联
    const [formulaSampleOpen, setFormulaSampleOpen] = useState(false);
    // Modified: Added rowIndex
    const [contextMenuTarget, setContextMenuTarget] = useState<{ x: number, y: number, colIndex: number, rowIndex?: number } | null>(null);
    const [filterSearchText, setFilterSearchText] = useState("");
    const [filterValues, setFilterValues] = useState<string[]>([]);
    const [filterOffset, setFilterOffset] = useState<number>(0);
    const [filterSelected, setFilterSelected] = useState<Set<string>>(new Set());
    const [pasteNoticeVisible, setPasteNoticeVisible] = useState(false);
    const [importProgressVisible, setImportProgressVisible] = useState(false);
    const [importProgress, setImportProgress] = useState({ completed: 0, total: 0 });
    const pasteNoticeTimer = useRef<number | undefined>(undefined);
    const pasteNoticeToken = useRef(0);
    const pasteEnvLogged = useRef(false);
    // ### 变更记录
    // - 2026-03-14: 原因=公式更新失败缺乏提示; 目的=展示轻量提示条
    // - 2026-03-14: 原因=需要自动隐藏; 目的=不打断用户操作
    const [formulaNoticeVisible, setFormulaNoticeVisible] = useState(false);
    const [formulaNoticeMessage, setFormulaNoticeMessage] = useState("");
    const formulaNoticeTimer = useRef<number | undefined>(undefined);
    // ### 变更记录
    // - 2026-03-14: 原因=避免提示叠加; 目的=复用统一入口
    // - 2026-03-14: 原因=提示需 3 秒自动消失; 目的=避免干扰
    const showFormulaNotice = useCallback((message: string) => {
        setFormulaNoticeMessage(message);
        setFormulaNoticeVisible(true);
        if (formulaNoticeTimer.current !== undefined) {
            window.clearTimeout(formulaNoticeTimer.current);
        }
        formulaNoticeTimer.current = window.setTimeout(() => {
            setFormulaNoticeVisible(false);
            formulaNoticeTimer.current = undefined;
        }, 3000);
    }, []);
    // ### 变更记录
    // - 2026-02-17: 原因=聚合函数名单来自工具函数; 目的=提示与检测一
    // - 2026-02-17: 原因=只初始化一 目的=避免重复创建
    const aggregateFunctionNames = useMemo(() => getAggregateFunctionNames(), []);
    // ### 变更记录
    // - 2026-02-17: 原因=聚合函数检 目的=输入时即时提
    // - 2026-02-17: 原因=兼容大小 目的=与用户输入一
    const aggregateFunctionRegex = useMemo(() => {
        const pattern = aggregateFunctionNames.join("|");
        return new RegExp(`\\b(${pattern})\\s*\\(`, "i");
    }, [aggregateFunctionNames]);
    // ### 变更记录
    // - 2026-02-17: 原因=弹窗提示需要文 目的=清晰说明不支持聚
    // - 2026-02-17: 原因=聚合名单可变; 目的=保持文案同步
    const aggregateUnsupportedMessage = useMemo(
        () => `不支${aggregateFunctionNames.join("/")} 等聚合函数`,
        [aggregateFunctionNames]
    );
    // ### 变更记录
    // - 2026-02-17: 原因=提供算术示例; 目的=减少输入成本
    // - 2026-02-17: 原因=保持列表固定; 目的=避免频繁渲染
    const formulaSamples = useMemo(
        () => ["A+B", "A-B", "A*B", "A/B", "(A+B)/C", "1+2"],
        []
    );
    // **[2026-02-16]** 变更原因：新增自定义表头图标入口
    // **[2026-02-16]** 变更目的：提供可验证headerIcons 配置
    const customHeaderIconKey = "custom_header_alert";
    // **[2026-02-16]** 变更原因：补充表头图标定义
    // **[2026-02-16]** 变更目的：让脚本可验证自定义图标渲染
    const customHeaderIcons = useMemo<SpriteMap>(() => ({
        [customHeaderIconKey]: ({ fgColor, bgColor }) => (
            `<svg width="20" height="20" fill="none" xmlns="http://www.w3.org/2000/svg">
                <rect x="1" y="1" width="18" height="18" rx="4" fill="${bgColor}"/>
                <path d="M6 10h8" stroke="${fgColor}" stroke-width="2" stroke-linecap="round"/>
                <circle cx="10" cy="6" r="1.2" fill="${fgColor}"/>
            </svg>`
        )
    }), []);
    // **[2026-02-16]** 变更原因：兼容外部传入图标
    // **[2026-02-16]** 变更目的：避免覆盖已headerIcons 配置
    const mergedHeaderIcons = useMemo<SpriteMap>(() => ({
        ...customHeaderIcons,
        ...(headerIcons ?? {})
    }), [customHeaderIcons, headerIcons]);
    // **[2026-02-16]** 变更原因：对齐表头高度配置
    // **[2026-02-16]** 变更目的：配合脚本校验表头高度
    const resolvedHeaderHeight = headerHeight ?? 48;
    const resolvedGroupHeaderHeight = groupHeaderHeight ?? resolvedHeaderHeight;
    
    // Added: Row Sizes State
    const [rowSizes, setRowSizes] = useState<Map<number, number>>(new Map());

    // ### 变更记录
    // - 2026-02-16: 原因=为过期行提供统一主题; 目的=视觉提示需要刷
    // - 2026-02-16: 原因=避免重复对象创建; 目的=减少渲染抖动
    // - 2026-02-16: 原因=补充文本色提 目的=满足行主题测试断言
    const staleRowTheme = useMemo<Partial<Theme>>(() => ({
        bgCell: "rgba(220, 38, 38, 0.12)",
        textDark: "#fca5a5"
    }), []);

    // ### 变更记录
    // - 2026-02-16: 原因=公式过期状态存formulaMetaMap; 目的=统一推导行级状
    // - 2026-02-16: 原因=保持轻量计算; 目的=减少 getRowThemeOverride 开销
    const staleRowSet = useMemo(() => {
        const next = new Set<number>();
        if (!formulaMetaMap) return next;
        for (const [key, meta] of formulaMetaMap.entries()) {
            if (!meta?.stale) continue;
            const rowPart = key.split(",")[0];
            const rowNumber = Number(rowPart);
            if (!Number.isNaN(rowNumber)) {
                next.add(rowNumber);
            }
        }
        return next;
    }, [formulaMetaMap]);

    // ### 变更记录
    // - 2026-02-16: 原因=集中行级主题规则; 目的=复用到渲染与测试入口
    // - 2026-02-16: 原因=避免重复逻辑; 目的=单一职责便于维护
    const getStaleRowTheme = useCallback((row: number) => {
        if (!staleRowSet.has(row)) return undefined;
        return staleRowTheme;
    }, [staleRowSet, staleRowTheme]);

    // Filter & Sort State
    const [filters, setFilters] = useState<Map<string, { col: string, val: string[], op: string }>>(new Map());
    const [sort, setSort] = useState<{ col: string, order: "asc" | "desc" } | null>(null);

    // Undo/Redo Stacks
    const undoStack = useRef<HistoryAction[]>([]);
    const redoStack = useRef<HistoryAction[]>([]);

    const notifyStackChange = useCallback(() => {
        if (onStackChange) {
            onStackChange(undoStack.current.length > 0, redoStack.current.length > 0);
        }
    }, [onStackChange]);

    // ### 变更记录
    // - 2026-02-15: 原因=Undo/Redo 需要同步公式元信息; 目的=避免公式生效后撤销出现异常
    // - 2026-02-15: 原因=沿用公式解析逻辑; 目的=保持onCellEdited 一
    const applyFormulaMetaFromValue = useCallback(
        (col: number, row: number, value: unknown) => {
            const raw = typeof value === "string" ? value : String(value ?? "");
            const trimmed = raw.trim();
            if (trimmed.startsWith("=")) {
                const parsed = parseAggregateFormula(trimmed);
                const rangeInfo = getRangeInfo(parsed);
                if (rangeInfo && rangeInfo.columns.length > 0) {
                    onFormulaMetaChange?.(col, row, {
                        formula: trimmed,
                        columns: rangeInfo.columns,
                        stale: false,
                        lastUpdatedAt: new Date().toISOString()
                    });
                    return;
                }
            }
            onFormulaMetaChange?.(col, row, null);
        },
        [onFormulaMetaChange]
    );

    // ### 变更记录
    // - 2026-02-15: 原因=Undo/Redo 后公式栏未更 目的=保持选中单元格与公式栏一
    const syncSelectionValueIfMatch = useCallback(
        (col: number, row: number, value: unknown) => {
            const current = selectionRef.current?.current?.cell;
            if (!current) return;
            if (current[0] !== col || current[1] !== row) return;
            const colName = columns[col]?.id || "";
            onSelectionChange?.(col, row, String(value ?? ""), colName);
        },
        [columns, onSelectionChange]
    );

    const executeUndo = useCallback(async () => {
        // ### 变更记录
// - 2026-03-14: 原因=默认会话只读；目禁止撤销写操作
        if (isReadOnly) {
            alert('当前会话为只读，无法撤销');
            return;
        }
        const action = undoStack.current.pop();
        if (!action) return;

        console.log("[GlideGrid] Undo:", action);
        
        try {
            if (action.type === 'cell-update') {
                // Restore old value
                // We need to call backend to revert
                const { row, col, oldValue, colName } = action;
                
                // Optimistic update
                const page = Math.floor(row / PAGE_SIZE) + 1;
                const rIdx = row % PAGE_SIZE;
                const pageData = cache.current.get(page);
                if (pageData && pageData.data[rIdx]) {
                    pageData.data[rIdx][col] = oldValue;
                    setVersion(v => v + 1);
                    formulaEngine.current.setCellValue(col, row, oldValue, tableName);
                    // ### 变更记录
                    // - 2026-02-15: 原因=撤销后需刷新公式元信 目的=避免公式状态残
                    applyFormulaMetaFromValue(col, row, oldValue);
                    // ### 变更记录
                    // - 2026-02-15: 原因=撤销后需刷新公式 目的=选中单元格值一
                    syncSelectionValueIfMatch(col, row, oldValue);
                }

                await updateCell({
                     session_id: normalizedSessionId,
                     table_name: tableName,
                     row_idx: row,
                     col_idx: col,
                     col_name: colName,
                     old_value: action.newValue, // The value we are undoing FROM
                     new_value: oldValue
                });

                redoStack.current.push(action);
            } else if (action.type === 'batch-update') {
                // Restore batch
                const updates = action.changes.map(c => ({
                    row: c.row,
                    col: c.colName,
                    val: c.oldValue
                }));

                // Apply optimistic updates
                updates.forEach(u => {
                    const colIdx = columns.findIndex(c => c.id === u.col || c.title === u.col);
                    if (colIdx === -1) return;
                    const page = Math.floor(u.row / PAGE_SIZE) + 1;
                    const rIdx = u.row % PAGE_SIZE;
                    const pageData = cache.current.get(page);
                    if (pageData && pageData.data[rIdx]) {
                        pageData.data[rIdx][colIdx] = u.val;
                    }
                    formulaEngine.current.setCellValue(colIdx, u.row, u.val, tableName);
                    // ### 变更记录
                    // - 2026-02-15: 原因=批量撤销需同步公式元信 目的=保持公式状态一
                    applyFormulaMetaFromValue(colIdx, u.row, u.val);
                    // ### 变更记录
                    // - 2026-02-15: 原因=批量撤销需同步公式 目的=选中单元格值一
                    syncSelectionValueIfMatch(colIdx, u.row, u.val);
                });
                setVersion(v => v + 1);

                // Send to backend (using batch_update_cells)
                // We need to chunk it if it's large? 
                // Let's assume undo batch is same size as paste.
                // Reusing the batch logic is complex inside this function.
                // For now, send one big request or rely on backend limit. 
                // Backend limit is 200MB now, so should be fine for 5000 cells.
                
                await batchUpdateCells({
                    table_name: tableName,
                    session_id: normalizedSessionId,
                    updates: updates
                });

                redoStack.current.push(action);
            }
        } catch (e) {
            console.error("Undo failed", e);
            alert("Undo failed: " + e);
            // Push back?
            undoStack.current.push(action);
        } finally {
            notifyStackChange();
        }
    }, [sessionId, tableName, columns, isReadOnly, notifyStackChange, applyFormulaMetaFromValue, syncSelectionValueIfMatch]);

    const executeRedo = useCallback(async () => {
        // ### 变更记录
// - 2026-03-14: 原因=默认会话只读；目禁止重做写操作
        if (isReadOnly) {
            alert('当前会话为只读，无法重做');
            return;
        }
        const action = redoStack.current.pop();
        if (!action) return;

        console.log("[GlideGrid] Redo:", action);

        try {
            if (action.type === 'cell-update') {
                const { row, col, newValue, oldValue, colName } = action;
                
                // Optimistic
                const page = Math.floor(row / PAGE_SIZE) + 1;
                const rIdx = row % PAGE_SIZE;
                const pageData = cache.current.get(page);
                if (pageData && pageData.data[rIdx]) {
                    pageData.data[rIdx][col] = newValue;
                    setVersion(v => v + 1);
                    formulaEngine.current.setCellValue(col, row, newValue, tableName);
                    // ### 变更记录
                    // - 2026-02-15: 原因=重做后需刷新公式元信 目的=避免公式状态残
                    applyFormulaMetaFromValue(col, row, newValue);
                    // ### 变更记录
                    // - 2026-02-15: 原因=重做后需刷新公式 目的=选中单元格值一
                    syncSelectionValueIfMatch(col, row, newValue);
                }

                await updateCell({
                     session_id: normalizedSessionId,
                     table_name: tableName,
                     row_idx: row,
                     col_idx: col,
                     col_name: colName,
                     old_value: oldValue,
                     new_value: newValue
                });

                undoStack.current.push(action);
            } else if (action.type === 'batch-update') {
                 const updates = action.changes.map(c => ({
                    row: c.row,
                    col: c.colName,
                    val: c.newValue
                }));

                updates.forEach(u => {
                    const colIdx = columns.findIndex(c => c.id === u.col || c.title === u.col);
                    if (colIdx === -1) return;
                    const page = Math.floor(u.row / PAGE_SIZE) + 1;
                    const rIdx = u.row % PAGE_SIZE;
                    const pageData = cache.current.get(page);
                    if (pageData && pageData.data[rIdx]) {
                        pageData.data[rIdx][colIdx] = u.val;
                    }
                    formulaEngine.current.setCellValue(colIdx, u.row, u.val, tableName);
                    // ### 变更记录
                    // - 2026-02-15: 原因=批量重做需同步公式元信 目的=保持公式状态一
                    applyFormulaMetaFromValue(colIdx, u.row, u.val);
                    // ### 变更记录
                    // - 2026-02-15: 原因=批量重做需同步公式 目的=选中单元格值一
                    syncSelectionValueIfMatch(colIdx, u.row, u.val);
                });
                setVersion(v => v + 1);

                await batchUpdateCells({
                    table_name: tableName,
                    session_id: normalizedSessionId,
                    updates: updates
                });

                undoStack.current.push(action);
            }
        } catch (e) {
            console.error("Redo failed", e);
            alert("Redo failed: " + e);
            redoStack.current.push(action);
        } finally {
            notifyStackChange();
        }
    }, [sessionId, tableName, columns, isReadOnly, notifyStackChange, applyFormulaMetaFromValue, syncSelectionValueIfMatch]);

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // Check if active element is body or grid to avoid conflict with other inputs?
            // GlideGrid captures events usually.
            // But we can listen on window for Ctrl+Z
            if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
                if (e.shiftKey) {
                    // Redo
                    e.preventDefault();
                    executeRedo();
                } else {
                    // Undo
                    e.preventDefault();
                    executeUndo();
                }
            } else if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
                // Redo
                e.preventDefault();
                executeRedo();
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, [executeUndo, executeRedo]);

  useEffect(() => {
    if (onFilterChange) {
      const activeFilters: ActiveFilter[] = [];
      filters.forEach((f, colId) => {
          // Find column index to get name
          const colIndex = columns.findIndex(c => c.id === colId);
          const colName = colIndex >= 0 ? (columns[colIndex].title || getExcelColumnName(colIndex)) : colId;
          
          let desc = "";
          if (f.val.length > 0) {
              if (f.val.length === 1) desc = `Is '${f.val[0]}'`;
              else desc = `${f.val.length} items`;
          } else {
              desc = "Active";
          }
          
          activeFilters.push({
              colId,
              colName,
              desc
          });
      });
      onFilterChange(activeFilters);
    }
  }, [filters, onFilterChange, columns]);

    // Global Metadata Store (to solve cross-page fragmentation)
    // Key: "startRow,startCol" -> MergeRange
    const globalMerges = useRef<Map<string, MergeRange>>(new Map());
    // Key: "row,col" -> "startRow,startCol" (Fast lookup for any cell in a merge)
    const cellToMergeMap = useRef<Map<string, string>>(new Map());
    // Key: "row,col" -> CellStyle
    const globalStyles = useRef<Map<string, CellStyle>>(new Map());

    const rebuildMergesFromCache = useCallback(() => {
        // ### 变更记录
        // - 2026-03-12: 原因=用户反馈合并单元格不可用; 目的=不再丢弃跨行/矩阵合并范围
        // - 2026-03-12: 原因=兼容后端字符串数 目的=统一为数值索引后再建索引
        const merges = collectMergesFromCachePages(Array.from(cache.current.values()));
        rebuildMergeIndex(merges, globalMerges.current, cellToMergeMap.current);
    }, []);

    // Force update version with timestamp
    useEffect(() => {
        console.log("[GlideGrid] Version updated:", version);
    }, [version]);
  const selectionRef = useRef<GridSelection | undefined>(undefined);

  // Sync selection to ref for imperative handle access
  useEffect(() => {
    selectionRef.current = selection;
  }, [selection]);

  useEffect(() => {
    return () => {
      if (pasteNoticeTimer.current !== undefined) {
        window.clearTimeout(pasteNoticeTimer.current);
        pasteNoticeTimer.current = undefined;
      }
      // ### 变更记录
      // - 2026-03-14: 原因=提示条计时器需清理; 目的=避免泄漏
      if (formulaNoticeTimer.current !== undefined) {
        window.clearTimeout(formulaNoticeTimer.current);
        formulaNoticeTimer.current = undefined;
      }
    };
  }, []);

  useEffect(() => {
    (window as any).CompactSelection = CompactSelection;
    (window as any).formulaEngine = formulaEngine.current;
  }, []);

  const filterColIndex = filterMenuTarget?.colIndex;

  // Initialize selection when menu opens
  useEffect(() => {
      if (filterMenuOpen && filterColIndex !== undefined) {
          setFilterOffset(0);
          const colId = columns[filterColIndex]?.id;
          const currentFilter = filters.get(colId || "");
          if (currentFilter) {
              setFilterSelected(new Set(currentFilter.val));
          } else {
              setFilterSelected(new Set()); 
          }
          setFilterSearchText("");
      }
  }, [filterMenuOpen, filterColIndex]); // Intentionally exclude filters/columns to run only on open

  // Fetch values when menu open or search changes
  useEffect(() => {
      if (!filterMenuOpen || filterColIndex === undefined) return;
      
      const colId = columns[filterColIndex]?.id;
      if (!colId) return;

      const fetchValues = async () => {
          try {
              const filterParams = Array.from(filters.values());
              // Add version to URL to prevent caching and ensure fresh data after edits
              // ### 变更记录
              // - 2026-03-12: 原因=用户反馈筛选值仅返回 5  目的=显式limit，避免后端默认小分页
              // - 2026-03-12: 原因=滚动加载需要稳定步 目的=offset/limit 保持同一分页大小
              let url = `/api/filter-values?table_name=${tableName}&column=${colId}&v=${version}&offset=${filterOffset}&limit=${FILTER_PAGE_SIZE}`;
              if (normalizedSessionId) {
                  url += `&session_id=${normalizedSessionId}`;
              }
              if (filterParams.length > 0) {
                  url += `&current_filters=${encodeURIComponent(JSON.stringify(filterParams))}`;
              }
              if (filterSearchText) {
                  url += `&search_text=${encodeURIComponent(filterSearchText)}`;
              }

              console.log(`[GlideGrid] Fetching filter values for ${colId} (v=${version}, offset=${filterOffset})`);
              let json = await fetchFilterValues(tableName, colId, filterSearchText || '', FILTER_PAGE_SIZE, filterOffset);
              let responseStatus: number | string = "n/a";
              if (!json || json.status !== 'ok' || !Array.isArray(json.values)) {
                  const activeFilterClauses = filterParams
                      .filter((item) => item.col && Array.isArray(item.val) && item.val.length > 0 && item.col !== colId)
                      .map((item) => {
                          const column = quoteSqlIdentifier(item.col);
                          const values = item.val.map((v) => quoteSqlLiteral(String(v))).join(", ");
                          return `${column} IN (${values})`;
                      });
                  const searchClause = filterSearchText
                      ? `CAST(${quoteSqlIdentifier(colId)} AS TEXT) LIKE ${quoteSqlLiteral(`%${filterSearchText}%`)}`
                      : "";
                  const whereParts = [...activeFilterClauses];
                  if (searchClause) {
                      whereParts.push(searchClause);
                  }
                  const whereSql = whereParts.length > 0 ? ` WHERE ${whereParts.join(" AND ")}` : "";
                  const offset = Math.max(0, filterOffset);
                  const fallbackSql = `SELECT DISTINCT ${quoteSqlIdentifier(colId)} AS value FROM ${quoteSqlIdentifier(tableName)}${whereSql} ORDER BY value LIMIT ${FILTER_PAGE_SIZE} OFFSET ${offset}`;
                  const fallbackRes = await fetch("/api/execute", {
                      method: "POST",
                      headers: { "Content-Type": "application/json" },
                      body: JSON.stringify({ sql: fallbackSql })
                  });
                  responseStatus = fallbackRes.status;
                  if (fallbackRes.ok) {
                      const fallbackParsed = await parseJsonResponseSafely(fallbackRes);
                      if (fallbackParsed.ok) {
                          const rows = Array.isArray(fallbackParsed.data?.rows) ? fallbackParsed.data.rows : [];
                          const values = rows
                              .map((row: any[]) => row?.[0])
                              .filter((value: any) => value !== null && value !== undefined)
                              .map((value: any) => String(value));
                          json = { status: "ok", values };
                      }
                  }
              }
              console.log(`[GlideGrid] Filter values response status=${responseStatus}`, json);
              
              if (json.status === 'ok' && Array.isArray(json.values)) {
                  console.log(`[GlideGrid] Filter values received:`, json.values);
                  
                  if (filterOffset === 0) {
                      setFilterValues(json.values);
                      // If no filter is active for this column, and we just opened the menu (no search),
                      // auto-select all returned values to mimic "Select All"
                      const currentFilter = filters.get(colId);
                      if (!currentFilter && !filterSearchText) {
                          setFilterSelected(new Set(json.values));
                      }
                  } else {
                      setFilterValues(prev => [...prev, ...json.values]);
                  }
              }
          } catch (e) {
              console.error("Failed to fetch filter values", e);
          }
      };
      
      const timeoutId = setTimeout(fetchValues, 200);
      return () => clearTimeout(timeoutId);
  }, [filterMenuOpen, filterColIndex, filterSearchText, tableName, filters, columns, version, filterOffset, sessionId]);

  // Trigger re-render when data arrives
  const [, forceUpdate] = useState({});

  // Initialize Formula Engine
  const formulaEngine = useRef(FormulaEngine.getInstance());
  // ### 变更记录
  // - 2026-02-16: 原因=公式列判断复 目的=只读与公式栏展示一
  // - 2026-02-16: 原因=减少重复遍历; 目的=提升渲染性能
  const formulaColumnIndexSet = React.useMemo(
      () => new Set(formulaColumns.map((item) => item.index)),
      [formulaColumns]
  );

  // ### 变更记录
  // - 2026-02-16: 原因=新增单元格格式入 目的=复用选区样式更新
  // - 2026-02-16: 原因=避免逻辑分散; 目的=集中处理范围更新
  const applySelectionStyle = useCallback(async (style: any) => {
      // ### 变更记录
      // - 2026-03-14: 原因=默认会话只读；目阻止样式修改入口
      if (isReadOnly) {
          alert('当前会话为只读，无法修改样式');
          return;
      }
      // **[2026-02-16]** 变更原因：无选区时直接返回
      // **[2026-02-16]** 变更目的：避免空请求与无意义刷新
      if (!selection || !selection.current) return;
      
      // **[2026-02-16]** 变更原因：需要后端范围更新
      // **[2026-02-16]** 变更目的：把选区转换start/end
      const range = selection.current.range;
      const styleRange: MergeRange = {
          start_col: range.x,
          start_row: range.y,
          end_col: range.x + range.width - 1,
          end_row: range.y + range.height - 1
      };
      
      try {
          // **[2026-02-16]** 变更原因：同步到后端
          // **[2026-02-16]** 变更目的：保证样式可持久化
          console.log(`[GlideGrid] Updating style for range:`, styleRange, style);
          const res = await fetch("/api/update_style_range", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                  table_name: tableName,
                  session_id: normalizedSessionId,
                  range: styleRange,
                  style: style
              })
          });
          const data = await readJsonOrThrow(res, "update_style_range");
          if (data.status === 'ok') {
               // **[2026-02-16]** 变更原因：减少刷新依赖
               // **[2026-02-16]** 变更目的：本地立即生效
               // Optimistic Update
               // Update Global Store directly
               for (let r = styleRange.start_row; r <= styleRange.end_row; r++) {
                   for (let c = styleRange.start_col; c <= styleRange.end_col; c++) {
                       const key = `${r},${c}`;
                       const currentStyle = globalStyles.current.get(key) || {};
                       globalStyles.current.set(key, { ...currentStyle, ...style });
                   }
               }
               setVersion(v => v + 1);
            } else {
                // **[2026-02-16]** 变更原因：保留失败提示
                // **[2026-02-16]** 变更目的：便于排查接口问题
                console.error("Style range update failed:", data.message || data.error || "unknown error");
            }
        } catch (e) {
          // **[2026-02-16]** 变更原因：捕获网络异常
          // **[2026-02-16]** 变更目的：避免异常中断渲染
          console.error("Style update error:", e);
      }
  }, [selection, tableName, normalizedSessionId, isReadOnly]);

  React.useImperativeHandle(ref, () => ({
    getCell: (col: number, row: number) => {
        const page = Math.floor(row / PAGE_SIZE) + 1;
        const pageData = cache.current.get(page);
        if (!pageData || !pageData.data) return null;
        const rowIndexInPage = row % PAGE_SIZE;
        const rowData = pageData.data[rowIndexInPage];
        return rowData ? (rowData[col] ?? null) : null;
    },
    updateCell: async (col: number, row: number, value: string) => {
      const item: Item = [col, row];
      const cell: GridCell = { 
          kind: GridCellKind.Text, 
          data: value, 
          displayData: value, 
          allowOverlay: true, 
          readonly: false 
      };
      await onCellEdited(item, cell);
    },
    fillRange: async (source: Rectangle, target: Rectangle) => {
      return fillRange(source, target);
    },
    updateStyle: async (col: number, row: number, style: any) => {
        // ### 变更记录
        // - 2026-03-14: 原因=默认会话只读；目阻止粘贴写入
        if (isReadOnly) {
            alert('当前会话为只读，无法修改样式');
            return;
        }
        try {
            console.log(`[GlideGrid] Updating style: Row ${row}, Col ${col}`, style);
            const res = await fetch("/api/update_style", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    table_name: tableName,
                    session_id: normalizedSessionId,
                    row: row,
                    col: col,
                    style: style
                })
            });
            const data = await readJsonOrThrow(res, "update_style");
            if (data.status === 'ok') {
                 // Refresh or Optimistic Update?
                 // For now, let's just refresh the page data or manually update cache
                 const page = Math.floor(row / PAGE_SIZE) + 1;
                 const pageData = cache.current.get(page);
                 if (pageData) {
                     // if (!pageData.metadata) pageData.metadata = { styles: {}, merges: [] }; // No longer used for rendering
                     const key = `${row},${col}`;
                     // pageData.metadata.styles[key] = { ...pageData.metadata.styles[key], ...style };
                     
                     // Update Global Store
                     const currentStyle = globalStyles.current.get(key) || {};
                     globalStyles.current.set(key, { ...currentStyle, ...style });
                     
                     forceUpdate({});
                 }
            } else {
                console.error("Style update failed:", data.message || data.error || "unknown error");
            }
        } catch (e) {
            console.error("Style update error:", e);
        }
    },
    updateSelectionStyle: async (style: any) => {
        // ### 变更记录
        // - 2026-02-16: 原因=统一调用入口; 目的=复用选区样式逻辑
        // - 2026-02-16: 原因=保持旧接 目的=避免外部调用变更
        await applySelectionStyle(style);
    },
    mergeSelection: async () => {
        // ### 变更记录
        // - 2026-03-14: 原因=默认会话只读；目阻止粘贴写入
        if (isReadOnly) {
            alert('当前会话为只读，无法合并');
            return;
        }
        if (!selectionRef.current || !selectionRef.current.current) return;
        
        const range = selectionRef.current.current.range;
        if (range.height > 1) {
            alert("当前版本仅支持横向合并，纵向/矩阵合并暂不支持");
            return;
        }
        // range has x, y, width, height
        // Convert to start/end
        const mergeRange: MergeRange = {
            start_col: range.x,
            start_row: range.y,
            end_col: range.x + range.width - 1,
            end_row: range.y + range.height - 1
        };
        
        try {
            console.log(`[GlideGrid] Merging selection:`, mergeRange);
            const res = await fetch("/api/update_merge", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    table_name: tableName,
                    range: mergeRange
                })
            });
            const data = await readJsonOrThrow(res, "update_merge");
            if (data.status === 'ok') {
                 // Update cache optimistically across all potential pages
                 // Since merge can span pages, we iterate cache
                 cache.current.forEach((pageData, pageNum) => {
                     if (!pageData.metadata) pageData.metadata = { styles: {}, merges: [] };
                     if (!pageData.metadata.merges) pageData.metadata.merges = [];
                     
                     // Remove overlaps
                     // We want to KEEP disjoint ranges. 
                     // Disjoint = (End < Start) OR (Start > End) ...
                     pageData.metadata.merges = pageData.metadata.merges.filter(m => 
                        m.end_row < mergeRange.start_row || 
                        m.start_row > mergeRange.end_row || 
                        m.end_col < mergeRange.start_col || 
                        m.start_col > mergeRange.end_col
                     );
                     
                     // Add new merge ONLY to the page where the start cell resides
                      // Glide Data Grid only looks for span at the top-left cell
                      const startPage = Math.floor(mergeRange.start_row / PAGE_SIZE) + 1;
                      if (pageNum === startPage) {
                          if (data.message === "Merged") {
                             pageData.metadata.merges.push(mergeRange);
                          } else if (data.message === "Unmerged") {
                              console.log("[GlideGrid] Unmerged selection:", mergeRange);
                              // Already removed by the filter above
                          }
                      }
                  });
                 rebuildMergesFromCache();
                 
                 // Force update version
                 setVersion(v => v + 1);
                 
                 // Handle Unmerged message
                 if (data.message === "Unmerged") {
                     alert("Range unmerged");
                 }
            } else {
                console.error("Merge failed:", data.message);
            }
        } catch (e) {
            console.error("Merge error:", e);
        }
    },
        setSelection: (newSelection: GridSelection) => {
        const normalizedSelection = normalizeGridSelection(newSelection) ?? newSelection;
        setSelection(normalizedSelection);
        selectionRef.current = normalizedSelection;
    },
    refresh: () => {
      cache.current.clear();
      fetching.current.clear();
      globalMerges.current.clear();
      cellToMergeMap.current.clear();
      globalStyles.current.clear();
      fetchPage(1);
    },
    pasteValues: (target: Item, values: readonly (readonly string[])[]) => {
        return onPaste(target, values);
    },
    toggleFreeze: () => {
        // GDG v6 不支持顶部行冻结，原 setFreezeRows(row) 已移除，仅保留列冻结
        if (freezeColumns > 0) {
            // Unfreeze
            setFreezeColumns(0);
        } else {
            // Freeze based on selection
            if (selectionRef.current && selectionRef.current.current) {
                const { cell } = selectionRef.current.current;
                const [col] = cell;
                // Freeze everything BEFORE this cell
                setFreezeColumns(col);
                alert("当前版本仅支持列冻结");
            }
        }
    },
    toggleFilter: () => {
        setShowFilterHeaders(prev => {
            const newState = !prev;
            setColumns(cols => cols.map(c => ({ ...c, hasMenu: newState })));
            return newState;
        });
    },
    openFilterMenuAt: (x: number, y: number, colIndex?: number) => {
        const maxCol = columns.length > 0 ? columns.length - 1 : 0;
        const safeCol = Math.max(0, Math.min(colIndex ?? 0, maxCol));
        const left = safeCol === 0 ? 0 : Math.max(0, x);
        setFilterMenuTarget({ x: left, y, colIndex: safeCol });
        setFilterMenuOpen(true);
        setContextMenuOpen(false);
    },
    removeFilter: (colId: string) => {
        const newFilters = new Map(filters);
        if (newFilters.delete(colId)) {
            setFilters(newFilters);
        }
    },
    undo: () => executeUndo(),
    redo: () => executeRedo(),
    // ### 变更记录
    // - 2026-02-16: 原因=补充行主题读 目的=提供 E2E 验证入口
    // - 2026-02-16: 原因=避免暴露内部集合; 目的=仅返回主题快
    getRowThemeForRow: (row: number) => getStaleRowTheme(row)
  }));

  // Reset cache when session/table changes
  useEffect(() => {
    cache.current.clear();
    fetching.current.clear();
    globalMerges.current.clear();
    cellToMergeMap.current.clear();
    globalStyles.current.clear();
    formulaEngine.current.clearSheet(tableName);
    setRowCount(0);
    setColumns([]);
    setRealColCount(0);
    setRealRowCount(0);
    fetchPage(1);
  }, [sessionId, tableName]);

  const onColumnResize = useCallback((_column: GridColumn, newSize: number, colIndex: number) => {
    setColumns(prevCols => {
        const newCols = [...prevCols];
        // Ensure we match the correct column, relying on index is usually safe for resize events
        // but finding by id is more robust if available
        const index = colIndex; 
        if (index >= 0 && index < newCols.length) {
            newCols[index] = { ...newCols[index], width: newSize };
        }
        return newCols;
    });
  }, []);

  const onColumnMoved = useCallback((startIndex: number, endIndex: number) => {
    setColumns(prevCols => {
        const newCols = [...prevCols];
        if (startIndex < 0 || startIndex >= newCols.length || endIndex < 0 || endIndex >= newCols.length) return prevCols;
        
        const [moved] = newCols.splice(startIndex, 1);
        newCols.splice(endIndex, 0, moved);
        return newCols;
    });
  }, []);

  const fetchPage = useCallback(
    async (page: number) => {
      if (fetching.current.has(page)) return;
      fetching.current.add(page);

      try {
        console.log(`[GlideGrid] Fetching page ${page}...`);
        
        // Build URL with Filters & Sort
        const filterParams = Array.from(filters.values());
        // ### 变更记录
        // - 2026-03-14: 原因=session_id 会导致后端报错；目的=统一归一化处理
        let url = `/api/grid-data?table_name=${tableName}&page=${page}&page_size=${PAGE_SIZE}`;
        if (normalizedSessionId) {
            url += `&session_id=${normalizedSessionId}`;
        }
        
        if (filterParams.length > 0) {
            url += `&filters=${encodeURIComponent(JSON.stringify(filterParams))}`;
        }
        if (sort) {
            url += `&sort=${encodeURIComponent(JSON.stringify(sort))}`;
        }

        // **[2026-03-12]** 变更原因：移除了 vite.config.ts 中的 Shim，直接调GridAPI
        // **[2026-03-12]** 变更目的：使得前端能够直接通过 GridAPI 访问后端 SQL 接口
        // ### �����¼
        // - 2026-03-14: ԭ��=�б�����Ҫ�ض��Ự����; Ŀ��=������ /api/grid-data��Я�� session_id����
        // - 2026-03-14: ԭ��=�ɺ�˿���ȱʧ grid-data; Ŀ��=ʧ�ܺ���˵� /api/execute��
        // - 2026-03-14: ԭ��=���� TDD �����ȶ�; Ŀ��=���л�ȡ��·�����������߼���
        let json;
        try {
            // ### �����¼
            // - 2026-03-14: ԭ��=�б������ݶ�ʧ; Ŀ��=�ԻỰά����ȡ�����������ݡ�
            const gridRes = await fetch(url);
            const gridData = await readJsonOrThrow(gridRes, "grid-data");
            if (!gridData || gridData.status === "error") {
                // - 2026-03-14: ??=??????????; ??=?????? Vite ?????
                throw new Error(`[GlideGrid] grid-data payload invalid: ${gridData?.message || "unknown"}`);
            }
            json = gridData;
        } catch (error) {
            // ### �����¼
            // - 2026-03-14: ԭ��=���ֻ����� grid-data �򱨴�; Ŀ��=������ SQL ��ȡ���ֿ��á�
            console.warn("[GlideGrid] grid-data failed, falling back to execute", error);
            json = await fetchGridData(tableName, page, PAGE_SIZE);
        }
        // ### 变更记录
        // - 2026-03-11 21:45: 原因=部分后端未提/api/grid-data; 目的=404/解析失败时回退/api/execute，保证网格可见
        if (!json || json.status === "error") {
            throw new Error(`[GlideGrid] grid-data payload invalid: ${json?.message || "unknown"}`);
        }

        // Update row count and columns from first page
        if (page === 1) {
          if (!json.columns || !Array.isArray(json.columns)) {
              console.error("[GlideGrid] Invalid columns data:", json);
              return;
          }
          // ### 变更记录
          // - 2026-02-16: 原因=同步公式列元信息; 目的=只读控制与公式栏显示
          // - 2026-02-16: 原因=后端字段可 目的=避免非数组导致异
          const formulaMeta =
              Array.isArray(json.formula_columns) ? json.formula_columns : [];
          const totalRows = json.total_rows || 0;
          setRealRowCount(totalRows);
          setRowCount(totalRows + 50); // Add 50 buffer rows

          const realCols = json.columns;
          setRealColCount(realCols.length);
          const serverTypes = Array.isArray(json.column_types) ? json.column_types : [];
          setColumnTypes(serverTypes);

          // **[2026-02-16]** 变更原因：加入自定义表头图标入口
          // **[2026-02-16]** 变更目的：验headerIcons 渲染通路
          const cols: GridColumn[] = realCols.map((c: string, index: number) => {
              const t = String(serverTypes[index] ?? "");
              const icon = index === 0 ? customHeaderIconKey : getColumnIcon(t);
              return {
                  title: getExcelColumnName(index),
                  id: c,
                  width: 150,
                  hasMenu: showFilterHeaders,
                  icon
              };
          });
          
          // Add 10 buffer columns
          for (let i = 0; i < 10; i++) {
              cols.push({
                  title: getExcelColumnName(realCols.length + i),
                  id: `__new_col_${i}`,
                  width: 150,
                  hasMenu: showFilterHeaders
              });
          }
          
          setColumns(cols);
          // ### 变更记录
          // - 2026-02-16: 原因=仅在第一页更 目的=列级信息无需分页重复
          // - 2026-02-16: 原因=保持公式列一 目的=避免状态滞
          setFormulaColumns(formulaMeta);
        }

        // Store in cache
        cache.current.set(page, {
          data: json.data || [],
          columns: json.columns || [],
          total_rows: json.total_rows || 0,
          metadata: json.metadata,
          // ### 变更记录
          // - 2026-02-16: 原因=缓存公式列元信息; 目的=页面内查询方
          // - 2026-02-16: 原因=接口字段可 目的=默认兼容
          formula_columns: Array.isArray(json.formula_columns)
              ? json.formula_columns
              : undefined
        });

        // Sync Metadata to Global Store
        if (json.metadata) {
            if (json.metadata.styles) {
                Object.entries(json.metadata.styles).forEach(([k, v]) => {
                    globalStyles.current.set(k, v as CellStyle);
                });
            }
                    if (json.metadata.merges && Array.isArray(json.metadata.merges)) {
                        rebuildMergesFromCache();
                    }
        }

        // Sync data to FormulaEngine
        if (json.data && Array.isArray(json.data)) {
            const startRow = (page - 1) * PAGE_SIZE;
            console.log(`[GlideGrid] Syncing page ${page} to FormulaEngine for ${tableName}. Rows: ${json.data.length}`);
            json.data.forEach((rowData: any[], rIdx: number) => {
                rowData.forEach((val: any, cIdx: number) => {
                    if (val !== null && val !== undefined) {
                        if (cIdx === 0 && (startRow + rIdx) === 0) {
                             console.log(`[GlideGrid] Syncing ${tableName}!A1 = ${val}`);
                        }
                        formulaEngine.current.setCellValue(cIdx, startRow + rIdx, val, tableName);
                    }
                });
            });
        }

        // Force re-render to show data
        forceUpdate({});
      } catch (err) {
        console.error("Failed to fetch page", page, err);
        try {
            const tableIdentifier = quoteSqlIdentifier(tableName);
            const offset = (page - 1) * PAGE_SIZE;
            const limit = PAGE_SIZE;
            const fallbackSql = `SELECT * FROM ${tableIdentifier} LIMIT ${limit} OFFSET ${offset}`;
            const fallbackRes = await fetch("/api/execute", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ sql: fallbackSql })
            });
            if (!fallbackRes.ok) {
                const fallbackErrorPreview = (await fallbackRes.text()).slice(0, 240);
                throw new Error(`[GlideGrid] execute fallback ${fallbackRes.status}: ${fallbackErrorPreview}`);
            }
            const parsedFallback = await parseJsonResponseSafely(fallbackRes);
            if (!parsedFallback.ok) {
                throw new Error(`[GlideGrid] execute fallback parse failed: ${parsedFallback.reason}`);
            }
            const fallbackJson = parsedFallback.data || {};
            const fallbackColumns = Array.isArray(fallbackJson.columns) ? fallbackJson.columns : [];
            const fallbackRows = Array.isArray(fallbackJson.rows) ? fallbackJson.rows : [];
            if (page === 1 && fallbackColumns.length > 0) {
                const fallbackCols: GridColumn[] = fallbackColumns.map((c: string, index: number) => ({
                    title: getExcelColumnName(index),
                    id: c,
                    width: 150,
                    hasMenu: showFilterHeaders,
                    icon: getColumnIcon("utf8")
                }));
                for (let i = 0; i < 10; i++) {
                    fallbackCols.push({
                        title: getExcelColumnName(fallbackColumns.length + i),
                        id: `__new_col_${i}`,
                        width: 150,
                        hasMenu: showFilterHeaders
                    });
                }
                setColumns(fallbackCols);
                setRealColCount(fallbackColumns.length);
                setColumnTypes([]);
                setFormulaColumns([]);
                let totalRows = offset + fallbackRows.length;
                try {
                    const countSql = `SELECT COUNT(*) AS total FROM ${tableIdentifier}`;
                    const countRes = await fetch("/api/execute", {
                        method: "POST",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({ sql: countSql })
                    });
                    if (countRes.ok) {
                        const parsedCount = await parseJsonResponseSafely(countRes);
                        if (parsedCount.ok) {
                            const countRows = Array.isArray(parsedCount.data?.rows) ? parsedCount.data.rows : [];
                            const maybeTotal = countRows?.[0]?.[0];
                            if (Number.isFinite(Number(maybeTotal))) {
                                totalRows = Number(maybeTotal);
                            }
                        }
                    }
                } catch (countErr) {
                    console.warn("[GlideGrid] fallback count query failed", countErr);
                }
                setRealRowCount(totalRows);
                setRowCount(totalRows + 50);
            }
            cache.current.set(page, {
                data: fallbackRows,
                columns: fallbackColumns,
                total_rows: offset + fallbackRows.length,
                metadata: undefined,
                formula_columns: undefined
            });
            forceUpdate({});
        } catch (fallbackErr) {
            console.error("Failed to fetch page via fallback", page, fallbackErr);
        }
      } finally {
        fetching.current.delete(page);
      }
    },
    [sessionId, tableName, rebuildMergesFromCache, filters, sort]
  );

  const onVisibleRegionChanged = useCallback((region: Rectangle) => {
    const startRow = region.y;
    const endRow = region.y + region.height;
    const startPage = Math.floor(startRow / PAGE_SIZE) + 1;
    const endPage = Math.floor(endRow / PAGE_SIZE) + 1;
    for (let page = startPage; page <= endPage; page++) {
        if (!cache.current.has(page)) {
            fetchPage(page);
        }
    }
  }, [fetchPage]);

  const onPaste = useCallback((target: Item, values: readonly (readonly string[])[]): boolean => {
      // ### 变更记录
      // - 2026-03-14: 原因=默认会话只读；目阻止粘贴写入
      if (isReadOnly) {
          alert('当前会话为只读，无法粘贴');
          return false;
      }
      if (!pasteEnvLogged.current) {
          const isSecureContext = window.isSecureContext === true;
          const hasClipboard = typeof navigator !== "undefined" && !!navigator.clipboard;
          const hasClipboardRead = !!navigator.clipboard?.read;
          const hasClipboardReadText = !!navigator.clipboard?.readText;
          const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
          const isWebView = /wv|webview|micromessenger|line|fb_iab|instagram|fbav|fban/.test(ua.toLowerCase());
          console.log("[GlideGrid] Paste env", {
              isSecureContext,
              hasClipboard,
              hasClipboardRead,
              hasClipboardReadText,
              isWebView
          });
          if (!isSecureContext || (!hasClipboardRead && !hasClipboardReadText)) {
              console.log("[GlideGrid] Clipboard API不可用，降级依赖事件剪贴板数据.");
          }
          pasteEnvLogged.current = true;
      }
      console.log("[GlideGrid] onPaste triggered");
      const data = trimPasteData(values);
      if (data.length === 0) {
          alert("粘贴失败：剪贴板无可用数");
          return true;
      }

      const noticeToken = pasteNoticeToken.current + 1;
      pasteNoticeToken.current = noticeToken;
      if (pasteNoticeTimer.current !== undefined) {
          window.clearTimeout(pasteNoticeTimer.current);
      }
      setPasteNoticeVisible(false);
      pasteNoticeTimer.current = window.setTimeout(() => {
          if (pasteNoticeToken.current === noticeToken) {
              setPasteNoticeVisible(true);
          }
      }, 2000);

      const selectionRange = selectionRef.current?.current?.range;
      const targetCol = target[0];
      const targetRow = target[1];
      const offsetCol = selectionRange ? targetCol - selectionRange.x : 0;
      const offsetRow = selectionRange ? targetRow - selectionRange.y : 0;
      const startCol = (selectionRange ? selectionRange.x : targetCol) + offsetCol;
      const startRow = (selectionRange ? selectionRange.y : targetRow) + offsetRow;
      const targetPage = Math.floor(startRow / PAGE_SIZE) + 1;

      const maxWidth = data.reduce((max, row) => Math.max(max, row.length), 0);
      const requiredColCount = startCol + maxWidth;
      const requiredTotalRows = startRow + data.length;
      const baseRealColCount = realColCount;
      const expandedRows = requiredTotalRows > realRowCount;

      if (expandedRows) {
          setRealRowCount(requiredTotalRows);
          setRowCount(requiredTotalRows + 50);
      }

      const BATCH_THRESHOLD = 50;
      const CHUNK_CELL_LIMIT = 25000;
      
      (async () => {
          try {
              // Pre-fetch pages for Undo safety to ensure we capture correct old values
              const startP = Math.floor(startRow / PAGE_SIZE) + 1;
              const endP = Math.floor((startRow + data.length - 1) / PAGE_SIZE) + 1;
              const missingPages: number[] = [];
              for (let p = startP; p <= endP; p++) {
                  if (!cache.current.has(p)) {
                      missingPages.push(p);
                  }
              }
              if (missingPages.length > 0) {
                  console.log("[GlideGrid] Pre-fetching missing pages for Undo safety:", missingPages);
                  await Promise.all(missingPages.map(p => fetchPage(p)));
              }

              let effectiveSessionId = normalizedSessionId;
              let sessionChanged = false;
              const insertedColumnNames: string[] = [];

              if (requiredColCount > baseRealColCount) {
                  const columnsToAdd = requiredColCount - baseRealColCount;
                  const baseName = `NewCol_${Date.now()}`;

                  for (let i = 0; i < columnsToAdd; i += 1) {
                      const colIdx = baseRealColCount + i;
                      const colName = `${baseName}_${i + 1}`;
                      const dataColIndex = colIdx - startCol;
                      const inferredType =
                          dataColIndex >= 0 && dataColIndex < maxWidth
                              ? inferColumnType(data, dataColIndex)
                              : "utf8";
                      try {
                          const res = await fetch("/api/insert-column", {
                              method: "POST",
                              headers: { "Content-Type": "application/json" },
                              body: JSON.stringify({
                                  table_name: tableName,
                                  session_id: effectiveSessionId,
                                  col_idx: colIdx,
                                  col_name: colName,
                                  data_type: inferredType
                              })
                          });
                          const data = await readJsonOrThrow(res, "insert-column");
                          if (data.status === "ok") {
                              if (data.session_id && data.session_id !== effectiveSessionId) {
                                  effectiveSessionId = data.session_id;
                                  sessionChanged = true;
                                  onSessionChange?.(data.session_id);
                              }
                              insertedColumnNames.push(colName);
                          } else {
                              alert("插入列失败" + data.message);
                              return;
                          }
                      } catch (e) {
                          console.error("Insert Column Error:", e);
                          alert("插入列失败");
                          return;
                      }
                  }

                  setRealColCount(requiredColCount);
              }

              const updates: { row: number, col: string, val: string }[] = [];
              const updateChunks: { row: number, col: string, val: string }[][] = [];
              const undoChanges: { row: number, col: number, oldValue: string, newValue: string, colName: string }[] = [];

              let currentChunk: { row: number, col: string, val: string }[] = [];
              let chunkCells = 0;
              let maxRow = 0;
              let skippedOutOfRange = 0;

              data.forEach((rowData, rIdx) => {
                  const rowIndex = startRow + rIdx;
                  if (rowIndex > maxRow) maxRow = rowIndex;
                  
                  rowData.forEach((cellVal, cIdx) => {
                      const colIndex = startCol + cIdx;
                      let colName = "";

                      if (colIndex < baseRealColCount) {
                          colName = columns[colIndex]?.id || getExcelColumnName(colIndex);
                      } else {
                          const insertedIndex = colIndex - baseRealColCount;
                          colName = insertedColumnNames[insertedIndex] || "";
                      }

                      if (!colName) {
                          skippedOutOfRange += 1;
                          return;
                      }

                      const newVal = cleanData(cellVal);
                      
                      // Capture Undo State
                      let oldValue = "";
                      const page = Math.floor(rowIndex / PAGE_SIZE) + 1;
                      const rIdxInPage = rowIndex % PAGE_SIZE;
                      const pageData = cache.current.get(page);
                      if (pageData && pageData.data[rIdxInPage]) {
                          oldValue = String(pageData.data[rIdxInPage][colIndex] ?? "");
                      }

                      undoChanges.push({
                          row: rowIndex,
                          col: colIndex,
                          colName: colName,
                          oldValue: oldValue,
                          newValue: newVal
                      });

                      const updateItem = {
                          row: rowIndex,
                          col: colName,
                          val: newVal
                      };
                      updates.push(updateItem);
                      currentChunk.push(updateItem);
                      chunkCells += 1;
                      if (chunkCells >= CHUNK_CELL_LIMIT) {
                          updateChunks.push(currentChunk);
                          currentChunk = [];
                          chunkCells = 0;
                      }
                  });
              });

              if (currentChunk.length > 0) {
                  updateChunks.push(currentChunk);
              }

              if (updates.length === 0) {
                  const reasonText = skippedOutOfRange > 0 ? "目标列越界" : "未生成任何更新";
                  alert(`粘贴失败：${reasonText}`);
                  console.warn("[GlideGrid] Paste ignored", {
                      skippedOutOfRange,
                      columns: columns.length,
                      startCol,
                      startRow
                  });
                  return;
              }

              // Push to Undo Stack
              if (undoChanges.length > 0) {
                  undoStack.current.push({
                      type: 'batch-update',
                      changes: undoChanges
                  });
                  redoStack.current = [];
                  notifyStackChange();
              }

              const refreshAfterFailure = () => {
                  cache.current.clear();
                  setVersion(v => v + 1);
                  if (!sessionChanged) {
                      fetchPage(1);
                      if (targetPage !== 1) {
                          fetchPage(targetPage);
                      }
                  }
              };

              if (maxRow >= rowCount || updates.length > BATCH_THRESHOLD || insertedColumnNames.length > 0 || expandedRows) {
                   console.log(`[GlideGrid] Batch Paste: ${updates.length} cells in ${updateChunks.length} chunks (limit ${CHUNK_CELL_LIMIT} cells/chunk)`);
                   try {
                       setImportProgressVisible(true);
                       setImportProgress({ completed: 0, total: updateChunks.length });
                       const maxRetry = 2;
                       const sendChunk = async (chunk: { row: number, col: string, val: string }[]) => {
                           let lastError: any;
                           for (let attempt = 0; attempt <= maxRetry; attempt += 1) {
                               try {
                                   const res = await fetch("/api/batch_update_cells", {
                                       method: "POST",
                                       headers: { "Content-Type": "application/json" },
                                       body: JSON.stringify({
                                           table_name: tableName,
                                           session_id: effectiveSessionId,
                                           updates: chunk
                                       })
                                   });
                                   const contentType = res.headers.get("content-type") || "";
                                   if (!res.ok) {
                                       const text = await res.text();
                                       throw new Error(`HTTP ${res.status}: ${text}`);
                                   }
                                   if (!contentType.includes("application/json")) {
                                       const text = await res.text();
                                       throw new Error(`NON_JSON ${res.status}: ${text}`);
                                   }
                                   const json = await res.json();
                                   if (json.status === 'ok') {
                                       return json;
                                   }
                                   throw new Error(json.message || "batch_failed");
                               } catch (e) {
                                   lastError = e;
                                   if (attempt < maxRetry) {
                                       await new Promise(r => setTimeout(r, 500 * (attempt + 1)));
                                   }
                               }
                           }
                           throw lastError;
                       };
                       for (let chunkIndex = 0; chunkIndex < updateChunks.length; chunkIndex += 1) {
                           const chunk = updateChunks[chunkIndex];
                           let json: any;
                           try {
                               json = await sendChunk(chunk);
                           } catch (e) {
                               const message = e instanceof Error ? e.message : String(e);
                               alert("粘贴失败: " + message);
                               console.error("[GlideGrid] Batch paste failed:", e);
                               refreshAfterFailure();
                               return;
                           }
                           if (json.session_id && json.session_id !== effectiveSessionId) {
                               effectiveSessionId = json.session_id;
                               sessionChanged = true;
                               onSessionChange?.(json.session_id);
                           }
                           setImportProgress(prev => ({
                               completed: Math.min(prev.completed + 1, prev.total),
                               total: prev.total
                           }));
                       }

                       if (!sessionChanged) {
                           cache.current.clear();
                           setVersion(v => v + 1);
                           fetchPage(1);
                           if (targetPage !== 1) {
                               fetchPage(targetPage);
                           }
                       }
                   } catch (e) {
                       console.error("Paste error:", e);
                       alert("粘贴失败");
                       refreshAfterFailure();
                   }
              } else {
                   console.log(`[GlideGrid] Optimistic Paste: ${updates.length} cells`);
                   updates.forEach(u => {
                       const page = Math.floor(u.row / PAGE_SIZE) + 1;
                       let pageData = cache.current.get(page);
                       if (!pageData) {
                           pageData = {
                               data: [],
                               columns: columns.map(c => c.id ?? ""),
                               total_rows: rowCount,
                               metadata: undefined
                           };
                           cache.current.set(page, pageData);
                       }
                       const rowInPage = u.row % PAGE_SIZE;
                       if (!pageData.data[rowInPage]) {
                           pageData.data[rowInPage] = [];
                       }
                       const colIndex = columns.findIndex(c => c.id === u.col);
                       if (colIndex >= 0) {
                           pageData.data[rowInPage][colIndex] = u.val;
                       }
                   });
                   forceUpdate({});
                   setVersion(v => v + 1);
                   
                   try {
                       const res = await fetch("/api/batch_update_cells", {
                           method: "POST",
                           headers: { "Content-Type": "application/json" },
                           body: JSON.stringify({
                               table_name: tableName,
                               session_id: effectiveSessionId,
                               updates: updates
                           })
                       });
                       const contentType = res.headers.get("content-type") || "";
                       if (!res.ok) {
                           const text = await res.text();
                           console.error("[GlideGrid] Optimistic paste http error:", res.status, text);
                           alert(`粘贴失败：状态码 ${res.status}`);
                           refreshAfterFailure();
                           return;
                       }
                       if (!contentType.includes("application/json")) {
                           const text = await res.text();
                           console.error("[GlideGrid] Optimistic paste non-JSON:", res.status, text);
                           alert(`粘贴失败：返回非 JSON 响应 (Status: ${res.status})`);
                           refreshAfterFailure();
                           return;
                       }
                       const json = await res.json();
                       if (json.status === 'ok') {
                           if (json.session_id && json.session_id !== effectiveSessionId) {
                               onSessionChange?.(json.session_id);
                           }
                       } else {
                           console.error("Background save failed:", json.message);
                           alert("粘贴失败: " + json.message);
                           refreshAfterFailure();
                       }
                   } catch (e) {
                       console.error("Background save failed:", e);
                       alert("粘贴失败");
                       refreshAfterFailure();
                   }
              }
          } finally {
              setImportProgressVisible(false);
              if (pasteNoticeToken.current === noticeToken) {
                  if (pasteNoticeTimer.current !== undefined) {
                      window.clearTimeout(pasteNoticeTimer.current);
                      pasteNoticeTimer.current = undefined;
                  }
                  setPasteNoticeVisible(false);
              }
          }
      })();

      return true;
  }, [columns, sessionId, tableName, rowCount, fetchPage, onSessionChange, realColCount, realRowCount]);

  // Refresh when filters or sort change
  useEffect(() => {
      cache.current.clear();
      fetching.current.clear();
      setRowCount(0);
      fetchPage(1);
  }, [filters, sort, fetchPage]);

  useEffect(() => {
      if (columnTypes.length === 0) return;
      setColumns(prev => {
          if (prev.length === 0) return prev;
          let changed = false;
          const next = prev.map((col, index) => {
              if (index >= realColCount) return col;
              const icon = getColumnIcon(String(columnTypes[index] ?? ""));
              if (col.icon === icon && col.hasMenu === showFilterHeaders) {
                  return col;
              }
              changed = true;
              return { ...col, icon, hasMenu: showFilterHeaders };
          });
          return changed ? next : prev;
      });
  }, [columnTypes, realColCount, showFilterHeaders]);

  const getCellContent = useCallback(
    (cell: Item): GridCell => {
      const [col, row] = cell;

      // Debug specific cell to trace merge logic
      if (col === 1 && row === 1) {
          // Force update version with timestamp
      }

      // Calculate page (1-based for API)
      const page = Math.floor(row / PAGE_SIZE) + 1;
      const rowIndexInPage = row % PAGE_SIZE;

      const pageData = cache.current.get(page);

      // Initialize defaults
      let rawValue = "";
      let displayStr = "";
      
      // Check cache for data
      if (pageData && pageData.data && pageData.data[rowIndexInPage]) {
          const val = pageData.data[rowIndexInPage][col];
          if (val !== undefined) {
               rawValue = val === null ? "" : String(val);
               displayStr = formulaEngine.current.calculate(rawValue, col, row, tableName);
          }
      } 
      // Else: It's a buffer cell (row >= realRowCount or col >= realColCount), default is empty ""
      // **[2026-02-26]** 变更原因：SPLIT 溢出值需补位展示
      // **[2026-02-26]** 变更目的：当无原始值且无计算结果时使用溢出映射
      if (!rawValue && !displayStr) {
          const spill = formulaEngine.current.getSpillValue(col, row, tableName);
          if (spill !== undefined) {
              displayStr = String(spill);
          }
      }

      // Apply Metadata (Styles & Merges) - APPLIES TO ALL CELLS (Real & Buffer)
       let themeOverride: any = undefined;
       let contentAlign: any = undefined;
       let span: [number, number] | undefined = undefined;
       // ### 变更记录
       // - 2026-03-14: 原因=默认会话需只读；目全局禁用编辑入口
       let readonly = isReadOnly; // Default writable unless readOnly
       // ### 变更记录
       // - 2026-02-16: 原因=公式列不允许编辑; 目的=防止单元格被覆盖
       // - 2026-02-16: 原因=仅作用真实列; 目的=避免影响缓冲
       if (row < realRowCount && col < realColCount && formulaColumnIndexSet.has(col)) {
           readonly = true;
       }
       
       const cellKey = `${row},${col}`;
       
       // 1. Handle Styles (Global Lookup)
       const style = globalStyles.current.get(cellKey);
       if (style) {
           if (!themeOverride) themeOverride = {};
           if (style.color) themeOverride.textDark = style.color;
           if (style.bg_color) {
               themeOverride.bgCell = style.bg_color;
           }
           
           let fontParts = ["13px", "sans-serif"];
           if (style.bold) fontParts.unshift("bold");
           if (style.italic) fontParts.unshift("italic");
           if (style.bold || style.italic) {
               themeOverride.baseFontStyle = fontParts.join(" ");
           }
           
           if (style.align) contentAlign = style.align;
       }

       // ### 变更记录
       // - 2026-02-16: 原因=支持单元格格 目的=统一显示格式
       // - 2026-02-16: 原因=仅影响显 目的=保留原始
       const formattedDisplay = style?.format
           ? formatCellValue(displayStr, style.format)
           : displayStr;
       // ### 变更记录
       // - 2026-03-14: 原因=公式回显存在等待; 目的=单元格内显示等待态
       // - 2026-03-14: 原因=只影响显示; 目的=不改写原始数据
       const pendingDisplay = pendingFormulaKeys.has(cellKey)
           ? "⏳ 计算中…"
           : formattedDisplay;

       // 2. Handle Merges (Global Lookup)
       const mergeKey = cellToMergeMap.current.get(cellKey);
       if (mergeKey) {
            const merge = globalMerges.current.get(mergeKey);
            if (merge) {
                const mStartRow = Number(merge.start_row);
                const mStartCol = Number(merge.start_col);
                const mEndCol = Number(merge.end_col);

                if (mStartRow === row && mStartCol === col) {
                    // Start Cell (Master)
                    span = [mStartCol, mEndCol];
                    
                    if (!themeOverride) themeOverride = {};
                    // ### 变更记录
                    // - 2026-03-14: 原因=合并背景不应固定白色；目沿用主题背景色
                    if (!themeOverride.bgCell) {
                         themeOverride.bgCell = customTheme.bgCell; 
                    }
                } else {
                    // Covered Cell
                    // Try to get Master Value (might be on another page)
                    let masterVal = "";
                    
                    // We can only get masterVal if the page is loaded in cache
                    const mPage = Math.floor(mStartRow / PAGE_SIZE) + 1;
                    const mPageData = cache.current.get(mPage);
                    if (mPageData && mPageData.data) {
                        const mRowIdx = mStartRow % PAGE_SIZE;
                        if (mPageData.data[mRowIdx]) {
                             masterVal = String(mPageData.data[mRowIdx][mStartCol] ?? "");
                        }
                    }

                    return {
                        kind: GridCellKind.Text,
                        allowOverlay: false,
                        readonly: true,
                        displayData: masterVal, 
                        data: masterVal,
                        copyData: masterVal,
                        span: undefined,
                        lastUpdated: version,
                        allowWrapping: true // Added
                    };
                }
            }
       }

       return {
            kind: GridCellKind.Text,
            allowOverlay: true,
            readonly: readonly,
            displayData: pendingDisplay,
            data: rawValue,
            // ### 变更记录
            // - 2026-03-14: 原因=等待态仅用于显示; 目的=复制仍保留真实展示值
            copyData: formattedDisplay,
            themeOverride,
            contentAlign,
            span,
            lastUpdated: version,
            allowWrapping: true // Added
       };
    },
    [fetchPage, columns, realRowCount, realColCount, version, formulaColumnIndexSet, isReadOnly, pendingFormulaKeys] 
  );

  // **[2026-02-16]** 变更原因：支持多单元格批量编辑入口
  // **[2026-02-16]** 变更目的：减少多onCellEdited 请求
  // **[2026-02-16]** 变更原因：保持与后端 batch_update_cells API 对齐
  // **[2026-02-16]** 变更目的：提升大规模编辑性能与一致性
  // **[2026-02-16]** 变更原因：需要同步更新公式元信息与依赖失效
  // **[2026-02-16]** 变更目的：确保公式列状态正确刷新
  const onCellsEdited = useCallback(
    (newValues: readonly EditListItem[]) => {
      // ### 变更记录
      // - 2026-03-14: 原因=默认会话只读；目阻止批量编辑入口
      if (isReadOnly) {
        alert('当前会话为只读，无法批量编辑');
        return false;
      }
      if (!newValues || newValues.length === 0) return true;
      // ### 变更记录
      // - 2026-03-14: 原因=批量公式不回显; 目的=预先计算涉及页
      // - 2026-03-14: 原因=批量下拉可能跨页; 目的=按页去重刷新
      const formulaPages = collectFormulaPages(newValues, PAGE_SIZE);
      // ### 变更记录
      // - 2026-03-14: 原因=公式回显存在等待; 目的=先标记等待态
      // - 2026-03-14: 原因=批量下拉可能多公式; 目的=一次性收集 key
      const pendingKeys = collectFormulaPendingKeys(newValues);
      if (pendingKeys.size > 0) {
        addPendingFormulaKeys(pendingKeys);
      }
      (async () => {
        try {
          const pages = new Set<number>();
          for (const edit of newValues) {
            const row = edit.location[1];
            if (row < 0) continue;
            pages.add(Math.floor(row / PAGE_SIZE) + 1);
          }
          const missingPages = Array.from(pages).filter(p => !cache.current.has(p));
          if (missingPages.length > 0) {
            await Promise.all(missingPages.map(p => fetchPage(p)));
          }

          // **[2026-02-16]** 变更原因：批量编辑需一次性构建更新集合
          // **[2026-02-16]** 变更目的：统一发送到 batch_update_cells
          const updates: { row: number, col: string, val: string }[] = [];
          // **[2026-02-16]** 变更原因：支持批量编辑的 Undo/Redo
          // **[2026-02-16]** 变更目的：与单元格编辑体验一致
          const undoChanges: { row: number, col: number, oldValue: string, newValue: string, colName: string }[] = [];
          // **[2026-02-16]** 变更原因：批量操作需触发列级失效
          // **[2026-02-16]** 变更目的：通知外部刷新依赖列
          const invalidatedCols = new Set<number>();
          let skippedFormula = 0;
          let skippedOutOfRange = 0;

          for (const edit of newValues) {
            const [col, row] = edit.location;
            if (col < 0 || row < 0 || col >= columns.length || row >= rowCount) {
              skippedOutOfRange += 1;
              continue;
            }
            if (formulaColumnIndexSet.has(col)) {
              skippedFormula += 1;
              continue;
            }
            if (edit.value.kind !== GridCellKind.Text) {
              continue;
            }
            const colName = columns[col]?.id;
            if (!colName) {
              skippedOutOfRange += 1;
              continue;
            }
            const page = Math.floor(row / PAGE_SIZE) + 1;
            const pageData = cache.current.get(page);
            const rowInPage = row % PAGE_SIZE;
            const oldValue = pageData?.data?.[rowInPage]?.[col];
            const raw = edit.value.data;
            const newValue = typeof raw === "string" ? raw : String(raw ?? "");
            undoChanges.push({ row, col, oldValue: String(oldValue ?? ""), newValue, colName });
            updates.push({ row, col: colName, val: newValue });
            // **[2026-02-16]** 变更原因：批量更新仍要解析公式元信息
            // **[2026-02-16]** 变更目的：保证公式栏stale 状态一致
            applyFormulaMetaFromValue(col, row, newValue);
            invalidatedCols.add(col);
            let nextPageData = pageData;
            if (!nextPageData) {
              nextPageData = {
                data: [],
                columns: columns.map(c => c.id ?? ""),
                total_rows: rowCount,
                metadata: undefined
              };
              cache.current.set(page, nextPageData);
            }
            if (!nextPageData.data[rowInPage]) {
              nextPageData.data[rowInPage] = [];
            }
            nextPageData.data[rowInPage][col] = newValue;
          }

          if (updates.length === 0) {
            if (skippedFormula > 0) {
              alert("插入公式列不允许批量编辑");
            } else if (skippedOutOfRange > 0) {
              alert("批量更新失败：目标超出范");
            }
            return;
          }

          // **[2026-02-16]** 变更原因：批量编辑完成后触发失效通知
          // **[2026-02-16]** 变更目的：让外部依赖列刷新缓存
          if (onInvalidateByColumn) {
            for (const colIndex of invalidatedCols) {
              onInvalidateByColumn(colIndex);
            }
          }

          if (undoChanges.length > 0) {
            undoStack.current.push({
              type: "batch-update",
              changes: undoChanges
            });
            redoStack.current = [];
            notifyStackChange();
          }

          forceUpdate({});
          setVersion(v => v + 1);

          const CHUNK_CELL_LIMIT = 25000;
          const updateChunks: { row: number, col: string, val: string }[][] = [];
          let currentChunk: { row: number, col: string, val: string }[] = [];
          for (const update of updates) {
            currentChunk.push(update);
            if (currentChunk.length >= CHUNK_CELL_LIMIT) {
              updateChunks.push(currentChunk);
              currentChunk = [];
            }
          }
          if (currentChunk.length > 0) {
            updateChunks.push(currentChunk);
          }

          let effectiveSessionId = normalizedSessionId;
          const refreshAfterFailure = () => {
            cache.current.clear();
            setVersion(v => v + 1);
            fetchPage(1);
          };

          // **[2026-02-16]** 变更原因：避免一次性提交过多单元格
          // **[2026-02-16]** 变更目的：降低服务端压力并保持响应
          for (const chunk of updateChunks) {
            try {
              const json = await batchUpdateCells({
                  table_name: tableName,
                  session_id: effectiveSessionId,
                  updates: chunk
              });

              if (json.status !== "ok") {
                alert("批量更新失败: " + (json.message || "batch_failed"));
                refreshAfterFailure();
                return;
              }
              if (json.session_id && json.session_id !== effectiveSessionId) {
                effectiveSessionId = json.session_id;
                onSessionChange?.(json.session_id);
              }
            } catch (e) {
              const message = e instanceof Error ? e.message : String(e);
              alert("批量更新失败: " + message);
              console.error("[GlideGrid] Batch edit failed:", e);
              refreshAfterFailure();
              return;
            }
          }

          // ### 变更记录
          // - 2026-03-14: 原因=公式结果依赖后端计算; 目的=提交后回拉计算结果
          // - 2026-03-14: 原因=批量下拉可能跨页; 目的=按涉及页去重刷新
          // - 2026-03-14: 原因=刷新失败不应阻塞批量流程; 目的=最佳努力刷新
          if (formulaPages.size > 0) {
            const refreshTasks = Array.from(formulaPages).map(async (page) => {
              cache.current.delete(page);
              await fetchPage(page);
            });
            const refreshResults = await Promise.allSettled(refreshTasks);
            for (const result of refreshResults) {
              if (result.status === "rejected") {
                console.warn("[GlideGrid] Formula page refresh failed:", result.reason);
                // ### 变更记录
                // - 2026-03-14: 原因=刷新失败缺少提示; 目的=提示用户重试
                // - 2026-03-14: 原因=批量可能包含多单元格; 目的=提示“等”
                if (pendingKeys.size > 0) {
                  const firstKey = pendingKeys.values().next().value as string | undefined;
                  if (firstKey) {
                    const [rowStr, colStr] = firstKey.split(",");
                    const rowIndex = Number(rowStr);
                    const colIndex = Number(colStr);
                    // ### 变更记录
                    // - 2026-03-14: 原因=批量提示需带“等”; 目的=保持文案一致
                    const baseMessage = buildFormulaFailureNotice(colIndex, rowIndex, columns);
                    showFormulaNotice(baseMessage.replace(" 更新失败，请重试", " 等更新失败，请重试"));
                  }
                }
              }
            }
          }

          if (skippedFormula > 0) {
            alert("部分更新被忽略：包含公式");
          } else if (skippedOutOfRange > 0) {
            alert("部分更新被忽略：目标超出范围");
          }
        } finally {
          // ### 变更记录
          // - 2026-03-14: 原因=提交/刷新可能提前返回; 目的=确保等待态清理
          // - 2026-03-14: 原因=避免卡在“计算中”; 目的=始终回到真实显示
          if (pendingKeys.size > 0) {
            clearPendingFormulaKeys(pendingKeys);
          }
        }
      })();
      return true;
    },
    [applyFormulaMetaFromValue, columns, fetchPage, formulaColumnIndexSet, isReadOnly, notifyStackChange, onInvalidateByColumn, onSessionChange, rowCount, sessionId, tableName, addPendingFormulaKeys, clearPendingFormulaKeys, showFormulaNotice]
  );

  const fillRange = useCallback(
    async (source: Rectangle, target: Rectangle) => {
      if (!source || !target) return false;
      const sourceWidth = Math.min(source.width, columns.length - source.x);
      const sourceHeight = Math.min(source.height, rowCount - source.y);
      const targetWidth = Math.min(target.width, columns.length - target.x);
      const targetHeight = Math.min(target.height, rowCount - target.y);
      if (sourceWidth <= 0 || sourceHeight <= 0 || targetWidth <= 0 || targetHeight <= 0) return false;

      const sourcePages = new Set<number>();
      for (let row = source.y; row < source.y + sourceHeight; row += 1) {
        if (row < 0 || row >= rowCount) continue;
        sourcePages.add(Math.floor(row / PAGE_SIZE) + 1);
      }
      const missingPages = Array.from(sourcePages).filter((page) => !cache.current.has(page));
      if (missingPages.length > 0) {
        await Promise.all(missingPages.map((page) => fetchPage(page)));
      }

      const sourceValues: string[][] = [];
      for (let r = 0; r < sourceHeight; r += 1) {
        const rowIndex = source.y + r;
        const page = Math.floor(rowIndex / PAGE_SIZE) + 1;
        const pageData = cache.current.get(page);
        const rowInPage = rowIndex % PAGE_SIZE;
        const rowData = pageData?.data?.[rowInPage];
        const rowValues: string[] = [];
        for (let c = 0; c < sourceWidth; c += 1) {
          const colIndex = source.x + c;
          const raw = rowData?.[colIndex];
          rowValues.push(raw !== undefined && raw !== null ? String(raw) : "");
        }
        sourceValues.push(rowValues);
      }

      const filledValues: string[][] = Array.from({ length: targetHeight }, () => Array.from({ length: targetWidth }, () => ""));
      const hasFormula = sourceValues.some((row) => row.some((val) => String(val ?? "").trim().startsWith("=")));
      if (sourceWidth === 1 && targetWidth === 1 && !hasFormula && targetHeight > sourceHeight) {
        const columnValues = sourceValues.map((row) => row[0] ?? "");
        const inferred = inferFillValues(columnValues, targetHeight);
        for (let r = 0; r < targetHeight; r += 1) {
          filledValues[r][0] = inferred[r] ?? "";
        }
      } else if (sourceHeight === 1 && targetHeight === 1 && !hasFormula && targetWidth > sourceWidth) {
        const rowValues = sourceValues[0]?.map((val) => val ?? "") ?? [];
        const inferred = inferFillValues(rowValues, targetWidth);
        for (let c = 0; c < targetWidth; c += 1) {
          filledValues[0][c] = inferred[c] ?? "";
        }
      } else {
        for (let r = 0; r < targetHeight; r += 1) {
          for (let c = 0; c < targetWidth; c += 1) {
            const baseRow = r % sourceHeight;
            const baseCol = c % sourceWidth;
            const baseValue = sourceValues[baseRow]?.[baseCol] ?? "";
            if (String(baseValue).trim().startsWith("=")) {
              filledValues[r][c] = shiftFormulaReferences(baseValue, c - baseCol, r - baseRow);
            } else {
              filledValues[r][c] = baseValue;
            }
          }
        }
      }

      const edits: EditListItem[] = [];
      for (let r = 0; r < targetHeight; r += 1) {
        const rowIndex = target.y + r;
        if (rowIndex < 0 || rowIndex >= rowCount) continue;
        for (let c = 0; c < targetWidth; c += 1) {
          const colIndex = target.x + c;
          if (colIndex < 0 || colIndex >= columns.length) continue;
          if (formulaColumnIndexSet.has(colIndex)) continue;
          const value = filledValues[r]?.[c] ?? "";
          const cell: GridCell = {
            kind: GridCellKind.Text,
            data: value,
            displayData: value,
            allowOverlay: true,
            readonly: false
          };
          edits.push({ location: [colIndex, rowIndex], value: cell });
        }
      }

      if (edits.length === 0) return false;
      return onCellsEdited(edits);
    },
    [columns.length, rowCount, fetchPage, onCellsEdited, formulaColumnIndexSet]
  );

  const onCellEdited = useCallback(
    async (cell: Item, newValue: GridCell) => {
      // ### 变更记录
      // - 2026-03-14: 原因=默认会话只读；目阻止单元格编辑入口
      if (isReadOnly) {
        alert('当前会话为只读，无法编辑');
        return;
      }
      if (newValue.kind !== GridCellKind.Text) return;

      const [col, row] = cell;
      // ### 变更记录
      // - 2026-02-16: 原因=公式列不允许编辑; 目的=阻止单元格覆
      // - 2026-02-16: 原因=用户提示; 目的=明确公式列规
      if (isFormulaColumnIndex(col, formulaColumns)) {
          alert("插入公式列不允许编辑单个单元");
          return;
      }
      const initialPage = Math.floor(row / PAGE_SIZE) + 1;
      const initialRowIndex = row % PAGE_SIZE;
      const initialPageData = cache.current.get(initialPage);
      // ### 变更记录
      // - 2026-02-15: 原因=公式计算会先写入 Calculating; 目的=保留真实旧值用Undo
      // - 2026-02-15: 原因=避免被中间态覆 目的=在写入前快照旧
      // - 2026-02-15: 原因=Undo 栈依赖旧 目的=确保旧值来自用户编辑前
      // - 2026-02-15: 原因=异步计算可能延后; 目的=避免旧值被延迟覆盖
      const initialValueSnapshot = initialPageData?.data?.[initialRowIndex]?.[col];

      const originalInput = newValue.data;
      const isFormulaInput = typeof originalInput === "string" && originalInput.trim().startsWith("=");
      let finalData = newValue.data;
      // ### 变更记录
      // - 2026-03-14: 原因=公式回显存在等待; 目的=单元格内显示“计算中”
      // - 2026-03-14: 原因=仅作用公式输入; 目的=避免普通编辑误触发
      const pendingKeySet = isFormulaInput
          ? new Set([buildFormulaKey(col, row)])
          : new Set<string>();
      if (pendingKeySet.size > 0) {
          addPendingFormulaKeys(pendingKeySet);
      }

      // ### 变更记录
      // - 2026-03-14: 原因=多余 try 导致语法错误; 目的=恢复正常语法结构
          if (typeof originalInput === "string" && originalInput.trim().startsWith("=")) {
              const parsed = parseAggregateFormula(originalInput.trim());
              const rangeInfo = getRangeInfo(parsed);
              if (rangeInfo && rangeInfo.columns.length > 0) {
                  onFormulaMetaChange?.(cell[0], cell[1], {
                      formula: originalInput.trim(),
                      columns: rangeInfo.columns,
                      stale: false,
                      lastUpdatedAt: new Date().toISOString()
                  });
              } else {
                  onFormulaMetaChange?.(cell[0], cell[1], null);
              }
          } else {
              onFormulaMetaChange?.(cell[0], cell[1], null);
          }

      // --- Backend Formula Interception ---
      const AGG_REGEX = /^=\s*(SUM|COUNT|COUNTA|AVG|AVERAGE|MAX|MIN)\s*\(\s*([A-Z]+)\s*:\s*([A-Z]+)\s*\)\s*$/i;
      const match = finalData.match(AGG_REGEX);
      if (match) {
           let func = match[1].toUpperCase();
           if (func === "AVERAGE") func = "AVG";
           if (func === "COUNTA") func = "COUNT"; // Map COUNTA to COUNT for now
           const startColStr = match[2].toUpperCase();
           const endColStr = match[3].toUpperCase();
           
           if (startColStr === endColStr) {
               const colIdx = getExcelColumnIndex(startColStr);
               const targetCol = columns[colIdx];
               
               if (targetCol) {
                   const c = col;
                   const r = row;
                   // 1. Optimistic "Calculating..."
                   const page = Math.floor(r / PAGE_SIZE) + 1;
                   const rIdx = r % PAGE_SIZE;
                   const pageData = cache.current.get(page);
                   if (pageData && pageData.data[rIdx]) {
                       pageData.data[rIdx][c] = "Calculating...";
                       setVersion(v => v + 1);
                   }

                   try {
                       const sql = `SELECT ${func}("${targetCol.id}") FROM "${tableName}"`;
                       console.log(`[GlideGrid] Backend Formula: ${finalData} -> ${sql}`);
                       const res = await fetch("/api/execute", {
                           method: "POST",
                           headers: { "Content-Type": "application/json" },
                           body: JSON.stringify({ sql })
                       });
                       const json = await readJsonOrThrow(res, "execute");
                       if (json.rows && json.rows.length > 0 && json.rows[0].length > 0) {
                           finalData = String(json.rows[0][0]);
                       } else {
                           throw new Error(json.error || "No result");
                       }
                   } catch (e) {
                       console.error("Backend formula failed", e);
                       alert("公式计算失败: " + e);
                       // Restore empty or keep formula? Let's keep formula string so user can edit it
                       // But if we proceed, we will write formula string to backend.
                       // Let's return here to avoid writing broken state?
                       // Or just fall through and write the formula string.
                       // If we write formula string, backend might accept it if col is string.
                       // If col is int, backend will error.
                       // Let's fall through.
                   }
               }
           }
      }

      onInvalidateByColumn?.(cell[0], { col: cell[0], row: cell[1] });

      // --- XLOOKUP Interception ---
      // Syntax: =XLOOKUP(lookup_value, "TargetTable", "JoinCol", "ReturnCol", [if_not_found])
      // Example: =XLOOKUP(A2, "orders", "order_id", "amount", 0)
      const XLOOKUP_REGEX = /^=XLOOKUP\(\s*([^,]+)\s*,\s*["']?([^"',]+)["']?\s*,\s*["']?([^"',]+)["']?\s*,\s*["']?([^"',]+)["']?\s*(?:,\s*([^)]+))?\s*\)$/i;
      const xMatch = finalData.match(XLOOKUP_REGEX);

      if (xMatch) {
          const lookupRef = xMatch[1].trim();
          const targetTable = xMatch[2].trim();
          const joinCol = xMatch[3].trim();
          const returnCol = xMatch[4].trim();
          const ifNotFound = xMatch[5] ? xMatch[5].trim() : "#N/A";

          // Resolve lookupRef
          let lookupValue = lookupRef;
          const cellRefMatch = lookupRef.match(/^([A-Z]+)([0-9]+)$/);
          if (cellRefMatch) {
              const refColStr = cellRefMatch[1];
              const refRowStr = cellRefMatch[2];
              const refColIdx = getExcelColumnIndex(refColStr);
              const refRowIdx = parseInt(refRowStr) - 1;

              const refPage = Math.floor(refRowIdx / PAGE_SIZE) + 1;
              const refRowInPage = refRowIdx % PAGE_SIZE;
              const refPageData = cache.current.get(refPage);
              if (refPageData && refPageData.data[refRowInPage]) {
                   const val = refPageData.data[refRowInPage][refColIdx];
                   lookupValue = val !== undefined && val !== null ? String(val) : "";
              } else {
                   console.warn("[GlideGrid] XLOOKUP reference not loaded:", lookupRef);
                   alert(`XLOOKUP 引用单元${lookupRef} 未加载或不存在`);
                   return; 
              }
          }

          // Optimistic "Loading..."
          const [c, r] = cell;
          const page = Math.floor(r / PAGE_SIZE) + 1;
          const rIdx = r % PAGE_SIZE;
          const pageData = cache.current.get(page);
          if (pageData && pageData.data[rIdx]) {
              pageData.data[rIdx][c] = "Loading...";
              setVersion(v => v + 1);
          }

          try {
              const safeValue = lookupValue.replace(/'/g, "''");
              const sql = `SELECT "${returnCol}" FROM "${targetTable}" WHERE "${joinCol}" = '${safeValue}' LIMIT 1`;
              console.log(`[GlideGrid] XLOOKUP SQL: ${sql}`);

              const res = await fetch("/api/execute", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ sql })
              });
              const json = await readJsonOrThrow(res, "execute");
              if (json.rows && json.rows.length > 0) {
                  finalData = String(json.rows[0][0]);
              } else {
                  // If ifNotFound is a string literal "...", remove quotes
                  if (ifNotFound.startsWith('"') && ifNotFound.endsWith('"')) {
                      finalData = ifNotFound.slice(1, -1);
                  } else if (ifNotFound.startsWith("'") && ifNotFound.endsWith("'")) {
                      finalData = ifNotFound.slice(1, -1);
                  } else {
                      finalData = ifNotFound;
                  }
              }
          } catch (e) {
              console.error("XLOOKUP failed", e);
              finalData = "#ERROR";
              alert("XLOOKUP 查询失败: " + e);
          }
      }

      // --- VLOOKUP Interception ---
      // Syntax: =VLOOKUP(LocalVal, TargetTable, ReturnCol, JoinCol)
      // Example: =VLOOKUP(A2, "orders", "amount", "order_id")
      const VLOOKUP_REGEX = /^=VLOOKUP\(\s*([^,]+)\s*,\s*["']?([^"',]+)["']?\s*,\s*["']?([^"',]+)["']?\s*,\s*["']?([^"',]+)["']?\s*\)$/i;
      const vMatch = finalData.match(VLOOKUP_REGEX);
      if (vMatch) {
          const lookupRef = vMatch[1].trim(); // e.g. A2 or 123
          const targetTable = vMatch[2].trim();
          const returnCol = vMatch[3].trim();
          const joinCol = vMatch[4].trim();

          // Resolve lookupRef
          let lookupValue = lookupRef;
          const cellRefMatch = lookupRef.match(/^([A-Z]+)([0-9]+)$/);
          if (cellRefMatch) {
              const refColStr = cellRefMatch[1];
              const refRowStr = cellRefMatch[2];
              const refColIdx = getExcelColumnIndex(refColStr);
              const refRowIdx = parseInt(refRowStr) - 1; // 1-based to 0-based

              // Check if loaded in cache
              const refPage = Math.floor(refRowIdx / PAGE_SIZE) + 1;
              const refRowInPage = refRowIdx % PAGE_SIZE;
              const refPageData = cache.current.get(refPage);
              if (refPageData && refPageData.data[refRowInPage]) {
                   // Ensure we get a string
                   const val = refPageData.data[refRowInPage][refColIdx];
                   lookupValue = val !== undefined && val !== null ? String(val) : "";
              } else {
                   // Fallback for not loaded cells - simplified
                   console.warn("[GlideGrid] VLOOKUP reference not loaded:", lookupRef);
                   // We proceed with lookupValue as literal if not found? 
                   // No, if it looks like A2 but not found, it's likely empty or unloaded.
                   // Let's alert.
                   alert(`VLOOKUP 引用单元${lookupRef} 未加载或不存在`);
                   return; 
              }
          }

          // Optimistic "Loading..."
          const [c, r] = cell;
          const page = Math.floor(r / PAGE_SIZE) + 1;
          const rIdx = r % PAGE_SIZE;
          const pageData = cache.current.get(page);
          if (pageData && pageData.data[rIdx]) {
              pageData.data[rIdx][c] = "Loading...";
              setVersion(v => v + 1);
          }

          try {
              // Construct SQL
              const safeValue = lookupValue.replace(/'/g, "''");
              const sql = `SELECT "${returnCol}" FROM "${targetTable}" WHERE "${joinCol}" = '${safeValue}' LIMIT 1`;
              console.log(`[GlideGrid] VLOOKUP SQL: ${sql}`);

              const res = await fetch("/api/execute", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ sql })
              });
              const json = await readJsonOrThrow(res, "execute");
              if (json.rows && json.rows.length > 0) {
                  finalData = String(json.rows[0][0]);
              } else {
                  finalData = "#N/A";
              }
          } catch (e) {
              console.error("VLOOKUP failed", e);
              finalData = "#ERROR";
              alert("VLOOKUP 查询失败: " + e);
          }
      }
      // ------------------------------------

      const colName = columns[col]?.id || `col_${col}`;
      
      const page = Math.floor(row / PAGE_SIZE) + 1;
      const rowIndexInPage = row % PAGE_SIZE;
      
      // ### 变更记录
      // - 2026-02-15: 原因=Undo 需要真实旧 目的=使用进入编辑前快
      // - 2026-02-15: 原因=支持公式生效后撤销; 目的=避免旧值被计算中间态替
      // - 2026-02-15: 原因=减少对缓存时序依 目的=稳定 Undo 结果
      let oldValue = initialValueSnapshot !== undefined && initialValueSnapshot !== null
          ? String(initialValueSnapshot)
          : "";
      // Optimistic update
      const pageData = cache.current.get(page);
      if (pageData) {
          // Ensure row exists
          if (!pageData.data[rowIndexInPage]) {
              pageData.data[rowIndexInPage] = [];
          }
          // ### 变更记录
          // - 2026-02-15: 原因=避免覆盖快照旧 目的=确保 Undo 还原真实旧
          // - 2026-02-15: 原因=缓存已被 Calculating 改写; 目的=优先使用快照
          // - 2026-02-15: 原因=保证 Undo 与用户期望一 目的=防止回退到中间
          if (initialValueSnapshot === undefined || initialValueSnapshot === null) {
              oldValue = String(pageData.data[rowIndexInPage][col] ?? "");
          }
          pageData.data[rowIndexInPage][col] = finalData; // Update cache immediately
          
          // Sync to FormulaEngine immediately for optimistic updates
          console.log(`[GlideGrid] Optimistic update: ${colName}${row+1} = ${finalData} (${tableName})`);
          formulaEngine.current.setCellValue(col, row, finalData, tableName);
          setVersion(v => v + 1); // Force update version with timestamp

          // Add to Undo Stack
          undoStack.current.push({
              type: 'cell-update',
              row,
              col,
              colName,
              oldValue,
              newValue: finalData
          });
          // Clear Redo Stack
          redoStack.current = [];
          notifyStackChange();
      }

      // Call backend API
      try {
          // Note: Backend expects col_idx, row_idx, new_value
          // row_idx is absolute
          console.log(`[GlideGrid] Updating cell: Row ${row}, Col ${col} (${colName}) -> ${finalData}`);
          
          const res = await fetch("/api/update_cell", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                  session_id: normalizedSessionId,
                  table_name: tableName,
                  row_idx: row,
                  col_idx: col,
                  col_name: colName,
                  old_value: oldValue,
                  new_value: finalData
              })
          });
          
          if (!res.ok) {
              const text = await res.text();
              let message = text;
              let code = "";
              try {
                  const errJson = JSON.parse(text);
                  message = errJson.error_message || errJson.message || text;
                  code = errJson.error_code || "";
              } catch (_) {}
              throw new Error(code ? `${code}: ${message}` : message);
          }
          
          const data = await readJsonOrThrow(res, "insert-column");
          if (data.status === 'ok') {
               console.log("[GlideGrid] Update success", data);
               // Check for session fork
               if (data.session_id && data.session_id !== sessionId) {
                   console.log(`[GlideGrid] Session forked: ${sessionId} -> ${data.session_id}`);
                   if (onSessionChange) {
                       onSessionChange(data.session_id);
                   }
               }
               // Formula cells sometimes need a fresh page pull to reflect computed value
               if (isFormulaInput) {
                   cache.current.delete(page);
                   // ### 变更记录
                   // - 2026-03-14: 原因=等待态需覆盖到刷新完成; 目的=等待拉取结束
                   // - 2026-03-14: 原因=避免瞬间取消等待; 目的=回显后再恢复
                   await fetchPage(page);
               }
          } else {
               const message = data.error_message || data.message || "Unknown error";
               const code = data.error_code || "";
               throw new Error(code ? `${code}: ${message}` : message);
          }
      } catch (e) {
          console.error("[GlideGrid] Update failed:", e);
          const errorMessage = e instanceof Error ? e.message : String(e);
          alert(`更新失败: ${errorMessage}`);
          // ### 变更记录
          // - 2026-03-14: 原因=公式更新失败缺少提示; 目的=提示用户重试
          // - 2026-03-14: 原因=提示文案需统一; 目的=复用 buildFormulaFailureNotice
          if (isFormulaInput) {
              showFormulaNotice(buildFormulaFailureNotice(col, row, columns));
          }
          // Revert? For POC, simple alert is enough.
      } finally {
          // ### 变更记录
          // - 2026-03-14: 原因=提交/刷新可能失败或提前返回; 目的=确保清理等待态
          // - 2026-03-14: 原因=避免卡在“计算中”; 目的=始终回到真实显示
          if (pendingKeySet.size > 0) {
              clearPendingFormulaKeys(pendingKeySet);
          }
      }
    },
    [sessionId, tableName, columns, isReadOnly, notifyStackChange, formulaColumns, fetchPage, addPendingFormulaKeys, clearPendingFormulaKeys]
  );

  const onGridSelectionChange = useCallback((newSelection: GridSelection) => {
      const normalizedSelection = normalizeGridSelection(newSelection) ?? newSelection;
      
      let finalSelection = normalizedSelection;

      if (normalizedSelection.current) {
          const { cell } = normalizedSelection.current;
          const [col, row] = cell;
          
          // const pageData = cache.current.get(page); // No longer needed for merge lookup

          const mergeKey = cellToMergeMap.current.get(`${row},${col}`);
          if (mergeKey) {
              const merge = globalMerges.current.get(mergeKey);
              if (merge) {
                  console.log("[GlideGrid] Snapping selection to merge range:", merge);
                  finalSelection = {
                      ...normalizedSelection,
                      current: {
                          ...normalizedSelection.current,
                          // Set anchor cell to start of merge
                          cell: [merge.start_col, merge.start_row],
                          // Set range to cover the merge
                          range: {
                              x: merge.start_col,
                              y: merge.start_row,
                              width: merge.end_col - merge.start_col + 1,
                              height: merge.end_row - merge.start_row + 1
                          },
                          rangeStack: [] // Range Stack
                      }
                  };
              }
          }
      }

      setSelection(finalSelection);
      selectionRef.current = finalSelection;
      
      // [Callback] Notify parent of selection change
      if (finalSelection.current) {
          const { cell } = finalSelection.current;
          const [col, row] = cell;
          
          // Calculate page for value retrieval
          const page = Math.floor(row / PAGE_SIZE) + 1;
          const rowIndexInPage = row % PAGE_SIZE;
          const pageData = cache.current.get(page);
          
          let val = "";
          let colName = "";
          
          if (columns[col]) {
              colName = columns[col].id || "";
          }
          
          if (pageData && pageData.data[rowIndexInPage]) {
              val = String(pageData.data[rowIndexInPage][col] ?? "");
          }
          // ### 变更记录
          // - 2026-02-16: 原因=公式列选中时显raw; 目的=公式栏展示原始表达式
          // - 2026-02-16: 原因=非公式列保持原 目的=避免误替
          val = getFormulaColumnDisplayValue(col, formulaColumns, val);
          
          if (onSelectionChange) {
              onSelectionChange(col, row, val, colName);
          }
      }
  }, [columns, onSelectionChange, formulaColumns]);

  const provideEditor = useCallback<ProvideEditorCallback<GridCell>>((cell) => {
    // ### 变更记录
    // - 2026-03-12: 原因=用户反馈点击单元格无法编 目的=普通文本回退官方默认编辑器，避免自定义编辑器拦截提交流程
    // - 2026-03-12: 原因=保留公式编辑能力; 目的=仅当输入'=' 开头时启用 FormulaEditor
    if (cell.kind === GridCellKind.Text && typeof cell.data === "string" && cell.data.trim().startsWith("=")) {
      return {
        editor: FormulaEditor as any,
      };
    }
    return undefined;
  }, []);

  const onHeaderMenuClick = useCallback((col: number, bounds: any) => {
      if (!gridWrapperRef.current) return;
      const gridRect = gridWrapperRef.current.getBoundingClientRect();
      
      console.log("[GlideGrid] Menu Click - Bounds:", bounds, "GridRect:", gridRect);

      let finalX = bounds.x;
      // We want menu to be at Grid Top + Header Height (approx bounds.y + bounds.height)
      let finalY = bounds.y + bounds.height;
      
      // Robust check: If bounds.y is significantly smaller than gridRect.top (e.g. local coord vs global)
      // If bounds.y < gridRect.top - 20, it implies bounds are local to the grid container
      if (bounds.y < gridRect.top - 20) {
          finalX = gridRect.left + bounds.x;
          finalY = gridRect.top + bounds.y + bounds.height;
      }

      console.log("[GlideGrid] Final Menu Pos:", { x: finalX, y: finalY });

      setFilterMenuTarget({ 
          x: finalX, 
          y: finalY, 
          colIndex: col 
      });
      setFilterMenuOpen(true);
      setContextMenuOpen(false); // Close other menus
  }, []);

  // ### 变更记录
  // - 2026-02-15: 原因=为过期公式单元格添加角标; 目的=提示需要刷
  // - 2026-02-15: 原因=避免影响原有内容绘制; 目的=drawContent 再绘制角
  // - 2026-02-15: 原因=遵循全局公式元信 目的=formulaMetaMap stale 状态为
  const drawCell = useCallback<DrawCellCallback>(
      (args, drawContent) => {
          drawContent();
          const key = buildFormulaKey(args.col, args.row);
          const isStale = formulaMetaMap?.get(key)?.stale === true;
          if (!isStale) return;
          const { ctx, rect } = args;
          const size = 8;
          ctx.save();
          ctx.fillStyle = "#ef4444";
          ctx.beginPath();
          ctx.moveTo(rect.x + rect.width - size, rect.y + 1);
          ctx.lineTo(rect.x + rect.width - 1, rect.y + 1);
          ctx.lineTo(rect.x + rect.width - 1, rect.y + size);
          ctx.closePath();
          ctx.fill();
          ctx.restore();
      },
      [formulaMetaMap]
  );

  useEffect(() => {
    if (filterMenuOpen || contextMenuOpen) {
      const handleGlobalClick = () => {
         setFilterMenuOpen(false);
         setContextMenuOpen(false);
      };
      // Delay to avoid immediate closing from the triggering click
      const timer = setTimeout(() => window.addEventListener('click', handleGlobalClick), 0);
      return () => {
          clearTimeout(timer);
          window.removeEventListener('click', handleGlobalClick);
      };
    }
  }, [filterMenuOpen, contextMenuOpen]);

  const onHeaderContextMenu = useCallback((colIndex: number, event: HeaderClickedEventArgs) => {
      event.preventDefault();
      setContextMenuTarget({ x: event.localEventX, y: event.localEventY, colIndex });
      setContextMenuOpen(true);
      setFilterMenuOpen(false); // Close other menus
  }, []);

  const handleInsertColumnBefore = async () => {
      if (contextMenuTarget) {
          const colIdx = contextMenuTarget.colIndex;
          const colName = `NewCol_${Date.now()}`;
          // **[2026-02-15]** 变更原因：新增列时允许输入默认公式
          // **[2026-02-15]** 变更目的：为列提供可编辑的默认公式入口，支持覆盖
          // **[2026-02-15]** 变更说明：空值视为无默认公式，避免错误写入
          const defaultFormulaInput = window.prompt("请输入新列默认公式（可为空）", "");
          // **[2026-02-15]** 变更原因：统一去除前后空白，降低用户输入差异
          const normalizedDefaultFormula = defaultFormulaInput?.trim();
          console.log(`[GlideGrid] Inserting column at ${colIdx}`);
          try {
              const res = await fetch("/api/insert-column", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      session_id: normalizedSessionId,
                      col_idx: colIdx,
                      col_name: colName,
                      // **[2026-02-15]** 变更目的：有默认公式时强制使Utf8
                      // **[2026-02-15]** 变更说明：公式字符串需要可存储类型
                      data_type: normalizedDefaultFormula ? "utf8" : undefined,
                      // **[2026-02-15]** 变更目的：传递列默认公式用于后端持久化与填充
                      default_formula: normalizedDefaultFormula || undefined
                  })
              });
              const data = await readJsonOrThrow(res, "insert-column");
              if (data.status === 'ok') {
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
              } else {
                  alert("Insert Column Failed: " + (data.message || data.error || "unknown error"));
              }
          } catch (e) {
              console.error("Insert Column Error:", e);
              alert("Insert Column Failed");
          }
      }
      setContextMenuOpen(false);
  };

  const handleInsertColumnAfter = async () => {
      if (contextMenuTarget) {
          const colIdx = contextMenuTarget.colIndex + 1;
          const colName = `NewCol_${Date.now()}`;
          // **[2026-02-15]** 变更原因：新增列时允许输入默认公式
          // **[2026-02-15]** 变更目的：为列提供可编辑的默认公式入口，支持覆盖
          // **[2026-02-15]** 变更说明：空值视为无默认公式，避免错误写入
          const defaultFormulaInput = window.prompt("请输入新列默认公式（可为空）", "");
          // **[2026-02-15]** 变更原因：统一去除前后空白，降低用户输入差异
          const normalizedDefaultFormula = defaultFormulaInput?.trim();
          console.log(`[GlideGrid] Inserting column at ${colIdx}`);
          try {
              const res = await fetch("/api/insert-column", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      session_id: normalizedSessionId,
                      col_idx: colIdx,
                      col_name: colName,
                      // **[2026-02-15]** 变更目的：有默认公式时强制使Utf8
                      // **[2026-02-15]** 变更说明：公式字符串需要可存储类型
                      data_type: normalizedDefaultFormula ? "utf8" : undefined,
                      // **[2026-02-15]** 变更目的：传递列默认公式用于后端持久化与填充
                      default_formula: normalizedDefaultFormula || undefined
                  })
              });
              const data = await readJsonOrThrow(res, "insert-column");
              if (data.status === 'ok') {
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
              } else {
                  alert("Insert Column Failed: " + (data.message || data.error || "unknown error"));
              }
          } catch (e) {
              console.error("Insert Column Error:", e);
              alert("Insert Column Failed");
          }
      }
      setContextMenuOpen(false);
  };

  // ### 变更记录
  // - 2026-02-16: 原因=新增公式列入 目的=从表头菜单触
  // - 2026-02-16: 原因=避免误操 目的=弹窗确认与输
  const handleOpenFormulaColumnDialog = () => {
      if (contextMenuTarget) {
          setFormulaColumnTargetIndex(contextMenuTarget.colIndex + 1);
      } else {
          setFormulaColumnTargetIndex(null);
      }
      // **[2026-02-16]** 变更原因：插入与编辑共用弹窗
      // **[2026-02-16]** 变更目的：打开时明确设置为插入模式
      setFormulaColumnDialogMode("insert");
      setFormulaColumnName("");
      setFormulaColumnInput("");
      setFormulaColumnError("");
      // **[2026-02-17]** 变更原因：弹窗重置示例状态
      // **[2026-02-17]** 变更目的：避免上次展开影响当前操作
      setFormulaSampleOpen(false);
      setFormulaColumnDialogOpen(true);
      setContextMenuOpen(false);
  };

  // **[2026-02-16]** 变更原因：需要支持公式列修改
  // **[2026-02-16]** 变更目的：复用公式列弹窗并回填内容
  const handleOpenFormulaColumnEditDialog = () => {
      if (!contextMenuTarget) {
          return;
      }
      const targetIndex = contextMenuTarget.colIndex;
      const meta = formulaColumns.find((item) => item.index === targetIndex);
      if (!meta) {
          setContextMenuOpen(false);
          return;
      }
      setFormulaColumnTargetIndex(targetIndex);
      setFormulaColumnDialogMode("edit");
      setFormulaColumnName(meta.name);
      setFormulaColumnInput(meta.raw_expression);
      setFormulaColumnError("");
      // **[2026-02-17]** 变更原因：编辑模式也需清理示例弹层
      // **[2026-02-17]** 变更目的：确保显示一致性
      setFormulaSampleOpen(false);
      setFormulaColumnDialogOpen(true);
      setContextMenuOpen(false);
  };

  // ### 变更记录
  // - 2026-02-17: 原因=算术公式校验需要列类型; 目的=统一取类型入
  // - 2026-02-17: 原因=后端可能缺失类型; 目的=允许前端推断兜底
  const getColumnTypeForValidation = (colIndex: number): string => {
      const serverType = String(columnTypes[colIndex] ?? "");
      if (serverType) return serverType;
      const pageData = cache.current.get(1);
      if (pageData?.data && Array.isArray(pageData.data)) {
          return inferColumnType(pageData.data, colIndex);
      }
      return "utf8";
  };
  // ### 变更记录
  // - 2026-02-17: 原因=输入检测聚合函 目的=即时提示不支
  // - 2026-02-17: 原因=统一检测入 目的=提交前复用判
  const detectAggregateFunction = (rawExpression: string): string | null => {
      if (typeof rawExpression !== "string") return null;
      const match = rawExpression.toUpperCase().match(aggregateFunctionRegex);
      return match ? match[1] : null;
  };

  // ### 变更记录
  // - 2026-02-16: 原因=生成公式列默认公式标 目的=与后端存储协议一
  // - 2026-02-16: 原因=校验表达 目的=避免无效公式写入
  const handleConfirmFormulaColumn = async () => {
      // ### 变更记录
      // - 2026-02-17: 原因=算术公式需要标准化; 目的=保证列名一致
      // - 2026-02-17: 原因=非算术保持原 目的=避免改变 IF 类表达式
      const normalizedArithmetic = normalizeArithmeticFormula(formulaColumnInput);
      const rawExpression = normalizedArithmetic ?? formulaColumnInput.trim();
      // ### 变更记录
      // - 2026-02-17: 原因=聚合函数不支持公式列; 目的=提交前明确提
      // - 2026-02-17: 原因=统一提示文案; 目的=与输入提示一
      const aggregateHit = detectAggregateFunction(rawExpression);
      if (aggregateHit) {
          setFormulaColumnError(aggregateUnsupportedMessage);
          return;
      }
      const columnIds = columns.slice(0, realColCount).map((col) => String(col.id ?? ""));
      const marker = buildFormulaColumnMarker(rawExpression, columnIds);
      if (!marker) {
          setFormulaColumnError("公式格式不合法，请使用类A+B 的表达式");
          return;
      }
      // ### 变更记录
      // - 2026-02-17: 原因=算术公式需校验列类 目的=阻止非数值列参与加减
      // - 2026-02-17: 原因=非算术公式跳 目的=符合 IF 等函数需
      const arithmeticColumns = extractArithmeticFormulaColumns(formulaColumnInput);
      const arithmeticIndexes = getArithmeticFormulaColumnIndexes(
          formulaColumnInput,
          columnIds.length
      );
      if (arithmeticIndexes && arithmeticColumns) {
          const nonNumericColumns: string[] = [];
          for (const name of arithmeticColumns) {
              const colIndex = getExcelColumnIndex(name);
              if (colIndex < 0 || colIndex >= columnIds.length) {
                  continue;
              }
              const inferredType = getColumnTypeForValidation(colIndex);
              if (!isNumericColumnType(inferredType)) {
                  nonNumericColumns.push(name);
              }
          }
          if (nonNumericColumns.length > 0) {
              setFormulaColumnError(`算术公式涉及非数值列: ${nonNumericColumns.join(", ")}`);
              return;
          }
      }
      // **[2026-02-16]** 变更原因：空字符串会被后端识别为无效会话
      // **[2026-02-16]** 变更目的：仅在有值时传session_id
      const normalizedSessionId = sessionId?.trim() ? sessionId : undefined;
      // **[2026-02-16]** 变更原因：编辑模式不走插入接口
      // **[2026-02-16]** 变更目的：调用更新公式列接口并刷新视图
      if (formulaColumnDialogMode === "edit") {
          if (formulaColumnTargetIndex === null) {
              setFormulaColumnError("无法确定修改");
              return;
          }
          try {
              // **[2026-02-16]** 变更原因：编辑公式列需要后端更新
              // **[2026-02-16]** 变更目的：调用更新接口持久化公式标记
              const res = await fetch("/api/update-column-formula", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      // **[2026-02-16]** 变更原因：空会话会导致后端报错
                      // **[2026-02-16]** 变更目的：编辑时保持与插入逻辑一致
                      session_id: normalizedSessionId,
                      col_idx: formulaColumnTargetIndex,
                      formula_raw: marker.raw,
                      formula_sql: marker.sql
                  })
              });
              const data = await readJsonOrThrow(res, "update-column-formula");
              if (data.status === 'ok') {
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
                  setFormulaColumnDialogOpen(false);
                  setFormulaColumnName("");
                  setFormulaColumnInput("");
                  setFormulaColumnError("");
                  // **[2026-02-17]** 变更原因：提交完成后关闭示例列表
                  // **[2026-02-17]** 变更目的：避免残留状态影响后续操作
                  setFormulaSampleOpen(false);
                  // **[2026-02-16]** 变更原因：编辑完成后回到插入模式
                  // **[2026-02-16]** 变更目的：避免下次打开状态错误
                  setFormulaColumnDialogMode("insert");
              } else {
                  setFormulaColumnError(data.message || "更新公式列失败");
              }
          } catch (e) {
              console.error("Update Formula Column Error:", e);
              setFormulaColumnError("更新公式列失");
          }
          return;
      }
      if (formulaColumnTargetIndex === null) {
          setFormulaColumnError("无法确定插入位置");
          return;
      }
      // ### 变更记录
      // - 2026-02-16: 原因=公式列必须有列名; 目的=保证列可识别
      // - 2026-02-16: 原因=复用校验逻辑; 目的=统一列名规范
      const normalizedName = validateFormulaColumnName(formulaColumnName);
      if (!normalizedName) {
          setFormulaColumnError("请输入列");
          return;
      }
      const colName = normalizedName;
      try {
          const res = await fetch("/api/insert-column", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                  table_name: tableName,
                  // **[2026-02-16]** 变更原因：空会话会导致后端报错
                  // **[2026-02-16]** 变更目的：插入公式列时允许无会话执行
                  session_id: normalizedSessionId,
                  col_idx: formulaColumnTargetIndex,
                  col_name: colName,
                  // **[2026-02-16]** 变更目的：公式列以默认公式标记持久化
                  // **[2026-02-16]** 变更说明：后端解JSON marker 后生成计算列
                  data_type: "utf8",
                  default_formula: JSON.stringify(marker)
              })
          });
          const data = await readJsonOrThrow(res, "insert-column");
          if (data.status === 'ok') {
              if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                  onSessionChange(data.session_id);
              } else {
                  cache.current.clear();
                  setVersion(v => v + 1);
                  fetchPage(1);
              }
              setFormulaColumnDialogOpen(false);
              setFormulaColumnName("");
              setFormulaColumnInput("");
              setFormulaColumnError("");
              // **[2026-02-17]** 变更原因：插入完成后关闭示例列表
              // **[2026-02-17]** 变更目的：保持弹窗状态清理一致
              setFormulaSampleOpen(false);
          } else {
              setFormulaColumnError(data.message || "插入公式列失败");
          }
      } catch (e) {
          console.error("Insert Formula Column Error:", e);
          setFormulaColumnError("插入公式列失");
      }
  };

  const handleDeleteColumn = async () => {
      if (contextMenuTarget) {
          const colIdx = contextMenuTarget.colIndex;
          if (!confirm(`确定要删除第 ${colIdx + 1} 列吗?`)) {
              setContextMenuOpen(false);
              return;
          }
          console.log(`[GlideGrid] Deleting column ${colIdx}`);
          try {
              const res = await fetch("/api/delete-column", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      session_id: normalizedSessionId,
                      col_idx: colIdx
                  })
              });
              const data = await readJsonOrThrow(res, "delete-column");
              if (data.status === 'ok') {
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
              } else {
                  alert("Delete Column Failed: " + (data.message || data.error || "unknown error"));
              }
          } catch (e) {
              console.error("Delete Column Error:", e);
              alert("Delete Column Failed");
          }
      }
      setContextMenuOpen(false);
  };

  const handleRefreshFormulaInContext = () => {
      if (contextMenuTarget && contextMenuTarget.rowIndex !== undefined) {
          if (canRefreshFromContext) {
              onRefreshFormula?.({ col: contextMenuTarget.colIndex, row: contextMenuTarget.rowIndex });
          }
      }
      setContextMenuOpen(false);
  };

  const handleRowHeight = () => {
      if (contextMenuTarget && contextMenuTarget.rowIndex !== undefined) {
          const rowIdx = contextMenuTarget.rowIndex;
          const currentHeight = rowSizes.get(rowIdx) || 35; // Default 35
          const newHeightStr = prompt("设置行高:", currentHeight.toString());
          if (newHeightStr !== null) {
              const newHeight = parseInt(newHeightStr, 10);
              if (!isNaN(newHeight) && newHeight > 10) {
                    setRowSizes((prev: Map<number, number>) => {
                        const next = new Map(prev);
                        next.set(rowIdx, newHeight);
                        return next;
                    });
                }
          }
      }
      setContextMenuOpen(false);
  };

  const handleClearColumnContent = () => {
      alert("清除内容 - 暂未实现后端接口");
      setContextMenuOpen(false);
  };

  const handleFreezeColumn = () => {
      if (contextMenuTarget) {
          setFreezeColumns(contextMenuTarget.colIndex + 1);
          setContextMenuOpen(false);
      }
  };

  const handleUnfreeze = () => {
      setFreezeColumns(0);
      setContextMenuOpen(false);
  };

  // Added: Row Operations Handlers
  const handleInsertRowAbove = async () => {
      if (contextMenuTarget && contextMenuTarget.rowIndex !== undefined) {
          const rowIdx = contextMenuTarget.rowIndex;
          console.log(`[GlideGrid] Inserting row above ${rowIdx}`);
          try {
              const res = await fetch("/api/insert-row", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      session_id: normalizedSessionId,
                      row_idx: rowIdx
                  })
              });
              const data = await readJsonOrThrow(res, "insert-row");
              if (data.status === 'ok') {
                  // If session changed, notify
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      // Just refresh
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
              } else {
                  alert("Insert Row Failed: " + (data.message || data.error || "unknown error"));
              }
          } catch (e) {
              console.error("Insert Row Error:", e);
              alert("Insert Row Failed");
          }
      }
      setContextMenuOpen(false);
  };

  const handleInsertRowBelow = async () => {
      if (contextMenuTarget && contextMenuTarget.rowIndex !== undefined) {
          const rowIdx = contextMenuTarget.rowIndex + 1;
          console.log(`[GlideGrid] Inserting row below ${contextMenuTarget.rowIndex} (at ${rowIdx})`);
          try {
              const res = await fetch("/api/insert-row", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      session_id: normalizedSessionId,
                      row_idx: rowIdx
                  })
              });
              const data = await readJsonOrThrow(res, "insert-row");
              if (data.status === 'ok') {
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
              } else {
                  alert("Insert Row Failed: " + (data.message || data.error || "unknown error"));
              }
          } catch (e) {
              console.error("Insert Row Error:", e);
              alert("Insert Row Failed");
          }
      }
      setContextMenuOpen(false);
  };

  const handleDeleteRow = async () => {
      if (contextMenuTarget && contextMenuTarget.rowIndex !== undefined) {
          const rowIdx = contextMenuTarget.rowIndex;
          if (!confirm(`确定要删除第 ${rowIdx + 1} 行吗?`)) {
              setContextMenuOpen(false);
              return;
          }
          console.log(`[GlideGrid] Deleting row ${rowIdx}`);
          try {
              const res = await fetch("/api/delete-row", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                      table_name: tableName,
                      session_id: normalizedSessionId,
                      row_idx: rowIdx
                  })
              });
              const data = await readJsonOrThrow(res, "delete-row");
              if (data.status === 'ok') {
                  if (data.session_id && data.session_id !== sessionId && onSessionChange) {
                      onSessionChange(data.session_id);
                  } else {
                      cache.current.clear();
                      setVersion(v => v + 1);
                      fetchPage(1);
                  }
              } else {
                  alert("Delete Row Failed: " + (data.message || data.error || "unknown error"));
              }
          } catch (e) {
              console.error("Delete Row Error:", e);
              alert("Delete Row Failed");
          }
      }
      setContextMenuOpen(false);
  };

  const handleSort = (order: "asc" | "desc") => {
      if (filterMenuTarget) {
          const colId = columns[filterMenuTarget.colIndex]?.id;
          if (colId) {
              setSort({ col: colId, order });
          }
      }
      setFilterMenuOpen(false);
  };

  const handleClearFilter = () => {
      if (filterMenuTarget) {
          const colId = columns[filterMenuTarget.colIndex]?.id;
          if (colId) {
              const newFilters = new Map(filters);
              newFilters.delete(colId);
              setFilters(newFilters);
          }
      }
      setFilterMenuOpen(false);
  };

  const handleFilterConfirm = () => {
      if (filterMenuTarget) {
          const colId = columns[filterMenuTarget.colIndex]?.id;
          if (colId) {
              const newFilters = new Map(filters);
              // Apply selected values as filter
              newFilters.set(colId, { col: colId, val: Array.from(filterSelected), op: "in" });
              setFilters(newFilters);
              
              const payload: FilterApplyPayload = {
                  colIndex: filterMenuTarget.colIndex,
                  columnId: colId,
                  selectedValues: Array.from(filterSelected),
                  searchText: filterSearchText,
              };
              onFilterApply?.(payload);
          }
      }
      setFilterMenuOpen(false);
  };

  const customTheme: Partial<Theme> = {
    // Base Colors
    bgCell: "#0a0f1c",
    bgHeader: "#0f172a",
    bgHeaderHasFocus: "#1e293b",
    bgHeaderHovered: "#1e293b",
    
    // Text Colors
    textDark: "#e5e7eb", 
    textMedium: "#94a3b8", 
    textLight: "#ffffff", 
    textHeader: "#94a3b8",
    textHeaderSelected: "#5ce1ff",
    
    // Borders & Lines
    borderColor: "rgba(148, 163, 184, 0.2)",
    drilldownBorder: "rgba(148, 163, 184, 0.2)",
    
    // Selection & Accent
    accentColor: "#5ce1ff", 
    accentLight: "rgba(92, 225, 255, 0.1)", 
    accentFg: "#5ce1ff",
    
    // Other
    linkColor: "#5ce1ff",
    headerFontStyle: "600 13px 'Inter', sans-serif",
    baseFontStyle: "13px 'Inter', sans-serif",
    fontFamily: "'Inter', sans-serif",
    editorFontSize: "13px",
    lineHeight: 1.4,
  };


  if (columns.length === 0) {
      return <div>Loading Grid Metadata...</div>;
  }

  const progressPercent = importProgress.total > 0
      ? Math.round((importProgress.completed / importProgress.total) * 100)
      : 0;

  // **[2026-02-16]** 变更原因：区分行菜单刷新条件
  // **[2026-02-16]** 变更目的：避免列头菜单误触发刷新入口
  const canRefreshFromContext =
      contextMenuTarget?.rowIndex !== undefined &&
      formulaMetaMap?.get(buildFormulaKey(contextMenuTarget.colIndex, contextMenuTarget.rowIndex))?.stale === true;
  // **[2026-02-16]** 变更原因：公式列需要编辑入口
  // **[2026-02-16]** 变更目的：仅在列头且为公式列时展示
  const canEditFormulaColumn =
      !!contextMenuTarget &&
      contextMenuTarget.rowIndex === undefined &&
      isFormulaColumnIndex(contextMenuTarget.colIndex, formulaColumns);

  return (
    <div 
        ref={gridWrapperRef}
        data-testid="glide-grid"
        style={{ width: "100%", height: "100%", overflow: "hidden", position: "relative" }} 
        onClick={() => {
            if (filterMenuOpen) setFilterMenuOpen(false);
            if (contextMenuOpen) setContextMenuOpen(false);
        }}
    >
      {/* **[2026-02-16]** 变更原因：提供脚本可定位的表头图标节点。 */}
      {/* **[2026-02-16]** 变更目的：让表头图标与高度验证更稳定。 */}
      <div
        data-testid="custom-header-icon-svg"
        data-header-height={String(resolvedHeaderHeight)}
        style={{ position: "absolute", width: 0, height: 0, overflow: "hidden" }}
        dangerouslySetInnerHTML={{
            __html: customHeaderIcons[customHeaderIconKey]({ fgColor: "#ffffff", bgColor: "#000000" })
        }}
      />
      <DataEditor
        theme={customTheme}
        provideEditor={provideEditor}
        getCellContent={getCellContent}
        columns={columns}
        rows={rowCount}
        
        // Excel Features
        onColumnResize={onColumnResize}
        onColumnMoved={onColumnMoved}
        onPaste={onPaste}
        rowMarkerWidth={50}
        
        // **[2026-02-16]** 变更原因：保留单格编辑通道。
        // **[2026-02-16]** 变更目的：兼容已有交互逻辑。
        onCellEdited={onCellEdited}
        // **[2026-02-16]** 变更原因：新增批量编辑通道。
        // **[2026-02-16]** 变更目的：提升粘贴与多选编辑体验。
        onCellsEdited={onCellsEdited}
        // Set range to cover the merge
        gridSelection={selection}
        onGridSelectionChange={onGridSelectionChange}
        
        // Allow spanning across cells
        
        // Force update version with timestamp
        
        getCellsForSelection={true}
        width={"100%"}
        height={"100%"}
        smoothScrollX={true}
        smoothScrollY={true}
        rowMarkers={"both"}
        freezeColumns={freezeColumns}
        onHeaderMenuClick={onHeaderMenuClick}
        onHeaderContextMenu={onHeaderContextMenu}
        onVisibleRegionChanged={onVisibleRegionChanged}
        // **[2026-02-16]** 变更原因：补齐滚动超出量配置。
        // **[2026-02-16]** 变更目的：适配更大边距滚动体验。
        overscrollX={overscrollX}
        overscrollY={overscrollY}
        // **[2026-02-16]** 变更原因：补齐表头高度相关配置。
        // **[2026-02-16]** 变更目的：支持分组表头布局。
        headerHeight={resolvedHeaderHeight}
        groupHeaderHeight={resolvedGroupHeaderHeight}
        // **[2026-02-16]** 变更原因：透传表头图标配置。
        // **[2026-02-16]** 变更目的：支持自定义表头展示。
        headerIcons={mergedHeaderIcons}
        // **[2026-02-16]** 变更原因：补齐右侧元素挂载入口。
        // **[2026-02-16]** 变更目的：扩展自定义侧边工具区。
        rightElement={rightElement}
        rightElementProps={rightElementProps}
        // **[2026-02-16]** 变更原因：补齐纵向分割线与缩放配置。
        // **[2026-02-16]** 变更目的：统一视觉细节与缩放策略。
        verticalBorder={verticalBorder}
        scaleToRem={scaleToRem}
        // **[2026-02-16]** 变更原因：补齐分组详情回调入口。
        // **[2026-02-16]** 变更目的：支持分组行信息渲染。
        getGroupDetails={getGroupDetails}

        // Added features
        onCellContextMenu={(cell, event: CellClickedEventArgs) => {
            event.preventDefault();
            setContextMenuTarget({ x: event.localEventX, y: event.localEventY, colIndex: cell[0], rowIndex: cell[1] });
            setContextMenuOpen(true);
        }}
        // ### 变更记录
        // - 2026-02-15: 原因=渲染过期角标; 目的=仅标记过期公式单元格
        // - 2026-02-15: 原因=保持默认绘制; 目的=只叠加角标不替换内容
        drawCell={drawCell}
        // ### 变更记录
        // - 2026-02-16: 原因=提供行级视觉提示; 目的=快速识别过期公式所在行
        // - 2026-02-16: 原因=与 staleRowSet 保持一致; 目的=避免重复判断
        getRowThemeOverride={getStaleRowTheme}
        rowHeight={(row) => rowSizes.get(row) ?? 34}
      />

      {(importProgressVisible || pasteNoticeVisible || formulaNoticeVisible) && (
          <div
              style={{
                  position: "absolute",
                  top: 12,
                  right: 12,
                  zIndex: 9999,
                  background: "#1f2937",
                  color: "#f9fafb",
                  padding: "8px 10px",
                  borderRadius: 6,
                  border: "1px solid #374151",
                  boxShadow: "0 8px 20px rgba(0,0,0,0.35)",
                  fontSize: 12
              }}
          >
              {importProgressVisible ? (
                  <div style={{ display: "flex", flexDirection: "column", gap: 6, minWidth: 180 }}>
                      <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                          <span>正在导入...</span>
                          <span>{progressPercent}%</span>
                      </div>
                      <div style={{ height: 6, background: "#374151", borderRadius: 4, overflow: "hidden" }}>
                          <div
                              style={{
                                  height: "100%",
                                  width: `${progressPercent}%`,
                                  background: "#38bdf8",
                                  transition: "width 0.2s ease"
                              }}
                          />
                      </div>
                      <div style={{ textAlign: "right", color: "#cbd5f5" }}>
                          {importProgress.completed}/{importProgress.total}
                      </div>
                  </div>
              ) : pasteNoticeVisible ? (
                  "粘贴处理中，请稍等..."
              ) : (
                  formulaNoticeMessage
              )}
          </div>
      )}
      
      {filterMenuOpen && filterMenuTarget && createPortal(
          <div 
            className="fixed z-50"
            style={{ 
                position: "fixed",
                zIndex: 9999,
                top: `${filterMenuTarget.y}px`, 
                left: `${filterMenuTarget.x}px`,
                width: 260,
                background: "#2b2b2b",
                color: "#f9fafb",
                borderRadius: 6,
                border: "1px solid #1f2937",
                boxShadow: "0 8px 24px rgba(0,0,0,0.35)",
                display: "flex",
                flexDirection: "column",
                fontSize: 12
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ padding: "6px 6px 4px 6px" }}>
                {[
                    { label: "升序(S)", action: () => handleSort("asc") },
                    { label: "降序(O)", action: () => handleSort("desc") },
                    { label: "清除筛选器(C)", action: handleClearFilter },
                ].map((item, idx) => (
                    <div
                        key={idx}
                        onClick={item.action}
                        style={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            padding: "4px 6px",
                            borderRadius: 4,
                            color: "#f9fafb",
                            cursor: "pointer"
                        }}
                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                    >
                        <span>{item.label}</span>
                    </div>
                ))}
            </div>
            <div style={{ height: 1, background: "#374151" }} />
            <div style={{ padding: "6px" }}>
                <input 
                    type="text" 
                    placeholder="搜索..."
                    value={filterSearchText}
                    onChange={(e) => {
                        setFilterSearchText(e.target.value);
                        setFilterOffset(0);
                    }}
                    style={{ 
                        width: "100%",
                        height: 28,
                        border: "1px solid #4b5563",
                        borderRadius: 4,
                        background: "#1f2937",
                        color: "#f9fafb",
                        padding: "0 6px"
                    }}
                    autoFocus
                />
            </div>
            <div style={{ height: 1, background: "#374151" }} />
            <div style={{ padding: "6px", display: "flex", flexDirection: "column", gap: 4 }}>
                <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", padding: "2px 4px" }}>
                    <input
                        type="checkbox"
                        checked={filterValues.length > 0 && filterSelected.size === filterValues.length}
                        onChange={(e) => {
                            if (e.target.checked) {
                                setFilterSelected(new Set(filterValues));
                            } else {
                                setFilterSelected(new Set());
                            }
                        }}
                    />
                    <span>(全选)</span>
                </label>
                <div 
                    style={{ maxHeight: 200, overflowY: "auto", border: "1px solid #4b5563", background: "#1f2937", borderRadius: 4, padding: "4px 6px" }}
                    onScroll={(e) => {
                        const target = e.currentTarget;
                        if (target.scrollHeight - target.scrollTop <= target.clientHeight + 10) {
                            // ### 变更记录
                            // - 2026-03-12: 原因=筛选滚动分页步长与请求 limit 不一致; 目的=避免重复页或漏页
                            setFilterOffset(prev => prev + FILTER_PAGE_SIZE);
                        }
                    }}
                >
                    {filterValues.length === 0 ? (
                        <div style={{ color: "#9ca3af", textAlign: "center", padding: "8px" }}>无数据</div>
                    ) : (
                        filterValues.map((val, i) => {
                            const displayValue = val === "" ? "(空白)" : val;
                            return (
                                <label key={`${val}-${i}`} style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0", cursor: "pointer" }}>
                                    <input
                                        type="checkbox"
                                        checked={filterSelected.has(val)}
                                        onChange={(e) => {
                                            const next = new Set(filterSelected);
                                            if (e.target.checked) {
                                                next.add(val);
                                            } else {
                                                next.delete(val);
                                            }
                                            setFilterSelected(next);
                                        }}
                                    />
                                    <span>{displayValue}</span>
                                </label>
                            );
                        })
                    )}
                </div>
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, padding: "8px 8px 10px 8px", borderTop: "1px solid #374151" }}>
                <button 
                    style={{ 
                        padding: "4px 12px",
                        borderRadius: 4,
                        border: "1px solid #4b5563",
                        background: "#374151",
                        color: "#f9fafb",
                        cursor: "pointer"
                    }}
                    onClick={() => setFilterMenuOpen(false)}
                >
                    取消
                </button>
                <button 
                    style={{ 
                        padding: "4px 12px",
                        borderRadius: 4,
                        border: "1px solid #2563eb",
                        background: "#2563eb",
                        color: "#ffffff",
                        cursor: "pointer"
                    }}
                    onClick={handleFilterConfirm}
                >
                    确定
                </button>
            </div>
          </div>,
          document.body
      )}

      {/* **[2026-02-16]** 变更原因：新增公式列弹窗。 */}
      {/* **[2026-02-16]** 变更目的：输入列名与公式并提示只读规则。 */}
      {formulaColumnDialogOpen && createPortal(
          <div
              style={{
                  position: "fixed",
                  inset: 0,
                  background: "rgba(0,0,0,0.45)",
                  zIndex: 10000,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center"
              }}
              // **[2026-02-17]** 变更原因：关闭弹窗时需要同步示例状态。
              // **[2026-02-17]** 变更目的：避免下次打开仍保持展开。
              onClick={() => {
                  setFormulaColumnDialogOpen(false);
                  setFormulaSampleOpen(false);
              }}
          >
              <div
                  style={{
                      width: 420,
                      background: "#1f2937",
                      color: "#f9fafb",
                      borderRadius: 8,
                      border: "1px solid #111827",
                      boxShadow: "0 12px 32px rgba(0,0,0,0.45)",
                      padding: "16px 16px 12px 16px",
                      display: "flex",
                      flexDirection: "column",
                      gap: 10
                  }}
                  onClick={(e) => e.stopPropagation()}
              >
                  {/* **[2026-02-16]** 变更原因：编辑模式需要区分标题。 */}
                  {/* **[2026-02-16]** 变更目的：明确用户当前操作类型。 */}
                  <div style={{ fontSize: 14, fontWeight: 600 }}>
                      {formulaColumnDialogMode === "edit" ? "编辑公式列" : "插入公式列"}
                  </div>
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                      <label style={{ fontSize: 12, color: "#e5e7eb" }}>列名</label>
                      <input
                          type="text"
                          value={formulaColumnName}
                          onChange={(e) => {
                              setFormulaColumnName(e.target.value);
                              setFormulaColumnError("");
                          }}
                          placeholder="例如：总价"
                          style={{
                              width: "100%",
                              height: 30,
                              border: "1px solid #374151",
                              borderRadius: 4,
                              background: "#111827",
                              color: "#f9fafb",
                              padding: "0 8px",
                              opacity: formulaColumnDialogMode === "edit" ? 0.6 : 1
                          }}
                          disabled={formulaColumnDialogMode === "edit"}
                          autoFocus
                      />
                  </div>
                  {/* **[2026-02-17]** 变更原因：新增公式示例入口与聚合提示。 */}
                  {/* **[2026-02-17]** 变更目的：减少输入成本并提示限制。 */}
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                      <label style={{ fontSize: 12, color: "#e5e7eb" }}>公式</label>
                      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                          <input
                              type="text"
                              value={formulaColumnInput}
                              onChange={(e) => {
                                  const nextValue = e.target.value;
                                  setFormulaColumnInput(nextValue);
                                  // **[2026-02-17]** 变更原因：输入时检测聚合函数。
                                  // **[2026-02-17]** 变更目的：即时提示不支持。
                                  const aggregateHit = detectAggregateFunction(nextValue);
                                  if (aggregateHit) {
                                      setFormulaColumnError(aggregateUnsupportedMessage);
                                  } else {
                                      setFormulaColumnError("");
                                  }
                              }}
                              placeholder="例如：B*C"
                              style={{
                                  width: "100%",
                                  height: 30,
                                  border: "1px solid #374151",
                                  borderRadius: 4,
                                  background: "#111827",
                                  color: "#f9fafb",
                                  padding: "0 8px"
                              }}
                          />
                          <button
                              style={{
                                  padding: "4px 10px",
                                  borderRadius: 4,
                                  border: "1px solid #4b5563",
                                  background: "#111827",
                                  color: "#e5e7eb",
                                  cursor: "pointer",
                                  whiteSpace: "nowrap"
                              }}
                              // **[2026-02-17]** 变更原因：新增示例切换按钮。
                              // **[2026-02-17]** 变更目的：便于快速填充公式。
                              onClick={() => setFormulaSampleOpen((prev) => !prev)}
                          >
                              选择公式
                          </button>
                      </div>
                  </div>
                  {/* **[2026-02-17]** 变更原因：展示公式示例列表。 */}
                  {/* **[2026-02-17]** 变更目的：提供一键填充能力。 */}
                  {formulaSampleOpen && (
                      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                          {formulaSamples.map((sample) => (
                              <button
                                  key={sample}
                                  style={{
                                      padding: "2px 8px",
                                      borderRadius: 999,
                                      border: "1px solid #4b5563",
                                      background: "#111827",
                                      color: "#e5e7eb",
                                      cursor: "pointer",
                                      fontSize: 12
                                  }}
                                  onClick={() => {
                                      setFormulaColumnInput(sample);
                                      setFormulaColumnError("");
                                      // **[2026-02-17]** 变更原因：选择后收起示例列表。
                                      // **[2026-02-17]** 变更目的：减少视觉干扰。
                                      setFormulaSampleOpen(false);
                                  }}
                              >
                                  {sample}
                              </button>
                          ))}
                      </div>
                  )}
                  {formulaColumnError && (
                      <div style={{ color: "#f87171", fontSize: 12 }}>{formulaColumnError}</div>
                  )}
                  {/* **[2026-02-17]** 变更原因：提示聚合函数不支持。 */}
                  {/* **[2026-02-17]** 变更目的：避免用户误用 SUM/AVG/COUNT。 */}
                  <div style={{ fontSize: 11, color: "#9ca3af" }}>
                      {aggregateUnsupportedMessage}
                  </div>
                  {/* **[2026-02-16]** 变更原因：插入与编辑提示合并。 */}
                  {/* **[2026-02-16]** 变更目的：减少误解并保持一致说明。 */}
                  <div style={{ fontSize: 12, color: "#cbd5f5" }}>
                      公式列不允许编辑单个单元格
                  </div>
                  <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 6 }}>
                      <button
                          style={{
                              padding: "4px 12px",
                              borderRadius: 4,
                              border: "1px solid #4b5563",
                              background: "#374151",
                              color: "#f9fafb",
                              cursor: "pointer"
                          }}
                          // **[2026-02-17]** 变更原因：取消时清理示例状态。
                          // **[2026-02-17]** 变更目的：保证下次打开一致。
                          onClick={() => {
                              setFormulaColumnDialogOpen(false);
                              setFormulaSampleOpen(false);
                          }}
                      >
                          取消
                      </button>
                      <button
                          style={{
                              padding: "4px 12px",
                              borderRadius: 4,
                              border: "1px solid #2563eb",
                              background: "#2563eb",
                              color: "#ffffff",
                              cursor: "pointer"
                          }}
                          onClick={handleConfirmFormulaColumn}
                      >
                          确定
                      </button>
                  </div>
              </div>
          </div>,
          document.body
      )}

      {/* Context Menu (Freeze) */}
      {contextMenuOpen && contextMenuTarget && createPortal(
          <div 
            className="fixed z-50"
            style={{ 
                position: "fixed",
                zIndex: 9999,
                top: `${contextMenuTarget.y}px`, 
                left: `${contextMenuTarget.x}px`,
                width: 180,
                background: "#2b2b2b",
                color: "#f9fafb",
                borderRadius: 6,
                border: "1px solid #1f2937",
                boxShadow: "0 8px 24px rgba(0,0,0,0.35)",
                display: "flex",
                flexDirection: "column",
                fontSize: 12,
                padding: "4px 0"
            }}
            onClick={(e) => e.stopPropagation()}
          >
              {contextMenuTarget.rowIndex !== undefined && (
                  <>
                      {canRefreshFromContext && (
                          <div 
                            style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                            onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                            onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                            onClick={handleRefreshFormulaInContext}
                          >
                              <span>刷新公式</span>
                          </div>
                      )}
                      {canRefreshFromContext && (
                          <div style={{ height: 1, background: "#374151", margin: "4px 0" }} />
                      )}
                      <div 
                        style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                        onClick={handleInsertRowAbove}
                      >
                          <span>插入行 (上)</span>
                      </div>
                      <div 
                        style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                        onClick={handleInsertRowBelow}
                      >
                          <span>插入行 (下)</span>
                      </div>
                      <div 
                        style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                        onClick={handleDeleteRow}
                      >
                          <span>删除行</span>
                      </div>
                      <div 
                        style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                        onClick={handleRowHeight}
                      >
                          <span>设置行高</span>
                      </div>
                      <div style={{ height: 1, background: "#374151", margin: "4px 0" }} />
                  </>
              )}
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleInsertColumnBefore}
              >
                  <span>插入列 (左)</span>
              </div>
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleInsertColumnAfter}
              >
                  <span>插入列 (右)</span>
              </div>
              {/* **[2026-02-16]** 变更原因：新增插入公式列入口。 */}
              {/* **[2026-02-16]** 变更目的：减少用户路径。 */}
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleOpenFormulaColumnDialog}
              >
                  <span>插入公式列</span>
              </div>
              {canEditFormulaColumn && (
                  <>
                      {/* **[2026-02-16]** 变更原因：公式列需要修改入口。 */}
                      {/* **[2026-02-16]** 变更目的：复用弹窗完成更新。 */}
                      <div 
                        style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                        onClick={handleOpenFormulaColumnEditDialog}
                      >
                          <span>编辑公式列</span>
                      </div>
                  </>
              )}
              <div style={{ height: 1, background: "#374151", margin: "4px 0" }} />
              {/* **[2026-02-16]** 变更原因：新增单元格格式入口。 */}
              {/* **[2026-02-16]** 变更目的：统一样式设置与操作反馈。 */}
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between", opacity: 0.7 }}
              >
                  <span>单元格格式</span>
              </div>
              <div 
                style={{ padding: "6px 12px 6px 24px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={() => {
                    applySelectionStyle({ format: "number" });
                    setContextMenuOpen(false);
                }}
              >
                  <span>数值</span>
              </div>
              <div 
                style={{ padding: "6px 12px 6px 24px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={() => {
                    applySelectionStyle({ format: "percent" });
                    setContextMenuOpen(false);
                }}
              >
                  <span>百分比</span>
              </div>
              <div 
                style={{ padding: "6px 12px 6px 24px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={() => {
                    applySelectionStyle({ format: "currency" });
                    setContextMenuOpen(false);
                }}
              >
                  <span>货币</span>
              </div>
              <div 
                style={{ padding: "6px 12px 6px 24px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={() => {
                    applySelectionStyle({ format: "date" });
                    setContextMenuOpen(false);
                }}
              >
                  <span>日期</span>
              </div>
              <div style={{ height: 1, background: "#374151", margin: "4px 0" }} />
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleDeleteColumn}
              >
                  <span>删除列</span>
              </div>
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleClearColumnContent}
              >
                  <span>清除内容</span>
              </div>
              <div style={{ height: 1, background: "#374151", margin: "4px 0" }} />
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleFreezeColumn}
              >
                  <span>冻结到此列</span>
              </div>
              <div 
                style={{ padding: "6px 12px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between" }}
                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#374151"}
                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "transparent"}
                onClick={handleUnfreeze}
              >
                  <span>取消冻结</span>
              </div>
          </div>,
          document.body
      )}
    </div>
  );
});
