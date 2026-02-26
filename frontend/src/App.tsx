import React, { useEffect, useState, useRef } from 'react';
// import { Workbook } from "@fortune-sheet/react";
// import "@fortune-sheet/react/dist/index.css";
import './App.css';
import WasmGrid from './components/WasmGrid';

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
  
  // State for WasmGrid
  const [gridColumns, setGridColumns] = useState<string[]>([]);
  const [gridRows, setGridRows] = useState<any[][]>([]);
  
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
          <option value="">选择数据表...</option>
          {tables.map((t: any) => (
            <option key={t.table_name} value={t.table_name}>
              {t.table_name}
            </option>
          ))}
        </select>
        
        {loading && <span style={{color: 'blue'}}>加载中...</span>}

        <span style={{marginLeft: 'auto', fontSize: '12px'}}>
          <span style={{marginRight: '10px', color: '#666'}}>{debugInfo}</span>
          后端状态: <span style={{ fontWeight: 'bold', color: backendStatus.includes('Connected') ? 'green' : 'red' }}>
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
    </div>
  );
}

export default App;
