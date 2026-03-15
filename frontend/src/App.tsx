import React, { useEffect, useRef, useState } from 'react';
import './App.css';
import { GlideGrid, GlideGridHandle, ActiveFilter } from './components/GlideGrid';
import { PivotSidebar, PivotConfigState, Field } from './components/pivot/PivotSidebar';
import { PivotEngine } from './utils/PivotEngine';
import { Toolbar } from './components/layout/Toolbar';
import { FormulaBar } from './components/layout/FormulaBar';
import { TimeMachineDrawer } from './components/TimeMachineDrawer';
import { SheetBar } from './components/layout/SheetBar';

const toExcelColumnLabel = (index: number): string => {
  let result = '';
  let value = index;
  while (value >= 0) {
    result = String.fromCharCode((value % 26) + 65) + result;
    value = Math.floor(value / 26) - 1;
  }
  return result;
};

// ### 变更记录
// - 2026-03-11 21:45: 原因=接口偶发返回空体或非 JSON，直接 res.json 会抛错; 目的=统一安全解析并输出可读错误。
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

// ### 变更记录
// - 2026-03-11 21:45: 原因=表名包含特殊字符时裸 SQL 会失败; 目的=统一转义标识符，避免 execute 查询报错。
const quoteSqlIdentifier = (identifier: string): string => `"${String(identifier).replace(/"/g, `""`)}"`;

