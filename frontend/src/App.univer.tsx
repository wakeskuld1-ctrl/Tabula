import React, { useEffect, useRef } from 'react';
import { Univer, LocaleType, UniverInstanceType } from '@univerjs/core';
import { defaultTheme } from '@univerjs/design';
import { UniverDocsPlugin } from '@univerjs/docs';
import { UniverDocsUIPlugin } from '@univerjs/docs-ui';
import { UniverFormulaEnginePlugin, FunctionType } from '@univerjs/engine-formula';
import { UniverRenderEnginePlugin } from '@univerjs/engine-render';
import { UniverSheetsPlugin } from '@univerjs/sheets';
import { UniverSheetsFormulaPlugin } from '@univerjs/sheets-formula';
import { UniverSheetsNumfmtPlugin } from '@univerjs/sheets-numfmt';
import { UniverSheetsUIPlugin } from '@univerjs/sheets-ui';
import { UniverUIPlugin } from '@univerjs/ui';

// Import Locale Data
import DesignEnUS from '@univerjs/design/lib/locale/en-US.json';
import UIEnUS from '@univerjs/ui/lib/locale/en-US.json';
import DocsUIEnUS from '@univerjs/docs-ui/lib/locale/en-US.json';
import SheetsEnUS from '@univerjs/sheets/lib/locale/en-US.json';
import SheetsUIEnUS from '@univerjs/sheets-ui/lib/locale/en-US.json';
import SheetsNumfmtEnUS from '@univerjs/sheets-numfmt/lib/locale/en-US.json';

import './App.css';
import "@univerjs/design/lib/index.css";
import "@univerjs/ui/lib/index.css";
import "@univerjs/docs-ui/lib/index.css";
import "@univerjs/sheets-ui/lib/index.css";
import "@univerjs/sheets-numfmt/lib/index.css";

