import React, { useEffect, useState, useRef, useMemo } from 'react';
// import { Workbook } from "@fortune-sheet/react";
// import "@fortune-sheet/react/dist/index.css";
import './App.css';
import WasmGrid from './components/WasmGrid';
import formulaHelpData from "./data/formula_help.json";
import { filterFormulaHelpItems, FormulaHelpItem } from "./utils/formulaHelp";
// ### 变更记录
// - 2026-03-15 21:45: 原因=修复乱码并集中管理文案; 目的=统一公式帮助与状态栏文本
// - 2026-03-15 21:45: 原因=配合测试固定文本; 目的=避免回归
import { APP_LABELS } from "./utils/appLabels";

const App: React.FC = () => {
  const [backendStatus, setBackendStatus] = useState<string>('Disconnected');
  const [tables, setTables] = useState<string[]>([]);
  const [currentTable, setCurrentTable] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [sheetData, setSheetData] = useState<any[]>([{
    name: "Sheet1",
    celldata: [],
    order: 0,
    status: 1
  }]);

  const [debugInfo, setDebugInfo] = useState<string>('');

  // ### 变更记录
  // - 2026-03-15 00:50: 原因=前端需展示公式帮助; 目的=提供抽屉开关状态
  // - 2026-03-15 00:50: 原因=避免影响主流程; 目的=独立管理UI状态
  // - 2026-03-15 21:45: 原因=修复乱码并统一注释; 目的=确保状态语义清晰
  const [formulaHelpOpen, setFormulaHelpOpen] = useState<boolean>(false);
  // ### 变更记录
  // - 2026-03-15 00:50: 原因=支持搜索过滤; 目的=快速定位公式用法
  // - 2026-03-15 00:50: 原因=保证交互顺滑; 目的=输入即过滤
  // - 2026-03-15 21:45: 原因=修复乱码; 目的=保持注释可读
  const [formulaHelpQuery, setFormulaHelpQuery] = useState<string>("");
  
  // State for WasmGrid
  const [gridColumns, setGridColumns] = useState<string[]>([]);
  const [gridRows, setGridRows] = useState<any[][]>([]);

  // ### 变更记录
  // - 2026-03-15 00:50: 原因=公式提示数据源需类型化; 目的=保证字段安全
  // - 2026-03-15 00:50: 原因=JSON可能为空; 目的=避免运行期异常
  // - 2026-03-15 21:45: 原因=修复乱码并统一文案来源; 目的=避免显示异常
  const formulaHelpItems = useMemo<FormulaHelpItem[]>(() => {
    if (!Array.isArray(formulaHelpData)) {
      return [];
    }
    return formulaHelpData as FormulaHelpItem[];
  }, []);

  // ### 变更记录
  // - 2026-03-15 00:50: 原因=搜索过滤需要复用逻辑; 目的=统一过滤策略
  // - 2026-03-15 00:50: 原因=避免重复计算; 目的=提升渲染性能
  // - 2026-03-15 21:45: 原因=修复乱码; 目的=保证注释可读
  const filteredFormulaHelpItems = useMemo(() => {
    return filterFormulaHelpItems(formulaHelpItems, formulaHelpQuery);
  }, [formulaHelpItems, formulaHelpQuery]);
  
  // Responsive Dimensions
  const [dimensions, setDimensions] = useState({ 
    width: window.innerWidth - 40, // Subtract padding 
    height: window.innerHeight - 100 // Subtract header
  });

  useEffect(() => {
    const handleResize = () => {
        // Simple adjustment: Width - padding, Height - header height
        setDimensions({
            width: window.innerWidth - 40,
            height: window.innerHeight - 100
        });
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Expose state setters for E2E testing
  useEffect(() => {
    (window as any).setGridColumns = setGridColumns;
    (window as any).setGridRows = setGridRows;
  }, []);

  const checkBackend = async () => {
    try {
      const res = await fetch('/api/health');
      if (res.ok) {
        const data = await res.json();
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
        const data = await res.json();
        console.log("Tables fetched:", data);
        setTables(data.tables || []);
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
        body: JSON.stringify({ sql: `SELECT * FROM ${tableName} LIMIT 1000` })
      });

      if (res.ok) {
        const data = await res.json();
        console.log("Fetch response:", data);
        if (data.error) {
          alert(`Error: ${data.error}`);
          setDebugInfo(`Error: ${data.error}`);
        } else {
          setGridColumns(data.columns);
          setGridRows(data.rows);
          transformDataToSheet(data.columns, data.rows, tableName);
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

  const transformDataToSheet = (columns: string[], rows: string[][], tableName: string) => {
    console.log(`Transforming data: ${columns.length} cols, ${rows.length} rows`);
    setDebugInfo(`Loaded ${tableName}: ${rows.length} rows`);
    
    // Calculate dimensions
    const rowCount = Math.max(100, rows.length + 10);
    const colCount = Math.max(26, columns.length + 5);

    // Initialize 2D array for sheet data (better for filter/sort compatibility than celldata)
    const tableData = Array.from({ length: rowCount }, () => Array(colCount).fill(null));

    // Header Row (Row 0)
    columns.forEach((col, cIndex) => {
      tableData[0][cIndex] = {
        v: col,
        m: col,
        ct: { fa: "General", t: "g" },
        bg: "#f3f3f3", // Light gray background for headers
        bl: 1 // Bold
      };
    });

    // Data Rows (Row 1+)
    rows.forEach((row, rIndex) => {
      row.forEach((cellVal, cIndex) => {
        // Try to detect number
        let val: any = cellVal;
        let type = "s"; // string default
        if (!isNaN(Number(cellVal)) && cellVal.trim() !== "") {
           val = Number(cellVal);
           type = "n";
        }

        tableData[rIndex + 1][cIndex] = {
            v: val,
            m: String(val),
            ct: { fa: "General", t: type }
        };
      });
    });

    setSheetData([{
      name: tableName || "Sheet1",
      index: tableName || "Sheet1", // Unique index is important for internal state
      data: tableData, // Use dense 2D array 'data' instead of sparse 'celldata'
      celldata: undefined, // Clear celldata to avoid conflicts
      config: {}, // Ensure config object exists
      calcChain: [], // Ensure calcChain exists
      row: rowCount,
      column: colCount,
      order: 0,
      status: 1
    }]);
  };

  const workbookRef = useRef<any>(null);

  useEffect(() => {
    checkBackend();
  }, []);

  useEffect(() => {
     if (workbookRef.current) {
        console.log("Workbook Ref mounted:", workbookRef.current);
        // Expose to window for debugging if possible
        (window as any).fortune = workbookRef.current;
     }
  }, [currentTable]);

  const handleTableChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const tableName = e.target.value;
    console.log("Selected table:", tableName);
    setCurrentTable(tableName);
    if (tableName) {
      fetchTableData(tableName);
    }
  };

  const settings = React.useMemo(() => ({
    lang: 'zh',
    showinfobar: true,
    showtoolbar: true,
    showsheetbar: true,
    showstatisticBar: true,
    row: 100,
    column: 30,
    data: sheetData,
    onChange: (_data: any) => {
      // console.log("Data changed:", _data);
    }
  }), [sheetData]);

  return (
    <div style={{ width: '100%', height: '100vh', position: 'relative', display: 'flex', flexDirection: 'column' }}>
      <div className="status-bar" style={{ display: 'flex', gap: '10px', alignItems: 'center', padding: '10px', background: '#f0f0f0', borderBottom: '1px solid #ccc' }}>
        <select 
          value={currentTable} 
          onChange={handleTableChange}
          style={{ padding: '4px 8px' }}
        >
          {/* ### 变更记录
              - 2026-03-15 21:45: 原因=统一表选择占位文案; 目的=避免乱码与重复维护 */}
          <option value="">{APP_LABELS.table.placeholder}</option>
          {tables.map((t: any) => (
            <option key={t.table_name} value={t.table_name}>
              {t.table_name}
            </option>
          ))}
        </select>
        
        {/* ### 变更记录
            - 2026-03-15 21:45: 原因=统一加载提示文案; 目的=双语一致 */}
        {loading && <span style={{color: 'blue'}}>{APP_LABELS.loading.text}</span>}

        {/* ### 变更记录
            - 2026-03-15 00:50: 原因=提供公式帮助入口; 目的=支持用户直接查看用法
            - 2026-03-15 00:50: 原因=不影响主流程; 目的=按钮置于状态栏右侧
            - 2026-03-15 21:45: 原因=修复乱码并统一文案; 目的=避免重复字符串 */}
        <button
          type="button"
          className="formula-help-button"
          onClick={() => setFormulaHelpOpen(true)}
        >
          {APP_LABELS.formulaHelp.button}
        </button>

        <span style={{marginLeft: 'auto', fontSize: '12px'}}>
          <span style={{marginRight: '10px', color: '#666'}}>{debugInfo}</span>
          {/* ### 变更记录
              - 2026-03-15 21:45: 原因=统一后端状态前缀文案; 目的=双语一致 */}
          {APP_LABELS.backend.label} <span style={{ fontWeight: 'bold', color: backendStatus.includes('Connected') ? 'green' : 'red' }}>
            {backendStatus}
          </span>
        </span>
      </div>
      {/* Workbook Container */}
      <div style={{ flex: 1, position: 'relative', padding: '10px' }}>
         {/* <Workbook 
             ref={workbookRef}
             key={currentTable + sheetData[0].name} 
             {...settings} 
             showFormulaBar={true}
             showToolbar={true}
             allowEdit={true}
             style={{ width: '100%', height: '100%' }} 
          /> */}
          <WasmGrid width={dimensions.width} height={dimensions.height} columns={gridColumns} data={gridRows} />
      </div>

      {/* Debug Info Overlay */}
      <div style={{
        position: 'absolute',
        bottom: '5px',
        right: '20px',
        background: 'rgba(255,255,255,0.8)',
        padding: '2px 5px',
        fontSize: '10px',
        pointerEvents: 'none',
        zIndex: 9999
      }}>
        {debugInfo}
      </div>

      {/* ### 变更记录
          - 2026-03-15 00:50: 原因=公式帮助抽屉; 目的=提供全量用法提示
          - 2026-03-15 00:50: 原因=保持布局简洁; 目的=抽屉覆盖侧边不遮挡主内容
          - 2026-03-15 21:45: 原因=修复乱码并统一文案; 目的=避免重复字符串 */}
      {formulaHelpOpen && (
        <div className="formula-help-overlay">
          <div className="formula-help-drawer">
            <div className="formula-help-header">
              {/* ### 变更记录
                  - 2026-03-15 21:45: 原因=统一标题文案; 目的=双语一致 */}
              <div className="formula-help-title">{APP_LABELS.formulaHelp.title}</div>
              <button
                type="button"
                className="formula-help-close"
                onClick={() => setFormulaHelpOpen(false)}
              >
                {/* ### 变更记录
                    - 2026-03-15 21:45: 原因=统一关闭文案; 目的=双语一致 */}
                {APP_LABELS.formulaHelp.close}
              </button>
            </div>

            <div className="formula-help-search">
              {/* ### 变更记录
                  - 2026-03-15 21:45: 原因=统一搜索占位文案; 目的=双语一致 */}
              <input
                type="text"
                value={formulaHelpQuery}
                onChange={(e) => setFormulaHelpQuery(e.target.value)}
                placeholder={APP_LABELS.formulaHelp.searchPlaceholder}
              />
            </div>

            <div className="formula-help-body">
              {filteredFormulaHelpItems.length === 0 && (
                <div className="formula-help-empty">
                  {/* ### 变更记录
                      - 2026-03-15 21:45: 原因=统一空态文案; 目的=避免乱码与重复 */}
                  {APP_LABELS.formulaHelp.empty}
                </div>
              )}
              {filteredFormulaHelpItems.length > 0 && (
                <table className="formula-help-table">
                  <thead>
                    <tr>
                      {/* ### 变更记录
                          - 2026-03-15 21:45: 原因=统一表头文案; 目的=双语一致 */}
                      <th>{APP_LABELS.formulaHelp.headers.functionName}</th>
                      <th>{APP_LABELS.formulaHelp.headers.syntax}</th>
                      <th>{APP_LABELS.formulaHelp.headers.example}</th>
                      <th>{APP_LABELS.formulaHelp.headers.paramNotes}</th>
                      <th>{APP_LABELS.formulaHelp.headers.purpose}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredFormulaHelpItems.map((item) => (
                      <tr key={item.name}>
                        <td>{item.name}</td>
                        <td>{item.syntax}</td>
                        <td>{item.example}</td>
                        <td>{item.paramNotes}</td>
                        <td>{item.purpose}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
