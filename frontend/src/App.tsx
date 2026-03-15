import React, { useEffect, useRef, useState } from 'react';
import './App.css';
import { GlideGrid, GlideGridHandle, ActiveFilter } from './components/GlideGrid';
import { PivotSidebar, PivotConfigState, Field } from './components/pivot/PivotSidebar';
import { PivotEngine } from './utils/PivotEngine';
import { Toolbar } from './components/layout/Toolbar';
import { FormulaBar } from './components/layout/FormulaBar';
import { TimeMachineDrawer } from './components/TimeMachineDrawer';
import { SheetBar } from './components/layout/SheetBar';
// ### Change Log
// - 2026-03-15: Reason=Single-row header needs shared helpers; Purpose=centralize brand + grouping rules
import { getBrandTitle, getHeaderGroups } from './utils/headerLayout';
// ### Change Log
// - 2026-03-15: Reason=Auto-hide loader completion notice; Purpose=keep debug overlay tidy
import { shouldAutoHideDebugInfo } from './utils/debugOverlay';
// ### Change Log
// - 2026-03-15: Reason=Persist pivot output to new session; Purpose=build update payloads
import { buildPivotUpdates, chunkPivotUpdates, buildPivotColumnAdds, buildPivotUpdatesWithOffset, formatPivotPersistError } from './utils/pivotSession';
// ### Change Log
// - 2026-03-15: Reason=Align pivot routes; Purpose=use GridAPI for ensure_columns
import { ensureColumns } from './utils/GridAPI';
// ### Change Log
// - 2026-03-15: Reason=Hide invalid system tables; Purpose=avoid sys_metadata selection errors
import { filterUserVisibleTables } from './utils/tableList';

const toExcelColumnLabel = (index: number): string => {
  let result = '';
  let value = index;
  while (value >= 0) {
    result = String.fromCharCode((value % 26) + 65) + result;
    value = Math.floor(value / 26) - 1;
  }
  return result;
};

// ### 鍙樻洿璁板綍
// - 2026-03-11 21:45: 鍘熷洜=鎺ュ彛鍋跺彂杩斿洖绌轰綋鎴栭潪 JSON锛岀洿鎺?res.json 浼氭姏閿? 鐩殑=缁熶竴瀹夊叏瑙ｆ瀽骞惰緭鍑哄彲璇婚敊璇€?
const parseJsonSafely = async (res: Response): Promise<{ ok: true; data: any } | { ok: false; reason: string; rawPreview: string }> => {
  const raw = await res.text();
  if (!raw || raw.trim().length === 0) {
    return { ok: false, reason: `empty response (status ${res.status})`, rawPreview: '' };
  }
  try {
    return { ok: true, data: JSON.parse(raw) };
  } catch {
    return { ok: false, reason: `invalid json (status ${res.status})`, rawPreview: raw.slice(0, 200) };
  }
};

// ### 鍙樻洿璁板綍
// - 2026-03-11 21:45: 鍘熷洜=琛ㄥ悕鍖呭惈鐗规畩瀛楃鏃惰８ SQL 浼氬け璐? 鐩殑=缁熶竴杞箟鏍囪瘑绗︼紝閬垮厤 execute 鏌ヨ鎶ラ敊銆?
const quoteSqlIdentifier = (identifier: string): string => `"${String(identifier).replace(/"/g, `""`)}"`;

// ### Change Log
// - 2026-03-14: Reason=Expose session tabs data; Purpose=normalize session fields for UI
type SessionItem = {
  sessionId: string;
  name: string;
  createdAt: number;
  isDefault: boolean;
  displayName: string;
};

// ### Change Log
// - 2026-03-14: Reason=Default session needs fixed label; Purpose=keep tab text consistent
const DEFAULT_SESSION_LABEL = "\u9ed8\u8ba4/\u53ea\u8bfb";

// ### Change Log
// - 2026-03-14: Reason=Tests need stable window.app; Purpose=create a placeholder during render
const ensureWindowApp = () => {
  if (typeof window === 'undefined') return null;
  const target = (window as any);
  if (!target.app) {
    target.app = {};
  }
  return target.app as Record<string, any>;
};