const App: React.FC = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const univerRef = useRef<Univer | null>(null);

  const [status, setStatus] = React.useState<string>('Initializing Univer...');
  const [backendStatus, setBackendStatus] = React.useState<string>('Disconnected');
  const [error, setError] = React.useState<string | null>(null);

  const checkBackend = async () => {
    try {
      const res = await fetch('/api/health');
      if (res.ok) {
        const data = await res.json();
        setBackendStatus(`Connected (v${data.version})`);
      } else {
        setBackendStatus('Backend Error');
      }
    } catch (e) {
      setBackendStatus('Backend Unreachable');
    }
  };

  useEffect(() => {
    checkBackend();
  }, []);

  useEffect(() => {
    if (!containerRef.current || univerRef.current) return;

    try {
      console.log('Starting Univer Init...');
      
      // Check Reflect Polyfill
      if (!Reflect || !Reflect.getMetadata) {
        throw new Error('Reflect.getMetadata is not defined. Polyfill missing?');
      }
      console.log('Reflect Polyfill detected.');

      console.log('Plugins:', { 
        Render: UniverRenderEnginePlugin, 
        Formula: UniverFormulaEnginePlugin, 
        UI: UniverUIPlugin, 
        Sheets: UniverSheetsPlugin 
      });

      // Initialize Univer
      const univer = new Univer({
        theme: defaultTheme,
        locale: LocaleType.EN_US,
        locales: {
          [LocaleType.EN_US]: {
            ...DesignEnUS,
            ...UIEnUS,
            ...DocsUIEnUS,
            ...SheetsEnUS,
            ...SheetsUIEnUS,
            ...SheetsNumfmtEnUS,
          }
        },
      });

      // Register Plugins - Minimal Set
      console.log('Registering Plugin: Render');
      univer.registerPlugin(UniverRenderEnginePlugin);
      
      console.log('Registering Plugin: Formula');
      univer.registerPlugin(UniverFormulaEnginePlugin);
      
      // Critical: UI Plugin needs explicit container
      console.log('Registering Plugin: UI');
      univer.registerPlugin(UniverUIPlugin, {
        container: containerRef.current,
        header: true,
        footer: true,
      });

      console.log('Registering Plugin: Docs');
      univer.registerPlugin(UniverDocsPlugin, {
        hasScroll: false,
      });
      console.log('Registering Plugin: DocsUI');
      univer.registerPlugin(UniverDocsUIPlugin);
      
      console.log('Registering Plugin: Sheets');
      univer.registerPlugin(UniverSheetsPlugin);
      console.log('Registering Plugin: SheetsUI');
      univer.registerPlugin(UniverSheetsUIPlugin);
      console.log('Registering Plugin: SheetsNumfmt');
      univer.registerPlugin(UniverSheetsNumfmtPlugin);

      const functionList = [
        {
          functionName: 'SUM',
          aliasFunctionName: 'SUM',
          functionType: FunctionType.Math,
          description: 'Adds all the numbers in a range of cells.',
          abstract: 'SUM(number1, [number2], ...)',
          functionParameter: [
            {
              name: 'number1',
              detail: 'The first number you want to add.',
              example: '1, A1',
              require: 1,
              repeat: 0,
            },
            {
              name: 'number2',
              detail: 'Additional numbers to add.',
              example: '2, A2',
              require: 0,
              repeat: 1,
            },
          ],
        },
        {
          functionName: 'AVERAGE',
          aliasFunctionName: 'AVERAGE',
          functionType: FunctionType.Statistical,
          description: 'Returns the average of its arguments.',
          abstract: 'AVERAGE(number1, [number2], ...)',
          functionParameter: [
            {
              name: 'number1',
              detail: 'The first number, cell reference, or range for which you want the average.',
              example: '1, A1',
              require: 1,
              repeat: 0,
            },
            {
              name: 'number2',
              detail: 'Additional numbers, cell references or ranges for which you want the average.',
              example: '2, A2',
              require: 0,
              repeat: 1,
            },
          ],
        }
      ];

      console.log('Registering Plugin: SheetsFormula');
      univer.registerPlugin(UniverSheetsFormulaPlugin, {
          description: functionList,
      });

      // Create Initial Workbook
      console.log('Creating Unit (SHEET)...');
      univer.createUnit(UniverInstanceType.UNIVER_SHEET, {
        id: 'workbook-1',
        name: 'Federated Query Results',
        sheetOrder: ['sheet-1'],
        sheets: {
          'sheet-1': {
            id: 'sheet-1',
            name: 'Query Data',
            cellData: {
              0: {
                0: { v: 'ID', t: 1 },
                1: { v: 'Transaction', t: 1 },
                2: { v: 'Amount', t: 1 },
                3: { v: 'Status', t: 1 },
              },
              1: {
                0: { v: 1001, t: 2 },
                1: { v: 'TXN_UNIVER_01', t: 1 },
                2: { v: 5000.00, t: 2 },
                3: { v: 'Completed', t: 1 },
              },
              2: {
                0: { v: 1002, t: 2 },
                1: { v: 'TXN_UNIVER_02', t: 1 },
                2: { v: 120.50, t: 2 },
                3: { v: 'Pending', t: 1 },
              }
            }
          }
        }
      });

      univerRef.current = univer;
      setStatus('Univer Initialized Successfully');
    } catch (err: any) {
      console.error('Univer Init Error:', err);
      setError(err.toString());
      setStatus('Initialization Failed');
    }

    // Cleanup not fully supported in this version of Univer, but good practice
    return () => {
      // univer.dispose(); 
    };
  }, []);

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
       <div style={{ height: '40px', background: '#f3f4f6', display: 'flex', alignItems: 'center', padding: '0 16px', borderBottom: '1px solid #e5e7eb', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center' }}>
            <span style={{ fontWeight: 600 }}>Federated Query Engine</span>
            <span style={{ marginLeft: '12px', fontSize: '12px', background: '#dbeafe', color: '#1e40af', padding: '2px 8px', borderRadius: '12px' }}>
              Univer Enterprise Mode (v0.4.2)
            </span>
          </div>
          <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
             <div style={{ fontSize: '12px', color: backendStatus.includes('Connected') ? 'green' : 'orange' }}>
               Backend: {backendStatus}
             </div>
             <div style={{ fontSize: '12px', color: error ? 'red' : 'green' }}>
               Univer: {status}
             </div>
          </div>
       </div>
       {error && (
         <div style={{ padding: '20px', background: '#fee2e2', color: '#b91c1c', borderBottom: '1px solid #ef4444' }}>
           <strong>Error:</strong> {error}
           <br/>
           Check console for details.
         </div>
       )}
       <div ref={containerRef} style={{ flex: 1, position: 'relative', border: '1px solid #e5e7eb' }} />
    </div>
  );
};

export default App;