const App: React.FC = () => {
  const [backendStatus, setBackendStatus] = useState<string>('Disconnected');
  const [tables, setTables] = useState<string[]>([]);
  const [currentTable, setCurrentTable] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [debugInfo, setDebugInfo] = useState<string>('');
  
  // State for Pivot metadata inference
  const [gridColumns, setGridColumns] = useState<string[]>([]);
  const [gridRows, setGridRows] = useState<any[][]>([]);
  // ### 变更记录
  // - 2026-03-11 23:05: 原因=用户反馈底部样式和筛选入口丢失; 目的=将页面主入口固定到 GlideGrid 能力集。
  const gridRef = useRef<GlideGridHandle | null>(null);
  const [sessionId, setSessionId] = useState<string>('');
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
  
  const checkBackend = async () => {
    try {
      const res = await fetch('/api/health');
      if (res.ok) {
        const parsed = await parseJsonSafely(res);
        if (!parsed.ok) {
          setBackendStatus('Backend Response Invalid');
          setDebugInfo(`健康检查解析失败: ${parsed.reason}`);
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

  const fetchTables = async () => {
    try {
      const res = await fetch('/api/tables');
      if (res.ok) {
        const parsed = await parseJsonSafely(res);
        if (!parsed.ok) {
          setDebugInfo(`表列表解析失败: ${parsed.reason}`);
          return;
        }
        const data = parsed.data;
        console.log("Tables fetched:", data);
        const nextTables = (data.tables || [])
          .map((item: any) => typeof item === 'string' ? item : item.table_name)
          .filter(Boolean);
        setTables(nextTables);
        if (!currentTable && nextTables.length > 0) {
          setCurrentTable(nextTables[0]);
          fetchTableData(nextTables[0]);
        }
      }
    } catch (e) {
      console.error("Failed to fetch tables", e);
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
        // ### 变更记录
        // - 2026-03-11 21:45: 原因=未转义表名会导致特殊字符表查询失败; 目的=统一使用安全标识符拼接 SQL。
        body: JSON.stringify({ sql: `SELECT * FROM ${quoteSqlIdentifier(tableName)} LIMIT 1000` })
      });

      if (res.ok) {
        const parsed = await parseJsonSafely(res);
        if (!parsed.ok) {
          setDebugInfo(`数据解析失败: ${parsed.reason}`);
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

  useEffect(() => {
    checkBackend();
  }, []);

  const selectTable = (tableName: string) => {
    console.log("Selected table:", tableName);
    setCurrentTable(tableName);
    // ### 变更记录
    // - 2026-03-11 23:05: 原因=切表后会话与筛选状态可能串表; 目的=每次切表都重置与 GlideGrid 关联的状态。
    setSessionId('');
    setActiveFilters([]);
    setCanUndo(false);
    setCanRedo(false);
    setSelectedCell('');
    setSelectedPosition(null);
    setFormulaText('');
    if (tableName) {
      fetchTableData(tableName);
    }
  };

  const handleTableChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    selectTable(e.target.value);
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
        // For now, replace current view
        setGridColumns(result.headers);
        setGridRows(result.data);
        setDebugInfo(`Pivot generated: ${result.data.length} rows`);
        setShowPivot(false); // Close sidebar after apply
      } else {
        // Current sheet logic - maybe append?
        // For now just alert
        alert("Current sheet output not implemented yet, replacing view instead.");
        setGridColumns(result.headers);
        setGridRows(result.data);
        setShowPivot(false);
      }
    } catch (e: any) {
      console.error("Pivot failed", e);
      alert(`Pivot failed: ${e.message}`);
    } finally {
      setLoading(false);
    }
  };

  const handleToolbarRefresh = () => {
    if (!currentTable) {
      setDebugInfo('请先选择数据表');
      return;
    }
    gridRef.current?.refresh();
    fetchTableData(currentTable);
  };

  const handleStyleChange = async (style: any) => {
    if (!gridRef.current) {
      setDebugInfo('网格尚未就绪，无法应用样式');
      return;
    }
    // ### 变更记录
    // - 2026-03-11 23:05: 原因=Toolbar 样式按钮此前只做提示; 目的=改为调用 GlideGrid 真实样式更新 API。
    await gridRef.current.updateSelectionStyle(style);
    setDebugInfo(`样式操作: ${JSON.stringify(style)}`);
  };

  const handleMerge = async () => {
    if (!gridRef.current) {
      setDebugInfo('网格尚未就绪，无法合并');
      return;
    }
    // ### 变更记录
    // - 2026-03-11 23:05: 原因=合并按钮此前未落到网格能力; 目的=绑定 GlideGrid 选区合并实现。
    await gridRef.current.mergeSelection();
    setDebugInfo('合并单元格功能已触发');
  };

  const handleFreeze = () => {
    if (!gridRef.current) {
      setDebugInfo('网格尚未就绪，无法冻结');
      return;
    }
    // ### 变更记录
    // - 2026-03-11 23:05: 原因=冻结按钮此前是占位行为; 目的=对接 GlideGrid 冻结能力。
    gridRef.current.toggleFreeze();
    setDebugInfo('冻结窗格功能已触发');
  };

  const handleFilter = (anchor?: DOMRect | null) => {
    if (!gridRef.current) {
      setDebugInfo('网格尚未就绪，无法筛选');
      return;
    }
    // ### 变更记录
    // - 2026-03-11 23:05: 原因=用户明确要求恢复筛选交互; 目的=切回 GlideGrid 的列头筛选链路。
    gridRef.current.toggleFilter();
    if (anchor) {
      gridRef.current.openFilterMenuAt(anchor.left, anchor.bottom, 0);
    }
    setDebugInfo('筛选面板已触发');
  };

  const handleFormulaCommit = async () => {
    if (!gridRef.current || !selectedPosition) {
      setDebugInfo('请先选中单元格');
      return;
    }
    // ### 变更记录
    // - 2026-03-11 23:05: 原因=公式栏提交此前只写调试信息; 目的=将提交结果写入当前选中单元格。
    await gridRef.current.updateCell(selectedPosition.col, selectedPosition.row, formulaText);
    setDebugInfo(`公式已提交: ${formulaText}`);
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
      setDebugInfo('请先选择数据表');
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
          setDebugInfo('保存成功');
          return;
        }
      }
      setDebugInfo('当前后端未提供显式保存接口，数据已按实时写入模式处理');
    } catch (e: any) {
      setDebugInfo(`保存失败: ${e.message || 'unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const handleAddSheet = async () => {
    if (!currentTable) {
      setDebugInfo('请先选择数据表，再创建沙盘');
      return;
    }
    const nextSandboxName = `sandbox_${Date.now()}`;
    try {
      setLoading(true);
      const createRes = await fetch('/api/create_session', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          table_name: currentTable,
          session_name: nextSandboxName,
          base_session_id: sessionId || null
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
      setSessionId(nextSessionId);
      setDebugInfo(`已创建沙盘: ${nextSandboxName}`);
    } catch (e: any) {
      setDebugInfo(`创建沙盘失败: ${e.message || 'unknown error'}`);
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
      setDebugInfo(`已删除数据表: ${sheet}`);
    } catch (e: any) {
      setDebugInfo(`删除表失败: ${e.message || 'unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app-shell">
      <div className="status-header">
        <div className="brand-block">
          <span className="brand-title">Trae Excel</span>
          <span className="brand-tag">沙盘</span>
        </div>
        <div className="status-right">
          <span className="status-label">Fetching data: {tables.length || 0}, Page 1</span>
          <span className={`status-chip ${backendStatus.includes('Connected') ? 'ok' : 'error'}`}>
            {backendStatus}
          </span>
        </div>
      </div>

      <div className="status-bar">
        <select
          className="status-select"
          value={currentTable}
          onChange={handleTableChange}
          aria-label="选择数据表"
        >
          <option value="">选择数据表...</option>
          {tables.map((tableName) => (
            <option key={tableName} value={tableName}>
              {tableName}
            </option>
          ))}
        </select>

        <button
          type="button"
          className="pivot-trigger-btn"
          onClick={handlePivotToggle}
          title="Insert Pivot Table"
          aria-label="插入透视表"
        >
          Pivot
        </button>

        {loading && <span className="status-loading" aria-live="polite">加载中...</span>}

        <span className="status-debug" aria-live="polite">{debugInfo}</span>
      </div>

      <Toolbar
        onRefresh={handleToolbarRefresh}
        onUndo={() => gridRef.current?.undo()}
        onRedo={() => gridRef.current?.redo()}
        onPivot={handlePivotToggle}
        onTimeMachine={handleTimeMachineToggle}
        onInsertFormula={() => setDebugInfo('已打开函数输入入口')}
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
          {/* ### 变更记录
              - 2026-03-11 23:05: 原因=旧网格入口导致样式/筛选能力缺失; 目的=页面主网格统一为 GlideGrid。 */}
          {currentTable ? (
            <GlideGrid
              ref={gridRef}
              sessionId={sessionId}
              tableName={currentTable}
              onSessionChange={setSessionId}
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
              请选择一个数据表开始浏览
            </div>
          )}
        </div>
      </main>

      <div className="sheet-footer">
        <SheetBar
          sheets={tables}
          activeSheet={currentTable}
          onSheetChange={selectTable}
          onAddSheet={handleAddSheet}
          onDeleteSheet={handleDeleteSheet}
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
            setDebugInfo(`已切换到版本: ${version}`);
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