const App: React.FC = () => {
  const [backendStatus, setBackendStatus] = useState<string>('Disconnected');
  const [tables, setTables] = useState<string[]>([]);
  const [currentTable, setCurrentTable] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [debugInfo, setDebugInfo] = useState<string>('');
  
  // State for Pivot metadata inference
  const [gridColumns, setGridColumns] = useState<string[]>([]);
  const [gridRows, setGridRows] = useState<any[][]>([]);
  // ### 鍙樻洿璁板綍
  // - 2026-03-11 23:05: 鍘熷洜=鐢ㄦ埛鍙嶉搴曢儴鏍峰紡鍜岀瓫閫夊叆鍙ｄ涪澶? 鐩殑=灏嗛〉闈富鍏ュ彛鍥哄畾鍒?GlideGrid 鑳藉姏闆嗐€?
  const gridRef = useRef<GlideGridHandle | null>(null);
  // ### 鍙樻洿璁板綍
  // - 2026-03-14: 鍘熷洜=浼氳瘽 tabs 闇€瑕侀泦涓姸鎬? 鐩殑=鍚屾 sessions/榛樿鍙/褰撳墠浼氳瘽銆?
  const [sessionId, setSessionId] = useState<string>('');
  const [sessions, setSessions] = useState<SessionItem[]>([]);
  const [defaultSessionId, setDefaultSessionId] = useState<string>('');
  // ### 变更记录
  // - 2026-03-14: Reason=Session fetch can race; Purpose=ignore stale responses.
  const sessionFetchToken = useRef(0);
  // ### Change Log
  // - 2026-03-15: Reason=Loader completion should auto-hide; Purpose=store timer handle safely
  const debugAutoHideTimer = useRef<number | undefined>(undefined);
  const [isReadOnly, setIsReadOnly] = useState<boolean>(false);
  const [activeFilters, setActiveFilters] = useState<ActiveFilter[]>([]);
  const [canUndo, setCanUndo] = useState<boolean>(false);
  const [canRedo, setCanRedo] = useState<boolean>(false);
  const [selectedCell, setSelectedCell] = useState<string>('');
  const [selectedPosition, setSelectedPosition] = useState<{ col: number; row: number } | null>(null);

  // Pivot State
  const [showPivot, setShowPivot] = useState<boolean>(false);
  const [pivotConfig, setPivotConfig] = useState<PivotConfigState>({
    rows: [],
    columns: [],
    values: [],
    filters: []
  });
  const [pivotFields, setPivotFields] = useState<Field[]>([]);
  const [showTimeMachine, setShowTimeMachine] = useState<boolean>(false);
  const [formulaText, setFormulaText] = useState<string>('');

  // ### Change Log
  // - 2026-03-15: Reason=Auto-hide loaded debug message; Purpose=remove overlay after 10s
  // - 2026-03-15: Reason=Only affect completion messages; Purpose=keep errors visible
  useEffect(() => {
    // ### Change Log
    // - 2026-03-15: Reason=Reset previous timer; Purpose=avoid multiple pending clears
    if (debugAutoHideTimer.current !== undefined) {
      window.clearTimeout(debugAutoHideTimer.current);
      debugAutoHideTimer.current = undefined;
    }
    // ### Change Log
    // - 2026-03-15: Reason=Only hide completion messages; Purpose=avoid clearing alerts
    if (shouldAutoHideDebugInfo(debugInfo, loading)) {
      debugAutoHideTimer.current = window.setTimeout(() => {
        setDebugInfo('');
        // ### Change Log
        // - 2026-03-15: Reason=Timer should not linger; Purpose=avoid stale handles
        debugAutoHideTimer.current = undefined;
      }, 10000);
    }
    return () => {
      if (debugAutoHideTimer.current !== undefined) {
        window.clearTimeout(debugAutoHideTimer.current);
        debugAutoHideTimer.current = undefined;
      }
    };
  }, [debugInfo, loading]);
  
  const checkBackend = async () => {
    try {
      const res = await fetch('/api/health');
      if (res.ok) {
        const parsed = await parseJsonSafely(res);
      // ### Change Log
        if (!parsed.ok) {
          setBackendStatus('Backend Response Invalid');
          // ### Change Log
          // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep health check errors readable.
          setDebugInfo(`Health check parse failed: ${parsed.reason}`);
          return;
        }
        const data = parsed.data;
        setBackendStatus(`Connected (v${data.version})`);
        fetchTables();
      } else {
        setBackendStatus('Backend Error');
      }
    } catch (e) {
      setBackendStatus('Backend Unreachable');
    }
  };

  const fetchTables = async (): Promise<string[]> => {
    try {
      const res = await fetch('/api/tables');
      if (res.ok) {
        const parsed = await parseJsonSafely(res);
        if (!parsed.ok) {
          // ### Change Log
          // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep table list errors readable.
          setDebugInfo(`Table list parse failed: ${parsed.reason}`);
          // ### Change Log
          // - 2026-03-14: Reason=Keep return type consistent; Purpose=avoid undefined in Promise<string[]>.
          return [];
        }
        const data = parsed.data;
        console.log("Tables fetched:", data);
        const nextTables = (data.tables || [])
          .map((item: any) => typeof item === 'string' ? item : item.table_name)
          .filter(Boolean);
        // ### Change Log
        // - 2026-03-15: Reason=System table not present in backend; Purpose=filter sys_metadata from UI
        const visibleTables = filterUserVisibleTables(nextTables);
        setTables(visibleTables);
        // ### 鍙樻洿璁板綍
        // - 2026-03-14: 鍘熷洜=琛ㄥ垏鎹㈤渶瑕佸悓姝?sessions锛涚洰鐨?缁熶竴閫氳繃 selectTable 鍏ュ彛銆?
        if (!currentTable && visibleTables.length > 0) {
          selectTable(visibleTables[0]);
        }
        return visibleTables;
      }
      return [];
    } catch (e) {
      console.error("Failed to fetch tables", e);
      return [];
    }
  };

  const fetchTableData = async (tableName: string) => {
    setLoading(true);
    setDebugInfo(`Fetching ${tableName}...`);
    console.log(`Fetching data for table: ${tableName}`);
    try {
      const res = await fetch('/api/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        // ### 鍙樻洿璁板綍
        // - 2026-03-11 21:45: 鍘熷洜=鏈浆涔夎〃鍚嶄細瀵艰嚧鐗规畩瀛楃琛ㄦ煡璇㈠け璐? 鐩殑=缁熶竴浣跨敤瀹夊叏鏍囪瘑绗︽嫾鎺?SQL銆?
        body: JSON.stringify({ sql: `SELECT * FROM ${quoteSqlIdentifier(tableName)} LIMIT 1000` })
      });

      if (res.ok) {
        const parsed = await parseJsonSafely(res);
        if (!parsed.ok) {
          // ### Change Log
          // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep data parse errors readable.
          setDebugInfo(`Data parse failed: ${parsed.reason}`);
          return;
        }
        const data = parsed.data;
        console.log("Fetch response:", data);
        if (data.error) {
          alert(`Error: ${data.error}`);
          setDebugInfo(`Error: ${data.error}`);
        } else {
          setGridColumns(data.columns);
          setGridRows(data.rows);
          setDebugInfo(`Loaded ${tableName}: ${data.rows?.length ?? 0} rows`);
        }
      } else {
        console.error("Fetch failed:", res.status, res.statusText);
        setDebugInfo(`Fetch failed: ${res.status}`);
      }
    } catch (e: any) {
      console.error("Failed to fetch table data", e);
      setDebugInfo(`Exception: ${e.message}`);
    } finally {
      setLoading(false);
    }
  };

    // ### 鍙樻洿璁板綍
  // - 2026-03-14: 鍘熷洜=鍚庣 sessions 杩斿洖缁撴瀯闇€瑕佺粺涓€锛涚洰鐨?鍓嶇绋冲畾娑堣垂銆?
  const normalizeSessions = (rawSessions: any[]): SessionItem[] => {
    return (rawSessions || [])
      .map((item: any) => ({
        sessionId: String(item?.session_id ?? item?.sessionId ?? ''),
        name: String(item?.name ?? ''),
        createdAt: Number(item?.created_at ?? item?.createdAt ?? 0),
        isDefault: Boolean(item?.is_default ?? item?.isDefault ?? false),
        displayName: ''
      }))
      .filter((item: SessionItem) => item.sessionId);
  };

  // ### 鍙樻洿璁板綍
  // - 2026-03-14: 鍘熷洜=榛樿浼氳瘽鍙兘鏈爣璁?is_default锛涚洰鐨?鎻愪緵绋冲畾鍏滃簳瑙勫垯銆?
  const resolveDefaultSessionId = (items: SessionItem[]): string => {
    const explicit = items.find((item) => item.isDefault);
    if (explicit?.sessionId) return explicit.sessionId;
    const sortedByOldest = [...items].sort((a, b) => a.createdAt - b.createdAt);
    return sortedByOldest[0]?.sessionId || '';
  };

  // ### 鍙樻洿璁板綍
  // - 2026-03-14: 鍘熷洜=搴曢儴 tabs 闇€瑕侀粯璁ょ疆椤讹紱鐩殑=淇濊瘉榛樿鍙浼氳瘽灞曠ず涓€鑷淬€?
  const buildSessionList = (items: SessionItem[], defaultId: string): SessionItem[] => {
    const decorated = items.map((item) => ({
      ...item,
      // ### 鍙樻洿璁板綍
      // - 2026-03-14: 鍘熷洜=榛樿浼氳瘽鍙兘缂哄皯 isDefault 鏍囪锛涚洰鐨?鍏滃簳鏍囪瘑榛樿椤广€?
      isDefault: item.isDefault || item.sessionId === defaultId,
      displayName: item.sessionId === defaultId
        ? DEFAULT_SESSION_LABEL
        : (item.name || item.sessionId.slice(0, 8))
    }));
    const defaultItem = decorated.find((item) => item.sessionId === defaultId);
    const others = decorated
      .filter((item) => item.sessionId !== defaultId)
      .sort((a, b) => b.createdAt - a.createdAt);
    return defaultItem ? [defaultItem, ...others] : others;
  };

  // ### 鍙樻洿璁板綍
  // - 2026-03-14: Reason=Centralize session switching; Purpose=sync active session state.
  // - 2026-03-14: Reason=Session list can refresh concurrently; Purpose=avoid stale switch overwrite.
  const switchSession = async (
    tableName: string,
    targetSessionId: string,
    defaultId: string,
    requestToken?: number
  ) => {
    if (!tableName || !targetSessionId) return;
    try {
      const res = await fetch('/api/switch_session', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          table_name: tableName,
          session_id: targetSessionId
        })
      });
      const parsed = await parseJsonSafely(res);
      if (!parsed.ok) {
        // ### Change Log
        // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep switch-session errors readable.
        setDebugInfo(`Switch session parse failed: ${parsed.reason}`);
        return;
      }
      if (parsed.data?.status !== 'ok') {
        // ### Change Log
        // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep switch-session errors readable.
        setDebugInfo(parsed.data?.message || 'Switch session failed');
        return;
      }
      // ### 变更记录

      // - 2026-03-14: Reason=Avoid stale session switch; Purpose=apply only latest fetch.

      if (requestToken && requestToken !== sessionFetchToken.current) return;

      setSessionId(targetSessionId);
      setIsReadOnly(Boolean(defaultId && targetSessionId === defaultId));
    } catch (e: any) {
      // ### Change Log
      // - 2026-03-14: Reason=Replace non-ASCII debug text; Purpose=keep switch-session errors readable.
      setDebugInfo(`Switch session failed: ${e.message || 'unknown error'}`);
    }
  };

  // ### 鍙樻洿璁板綍
  // - 2026-03-14: Reason=Refresh sessions on table change; Purpose=sync tabs with backend.
  // - 2026-03-14: Reason=Normalize sessions payload & handle races; Purpose=keep tabs consistent.
  const fetchSessionsForTable = async (tableName: string, preferredSessionId?: string) => {
    if (!tableName) return;
    // ### 变更记录
    // - 2026-03-14: Reason=Session fetch can race; Purpose=tag latest request.
    const requestToken = sessionFetchToken.current + 1;
    sessionFetchToken.current = requestToken;
    // ### Change Log
    try {
      const res = await fetch(`/api/sessions?table_name=${encodeURIComponent(tableName)}`);
      // ### Change Log
      if (!res.ok) {
        // ### Change Log
        // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep session fetch errors readable.
        setDebugInfo(`Fetch sessions failed: ${res.status}`);
        return;
      }
      const parsed = await parseJsonSafely(res);
      if (!parsed.ok) {
        // ### Change Log
        // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep session fetch errors readable.
        setDebugInfo(`Fetch sessions parse failed: ${parsed.reason}`);
        return;
      }
      // ### 变更记录
      // - 2026-03-14: Reason=Backend payload can be nested; Purpose=normalize sessions safely.
      const payload = parsed.data ?? {};
      const rawSessions = Array.isArray(payload.sessions)
        ? payload.sessions
        : (Array.isArray(payload.data?.sessions) ? payload.data.sessions : []);
      // ### Change Log
      const normalized = normalizeSessions(rawSessions);
      // ### 变更记录
      // - 2026-03-14: Reason=Ignore stale session response; Purpose=avoid overwriting new tabs.
      if (requestToken !== sessionFetchToken.current) return;
      // ### Change Log
      // ### Change Log
      // - 2026-03-14: Reason=No sessions returned for base table; Purpose=render synthetic default tab
      if (normalized.length === 0) {
        const syntheticDefault: SessionItem = {
          sessionId: '',
          name: '',
          createdAt: 0,
          isDefault: true,
          displayName: DEFAULT_SESSION_LABEL
        };

        setSessions([syntheticDefault]);
        setDefaultSessionId('');
        setSessionId('');
        setIsReadOnly(true);
        return;
      }
      const defaultId = resolveDefaultSessionId(normalized);
      const activeFromApi = String(payload.active_session_id || payload.data?.active_session_id || '');
      const targetId = preferredSessionId || activeFromApi || defaultId || normalized[0]?.sessionId || '';
      const sessionList = buildSessionList(normalized, defaultId);
      setSessions(sessionList);
      setDefaultSessionId(defaultId);
      // ### 变更记录
      // - 2026-03-14: Reason=Ensure only latest fetch can switch; Purpose=keep active session stable.
      if (requestToken !== sessionFetchToken.current) return;
      if (targetId) {
        await switchSession(tableName, targetId, defaultId, requestToken);
      } else {
        setSessionId('');
        setIsReadOnly(false);
      }
    } catch (e: any) {
      // ### Change Log
      // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep session fetch errors readable.
      setDebugInfo(`Fetch sessions error: ${e?.message || 'unknown error'}`);
    }
  };

  // ### Change Log
  // - 2026-03-14: Reason=Centralize table selection side-effects; Purpose=sync grid, sessions, and UI state
  const selectTable = (tableName: string) => {
    console.log("Selected table:", tableName);
    setCurrentTable(tableName);
    // ### Change Log
    // - 2026-03-14: Reason=Reset session-scoped state on table switch; Purpose=avoid cross-table leakage
    setSessionId('');
    setSessions([]);
    setDefaultSessionId('');
    setIsReadOnly(false);
    setActiveFilters([]);
    setCanUndo(false);
    setCanRedo(false);
    setSelectedCell('');
    setSelectedPosition(null);
    setFormulaText('');
    if (tableName) {
      fetchTableData(tableName);
      fetchSessionsForTable(tableName);
    }
  };

  // ### Change Log
  // - 2026-03-14: Reason=Ensure backend tables load on mount; Purpose=populate table selector
  useEffect(() => {
    checkBackend();
  }, []);

  // ### 鍙樻洿璁板綍
  // - 2026-03-14: 鍘熷洜=鏂版矙鐩橀渶瑕?SheetN 鍛藉悕锛涚洰鐨?淇濇寔鍛藉悕閫掑涓斿彲璇汇€?
  const buildNextSheetName = (items: SessionItem[]): string => {
    const pattern = /^Sheet(\d+)$/i;
    let maxIndex = 0;
    for (const item of items) {
      const match = pattern.exec(item.name);
      if (match) {
        const value = Number(match[1]);
        if (Number.isFinite(value)) {
          maxIndex = Math.max(maxIndex, value);
        }
      }
    }
    return `Sheet${maxIndex + 1}`;
  };

  const handleTableChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    selectTable(e.target.value);
  };

  // ### 鍙樻洿璁板綍
  // - 2026-03-14: 鍘熷洜=榛樿浼氳瘽闇€瑕佸彧璇伙紱鐩殑=鍚戠綉鏍奸€忎紶璇诲啓鐘舵€併€?
  const refreshTables = async (): Promise<string[]> => {
    return await fetchTables();
  };

  // ### Change Log
  // - 2026-03-14: Reason=Require table before switching session; Purpose=avoid empty table requests
  // - 2026-03-14: Reason=Allow default session with empty id; Purpose=sync read-only default without backend call
  const handleSessionChange = async (targetSessionId: string) => {
    if (!currentTable) {
      // ### 变更记录
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Replace non-ASCII prompt; Purpose=keep session selection readable.
      setDebugInfo('Please select a table before switching sessions');
      return;
    }
    if (!targetSessionId) {
      setSessionId('');
      setIsReadOnly(true);
      return;
    }
    await switchSession(currentTable, targetSessionId, defaultSessionId);
  };

  const handlePivotToggle = () => {
    if (!currentTable) {
      alert("Please select a table first");
      return;
    }

    if (showPivot) {
      setShowPivot(false);
      return;
    }

    // Infer fields from current grid columns
    // We assume gridColumns are the field names.
    // We try to infer type from first row of data if available.
    const fields: Field[] = gridColumns.map((col, index) => {
      let type: 'string' | 'number' | 'date' = 'string';
      if (gridRows.length > 0) {
        const val = gridRows[0][index];
        if (typeof val === 'number') type = 'number';
        else if (val instanceof Date) type = 'date';
        // Simple heuristic for date string
        else if (typeof val === 'string' && !isNaN(Date.parse(val)) && val.includes('-')) type = 'date';
      }
      return { id: col, label: col, type };
    });

    setPivotFields(fields);
    setShowPivot(true);
  };

  const handlePivotApply = async (outputMode: 'new-sheet' | 'current-sheet') => {
    // ### Change Log
    // - 2026-03-15: Reason=Friendly errors needed; Purpose=centralize pivot failure messages
    const failWithPivotError = (input: { step: "create_session" | "ensure_columns" | "batch_update" | "current_sheet"; status?: number; message?: string; }) => {
      const message = formatPivotPersistError(input);
      setDebugInfo(message);
      throw new Error(message);
    };
    try {
      setLoading(true);
      const engine = PivotEngine.getInstance();
      const result = await engine.query({
        sourceTable: currentTable,
        rows: pivotConfig.rows.map(f => ({ id: f.id, label: f.label })),
        columns: pivotConfig.columns.map(f => ({ id: f.id, label: f.label })),
        values: pivotConfig.values.map(f => ({ id: f.id, label: f.label, type: f.type })), // TODO: agg
        filters: pivotConfig.filters
      });

      console.log("Pivot Result:", result);
      
      if (outputMode === 'new-sheet') {
        // ### Change Log
        // - 2026-03-15: Reason=Pivot should persist to new session; Purpose=write results to backend
        const nextSessionId = await handleAddSheet();
        if (!nextSessionId) {
          failWithPivotError({ step: "create_session", message: "新建 Sheet 失败" });
        }
        // ### Change Log
        // - 2026-03-15: Reason=Use base table columns for updates; Purpose=align with batch_update_cells schema
        const columnNames = gridColumns.length > 0 ? gridColumns : result.headers.map((_, index) => `col_${index}`);
        // ### Change Log
        // - 2026-03-15: Reason=Pivot headers may exceed base columns; Purpose=auto expand schema before writes
        const columnAdds = buildPivotColumnAdds({
          headers: result.headers,
          columnNames,
          colOffset: 0,
          prefix: "pivot_col_"
        });
        if (columnAdds.length > 0) {
          // ### Change Log
          // - 2026-03-15: Reason=Route alignment; Purpose=delegate ensure_columns to GridAPI
          try {
            await ensureColumns({
              table_name: currentTable,
              session_id: nextSessionId,
              columns: columnAdds
            });
          } catch (error) {
            const status = (error as Error & { status?: number }).status;
            const body = (error as Error & { body?: string }).body;
            failWithPivotError({ step: "ensure_columns", status, message: body || (error as Error).message });
          }
          columnAdds.forEach((col) => columnNames.push(col.name));
        }
        const updates = buildPivotUpdates({
          headers: result.headers,
          data: result.data,
          columnNames
        });
        // ### Change Log
        // - 2026-03-15: Reason=Large payloads must be chunked; Purpose=avoid oversized requests
        const batches = chunkPivotUpdates(updates, 500);
        let batchIndex = 0;
        for (const batch of batches) {
          batchIndex += 1;
          // ### Change Log
          // - 2026-03-15: Reason=Persist can be slow; Purpose=show progress to user
          setDebugInfo(`Pivot 落库中... (${batchIndex}/${batches.length})`);
          const batchRes = await fetch('/api/batch_update_cells', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              table_name: currentTable,
              session_id: nextSessionId,
              updates: batch
            })
          });
          if (!batchRes.ok) {
            const text = await batchRes.text();
            failWithPivotError({ step: "batch_update", status: batchRes.status, message: text });
          }
        }
        // ### Change Log
        // - 2026-03-15: Reason=tsc rejects null; Purpose=pass undefined when session id missing
        await fetchSessionsForTable(currentTable, nextSessionId || undefined);
        // ### Change Log
        // - 2026-03-15: Reason=Close pivot after persist; Purpose=return to grid view
        setShowPivot(false);
        // ### Change Log
        // - 2026-03-15: Reason=Keep user informed; Purpose=show pivot persistence result
        setDebugInfo(`Pivot saved to new sheet`);
      } else {
        // ### Change Log
        // - 2026-03-15: Reason=current-sheet needs persistence; Purpose=write pivot to current session
        if (isReadOnly) {
          setDebugInfo(formatPivotPersistError({
            step: "current_sheet",
            message: "当前会话只读，无法写入 Pivot 结果"
          }));
          return;
        }
        const hasSelection = Boolean(selectedPosition);
        const rowOffset = hasSelection ? selectedPosition!.row : 0;
        const colOffset = hasSelection ? selectedPosition!.col : 0;
        const baseColumnNames = gridColumns.length > 0
          ? [...gridColumns]
          : result.headers.map((_, index) => `col_${index}`);
        const columnAdds = buildPivotColumnAdds({
          headers: result.headers,
          columnNames: baseColumnNames,
          colOffset,
          prefix: "pivot_col_"
        });
        if (columnAdds.length > 0) {
          // ### Change Log
          // - 2026-03-15: Reason=Route alignment; Purpose=delegate ensure_columns to GridAPI
          try {
            await ensureColumns({
              table_name: currentTable,
              // ### Change Log
              // - 2026-03-15: Reason=tsc build rejects null; Purpose=align with string | undefined
              session_id: sessionId || undefined,
              columns: columnAdds
            });
          } catch (error) {
            const status = (error as Error & { status?: number }).status;
            const body = (error as Error & { body?: string }).body;
            failWithPivotError({ step: "ensure_columns", status, message: body || (error as Error).message });
          }
          columnAdds.forEach((col) => baseColumnNames.push(col.name));
        }
        const updates = buildPivotUpdatesWithOffset({
          headers: result.headers,
          data: result.data,
          columnNames: baseColumnNames,
          rowOffset,
          colOffset
        });
        const batches = chunkPivotUpdates(updates, 500);
        let batchIndex = 0;
        for (const batch of batches) {
          batchIndex += 1;
          setDebugInfo(`Pivot 落库中... (${batchIndex}/${batches.length})`);
          const batchRes = await fetch('/api/batch_update_cells', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              table_name: currentTable,
              session_id: sessionId || null,
              updates: batch
            })
          });
          if (!batchRes.ok) {
            const text = await batchRes.text();
            failWithPivotError({ step: "batch_update", status: batchRes.status, message: text });
          }
        }
        await fetchTableData(currentTable);
        setShowPivot(false);
        setDebugInfo(hasSelection
          ? "Pivot 已写入当前 Sheet"
          : "未选择单元格，已从 A1 写入 Pivot 结果");
      }
    } catch (e: any) {
      console.error("Pivot failed", e);
      if (e?.message) {
        setDebugInfo(e.message);
      } else {
        setDebugInfo(formatPivotPersistError({ step: "batch_update", message: "Pivot 落库失败" }));
      }
    } finally {
      setLoading(false);
    }
  };

  const handleToolbarRefresh = () => {
    if (!currentTable) {
      // ### ????
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Clarify missing table; Purpose=avoid incorrect refresh prompt.
      setDebugInfo('Please select a table first');
      return;
    }
    gridRef.current?.refresh();
    fetchTableData(currentTable);
  };

  const handleStyleChange = async (style: any) => {
    if (!gridRef.current) {
      // ### 变更记录
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      setDebugInfo('Grid not ready, cannot apply style');
      return;
    }
    // ### 鍙樻洿璁板綍
    // - 2026-03-11 23:05: 鍘熷洜=Toolbar 鏍峰紡鎸夐挳姝ゅ墠鍙仛鎻愮ず; 鐩殑=鏀逛负璋冪敤 GlideGrid 鐪熷疄鏍峰紡鏇存柊 API銆?
    await gridRef.current.updateSelectionStyle(style);
    // ### Change Log
    // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep style updates readable.
    setDebugInfo(`Style applied: ${JSON.stringify(style)}`);
  };

  const handleMerge = async () => {
    if (!gridRef.current) {
      // ### ????
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Restore merge guard message; Purpose=avoid misleading debug text.
      setDebugInfo('Grid not ready, cannot merge');
      return;
    }
    // ### ??????
    // - 2026-03-11 23:05: ???=???????????????????? ???=??? GlideGrid ???????????
    await gridRef.current.mergeSelection();
    // ### Change Log
    // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep merge action readable.
    setDebugInfo('Merge requested');
  };

  const handleFreeze = () => {
    if (!gridRef.current) {
      // ### ????
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Restore freeze guard message; Purpose=avoid misleading debug text.
      setDebugInfo('Grid not ready, cannot freeze');
      return;
    }
    // ### ????
    // - 2026-03-11 23:05: ??=???????????; ??=?? GlideGrid ?????
    gridRef.current.toggleFreeze();
    // ### ????
    // - 2026-03-14: Reason=Fix garbled literal; Purpose=keep debug message readable.
    // ### Change Log
    // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep freeze action readable.
    setDebugInfo('Freeze toggled');
  };

  const handleFilter = (anchor?: DOMRect | null) => {
    if (!gridRef.current) {
      // ### ????
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Restore filter guard message; Purpose=avoid misleading debug text.
      setDebugInfo('Grid not ready, cannot filter');
      return;
    }
    // ### ????
    // - 2026-03-11 23:05: ??=????????????; ??=?? GlideGrid ????????
    gridRef.current.toggleFilter();
    if (anchor) {
      gridRef.current.openFilterMenuAt(anchor.left, anchor.bottom, 0);
    }
    // ### Change Log
    // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep filter action readable.
    setDebugInfo('Filter panel triggered');
  };

  const handleFormulaCommit = async () => {
    if (!gridRef.current || !selectedPosition) {
      // ### ????
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Replace non-ASCII prompt; Purpose=keep formula prompt readable.
      setDebugInfo('Please select a cell first');
      return;
    }
    // ### ????
    // - 2026-03-11 23:05: ??=?????????????; ??=???????????????
    await gridRef.current.updateCell(selectedPosition.col, selectedPosition.row, formulaText);
    // ### Change Log
    // - 2026-03-14: Reason=Replace non-ASCII debug text; Purpose=keep formula submit readable.
    setDebugInfo(`Formula submitted: ${formulaText}`);
  };

  const handleTimeMachineToggle = () => {
    if (!currentTable) {
      alert("Please select a table first");
      return;
    }
    setShowTimeMachine((prev) => !prev);
  };

  const handleSave = async () => {
    if (!currentTable) {
      // ### ????
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Clarify missing table; Purpose=avoid misleading save message.
      setDebugInfo('Please select a table first');
      return;
    }
    try {
      setLoading(true);
      const res = await fetch('/api/save_session', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ table_name: currentTable, session_id: sessionId || null })
      });
      if (res.ok) {
        const parsed = await parseJsonSafely(res);
        if (parsed.ok && parsed.data?.status === 'ok') {
          // ### Change Log
          // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep save result readable.
          setDebugInfo('Save completed');
          return;
        }
      }
      // ### Change Log
      // - 2026-03-14: Reason=Replace garbled debug text; Purpose=clarify save behavior.
      setDebugInfo('Backend has no explicit save endpoint; data is written in real time.');
    } catch (e: any) {
      // ### Change Log
      // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep save errors readable.
      setDebugInfo(`Save failed: ${e.message || 'unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const handleAddSheet = async (): Promise<string | null> => {
    if (!currentTable) {
      // ### 变更记录
      // - 2026-03-14: Reason=Fix garbled literal; Purpose=prevent unterminated string error.
      // ### Change Log
      // - 2026-03-14: Reason=Replace non-ASCII prompt; Purpose=keep create sandbox prompt readable.
      setDebugInfo('Please select a table before creating a sandbox');
      return null;
    }
    const nextSandboxName = buildNextSheetName(sessions);
    try {
      setLoading(true);
      const createRes = await fetch('/api/create_session', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          table_name: currentTable,
          session_name: nextSandboxName,
          // ### 鍙樻洿璁板綍
          // - 2026-03-14: 鍘熷洜=鍚庣瀛楁涓?from_session_id锛涚洰鐨?閬垮厤 base_session_id 瀵艰嚧鍒嗘敮缂哄け銆?
          from_session_id: sessionId || null
        })
      });
      if (!createRes.ok) {
        throw new Error(`create session failed: ${createRes.status}`);
      }
      const parsed = await parseJsonSafely(createRes);
      if (!parsed.ok) {
        throw new Error(parsed.reason);
      }
      const nextSessionId =
        parsed.data?.session?.session_id
        || parsed.data?.session_id
        || '';
      if (!nextSessionId) {
        throw new Error(parsed.data?.message || 'invalid create_session response');
      }
      // ### Change Log
      // - 2026-03-15: Reason=tsc rejects null; Purpose=pass undefined when session id missing
      await fetchSessionsForTable(currentTable, nextSessionId || undefined);
      // ### Change Log
      // - 2026-03-14: Reason=Replace non-ASCII debug text; Purpose=keep create sandbox result readable.
      setDebugInfo(`Sandbox created: ${nextSandboxName}`);
      return nextSessionId;
    } catch (e: any) {
      // ### Change Log
      // - 2026-03-14: Reason=Replace non-ASCII debug text; Purpose=keep create sandbox errors readable.
      setDebugInfo(`Create sandbox failed: ${e.message || 'unknown error'}`);
      return null;
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteSheet = async (sheet: string) => {
    if (!sheet) return;
    try {
      setLoading(true);
      let deleted = false;
      const deleteRes = await fetch('/api/delete_table', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ table_name: sheet })
      });
      if (deleteRes.ok) {
        const parsed = await parseJsonSafely(deleteRes);
        deleted = parsed.ok && parsed.data?.status === 'ok';
      }
      if (!deleted) {
        const fallbackRes = await fetch('/api/execute', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ sql: `DROP TABLE ${quoteSqlIdentifier(sheet)}` })
        });
        if (!fallbackRes.ok) {
          throw new Error(`delete table failed: ${fallbackRes.status}`);
        }
        const parsedFallback = await parseJsonSafely(fallbackRes);
        if (!parsedFallback.ok || parsedFallback.data?.error) {
          throw new Error(parsedFallback.ok ? parsedFallback.data.error : parsedFallback.reason);
        }
      }
      if (sheet === currentTable) {
        setCurrentTable('');
      }
      await fetchTables();
      // ### Change Log
      // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep delete result readable.
      setDebugInfo(`Table deleted: ${sheet}`);
    } catch (e: any) {
      // ### Change Log
      // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep delete errors readable.
      setDebugInfo(`Delete table failed: ${e.message || 'unknown error'}`);
    } finally {
      setLoading(false);
    }
  };


  // ### Change Log
  // - 2026-03-14: Reason=Expose window.app for E2E; Purpose=signal readiness for tests
  useEffect(() => {
    const appHandle = ensureWindowApp();
    if (!appHandle) return;
    Object.assign(appHandle, {
      selectTable,
      refreshTables,
      getTables: () => tables,
      getSessions: () => sessions,
      getCurrentSession: () => sessionId,
      isReadOnly: () => isReadOnly,
      createSession: async () => {
        const id = await handleAddSheet();
        return id || '';
      },
      switchSession: async (id: string) => {
        await handleSessionChange(id);
        return id;
      },
      // ### Change Log
      // - 2026-03-14: Reason=Use handleDeleteSheet to avoid unused warning; Purpose=expose delete for tests.
      deleteSheet: handleDeleteSheet,
      __ready: true
    });
  }, [selectTable, refreshTables, tables, sessions, sessionId, isReadOnly, handleAddSheet, handleSessionChange]);

  return (
    <div className="app-shell">
      {/* ### Change Log
          - 2026-03-15: Reason=Merge header + status bar; Purpose=single-row top layout
      */}
      <div className="status-header">
        {/* ### Change Log
            - 2026-03-15: Reason=Group left controls; Purpose=brand + selector + pivot in one row
        */}
        <div className="status-left" data-groups-left={getHeaderGroups().left.join(',')}>
          <div className="brand-block">
            {/* ### Change Log
                - 2026-03-15: Reason=Rename product; Purpose=display Tabula brand
            */}
            <span className="brand-title">{getBrandTitle()}</span>
            {/* ### Change Log
                - 2026-03-14: Reason=Replace garbled UI label; Purpose=keep brand tag readable.
            */}
            <span className="brand-tag">Sandbox</span>
          </div>

          {/* ### Change Log
              - 2026-03-14: Reason=Replace garbled table selector labels; Purpose=avoid unterminated strings.
          */}
          <select
            className="status-select"
            value={currentTable}
            onChange={handleTableChange}
            aria-label="Select table"
          >
            <option value="">Select a table...</option>
            {tables.map((tableName) => (
              <option key={tableName} value={tableName}>
                {tableName}
              </option>
            ))}
          </select>

          {/* ### Change Log
              - 2026-03-14: Reason=Replace garbled pivot label; Purpose=keep aria-label readable.
          */}
          <button
            type="button"
            className="pivot-trigger-btn"
            onClick={handlePivotToggle}
            title="Insert Pivot Table"
            aria-label="Insert pivot table"
          >
            Pivot
          </button>
        </div>

        {/* ### Change Log
            - 2026-03-15: Reason=Group right status; Purpose=keep fetching/debug on one row
        */}
        <div className="status-right" data-groups-right={getHeaderGroups().right.join(',')}>
          <span className="status-label">Fetching data: {tables.length || 0}, Page 1</span>
          <span className={`status-chip ${backendStatus.includes('Connected') ? 'ok' : 'error'}`}>
            {backendStatus}
          </span>
          {/* ### Change Log
              - 2026-03-14: Reason=Replace garbled loading text; Purpose=keep status readable.
          */}
          {loading && <span className="status-loading" aria-live="polite">Loading...</span>}

          <span className="status-debug" aria-live="polite">{debugInfo}</span>
        </div>
      </div>

      <Toolbar
        onRefresh={handleToolbarRefresh}
        onUndo={() => gridRef.current?.undo()}
        onRedo={() => gridRef.current?.redo()}
        onPivot={handlePivotToggle}
        onTimeMachine={handleTimeMachineToggle}
        onInsertFormula={() => {
          // ### Change Log
          // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep formula entry message readable.
          setDebugInfo('Formula input opened');
        }}
        onStyleChange={handleStyleChange}
        onMerge={handleMerge}
        onFreeze={handleFreeze}
        onSave={handleSave}
        onFilter={handleFilter}
        canUndo={canUndo}
        canRedo={canRedo}
        activeFilters={activeFilters}
        onRemoveFilter={(colId) => gridRef.current?.removeFilter(colId)}
      />

      <FormulaBar
        selectedCell={selectedCell}
        value={formulaText}
        onChange={setFormulaText}
        onCommit={handleFormulaCommit}
        onRefresh={handleToolbarRefresh}
        canRefresh={Boolean(currentTable)}
      />

      <main className="grid-stage">
        <div className="grid-frame">
          {/* ### 鍙樻洿璁板綍
              - 2026-03-11 23:05: 鍘熷洜=鏃х綉鏍煎叆鍙ｅ鑷存牱寮?绛涢€夎兘鍔涚己澶? 鐩殑=椤甸潰涓荤綉鏍肩粺涓€涓?GlideGrid銆?*/}
          {/* ### Change Log
              - 2026-03-14: Reason=Pass readOnly + session hooks; Purpose=keep grid state in sync with active session
          */}
          {currentTable ? (
            <GlideGrid
              ref={gridRef}
              sessionId={sessionId}
              tableName={currentTable}
              readOnly={isReadOnly}
              onSessionChange={(nextSessionId) => {
                setSessionId(nextSessionId);
                setIsReadOnly(Boolean(defaultSessionId && nextSessionId === defaultSessionId));
              }}
              onSelectionChange={(col, row, value) => {
                setSelectedPosition({ col, row });
                setSelectedCell(`${toExcelColumnLabel(col)}${row + 1}`);
                setFormulaText(value);
              }}
              onFilterChange={setActiveFilters}
              onStackChange={(undo, redo) => {
                setCanUndo(undo);
                setCanRedo(redo);
              }}
            />
          ) : (
            <div className="grid-empty-state" role="status" aria-live="polite">
              {/* ### Change Log
                  - 2026-03-14: Reason=Replace garbled empty-state text; Purpose=keep prompt readable.
              */}
              Select a table to start browsing.
            </div>
          )}
        </div>
      </main>

      <div className="sheet-footer">
        <SheetBar
          sessions={sessions}
          activeSessionId={sessionId}
          onSessionChange={handleSessionChange}
          onAddSession={handleAddSheet}
        />
      </div>

      {showPivot && (
        <PivotSidebar
          fields={pivotFields}
          config={pivotConfig}
          onConfigChange={setPivotConfig}
          onApply={handlePivotApply}
          onClose={() => setShowPivot(false)}
        />
      )}

      {showTimeMachine && currentTable && (
        <TimeMachineDrawer
          tableName={currentTable}
          onClose={() => setShowTimeMachine(false)}
          onCheckout={(version) => {
            // ### Change Log
            // - 2026-03-14: Reason=Replace garbled debug text; Purpose=keep time machine messages readable.
            setDebugInfo(`Checked out version: ${version}`);
            setShowTimeMachine(false);
          }}
        />
      )}

      <div className="debug-overlay">
        {debugInfo}
      </div>
    </div>
  );
}

export default App;













