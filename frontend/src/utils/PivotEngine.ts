
export interface PivotField {
    id: string;
    label: string;
    type?: string;
    agg?: 'sum' | 'count' | 'avg' | 'min' | 'max';
}

export interface PivotConfig {
    sourceTable: string;
    rows: PivotField[];
    columns: PivotField[];
    values: PivotField[];
    filters: any[];
}

export interface PivotResult {
    headers: string[];
    data: (string | number)[][];
}

export class PivotEngine {
    private static instance: PivotEngine;

    private constructor() {}

    public static getInstance(): PivotEngine {
        if (!PivotEngine.instance) {
            PivotEngine.instance = new PivotEngine();
        }
        return PivotEngine.instance;
    }

    public async query(config: PivotConfig): Promise<PivotResult> {
        console.log("Pivot Engine Query:", config);

        if (!config.sourceTable) {
            throw new Error("Source table is required");
        }

        const { sql, isMatrix } = this.generateSQL(config);
        console.log("Generated SQL:", sql);

        try {
            const response = await fetch('/api/execute', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ sql })
            });
            
            const result = await response.json();
            
            if (result.error) {
                throw new Error(result.error);
            }

            /* **[2026-02-26]** 变更原因：移除未使用的返回字段避免编译警告；变更目的：保持严格模式可通过。 */
            return this.processData(result.rows, config, isMatrix);
        } catch (e: any) {
            console.error("Pivot execution failed", e);
            throw new Error(e.message || "Failed to execute pivot query");
        }
    }

    private generateSQL(config: PivotConfig): { sql: string, isMatrix: boolean } {
        const tableName = `"${config.sourceTable}"`;
        
        // Fields for Group By (Identifiers only)
        const rowIdentifiers = config.rows.map(r => `"${r.id}"`);
        const colIdentifiers = config.columns.map(c => `"${c.id}"`);
        
        // Fields for Select (With Aliases)
        /* **[2026-03-10]** 变更原因：Pivot列名与Sidebar不一致；变更目的：使用Alias显示友好列名（如 "New Column" 而非 "newcol"）。 */
        const rowSelects = config.rows.map(r => `"${r.id}" AS "${r.label}"`);
        const colSelects = config.columns.map(c => `"${c.id}" AS "${c.label}"`);
        
        // Handle values with default aggregation and aliases
        const valueSelects = config.values.map(v => {
            const agg = v.agg || (v.type === 'number' ? 'SUM' : 'COUNT');
            /* **[2026-03-10]** 变更原因：聚合结果列名难读；变更目的：显示如 "Sum of Sales" 的格式。 */
            return `${agg}("${v.id}") AS "${agg} of ${v.label}"`;
        });

        // If no values, add a default count
        if (valueSelects.length === 0) {
            valueSelects.push('COUNT(*) AS "Count"');
        }

        const selectFields = [...rowSelects, ...colSelects, ...valueSelects];
        const groupByFields = [...rowIdentifiers, ...colIdentifiers];

        let sql = `SELECT ${selectFields.join(", ")} FROM ${tableName}`;

        // TODO: Handle Filters
        if (config.filters && config.filters.length > 0) {
            // Implement simple filtering if needed
            // sql += ` WHERE ...`
        }

        if (groupByFields.length > 0) {
            sql += ` GROUP BY ${groupByFields.join(", ")}`;
        }

        // Add limit to prevent browser crash on huge result
        sql += ` LIMIT 10000`;
        
        return { sql, isMatrix: config.columns.length > 0 };
    }

    /* **[2026-02-26]** 变更原因：columns 未使用导致告警；变更目的：收敛参数与实际逻辑。 */
    private processData(rows: any[][], config: PivotConfig, isMatrix: boolean): PivotResult {
        // If not a matrix (no columns), just return the flat result
        if (!isMatrix) {
            // Map headers: Rows + Values
            const headers = [...config.rows.map(r => r.label)];
            
            if (config.values.length === 1) {
                 headers.push(config.values[0].label);
            } else {
                 config.values.forEach(v => {
                     headers.push(`${v.label} (${v.agg || 'Count'})`);
                 });
            }

            if (config.values.length === 0) headers.push('Count'); // Default count header

            return {
                headers: headers,
                data: rows
            };
        }

        // Matrix Logic
        // We need to pivot:
        // Rows: unique combinations of row fields
        // Cols: unique combinations of col fields
        // Values: fill the cell

        const rowFieldCount = config.rows.length;
        const colFieldCount = config.columns.length;
        // Maps to store unique keys
        const rowKeys = new Map<string, any[]>(); // key -> row values
        const colKeys = new Map<string, any[]>(); // key -> col values

        // Helper to generate key
        const getKey = (vals: any[]) => vals.join('|||');

        // Simpler approach for dataMap: Map<rKey, Map<cKey, measureVals>>
        const matrixMap = new Map<string, Map<string, any[]>>();

        rows.forEach(row => {
            const rowVals = row.slice(0, rowFieldCount);
            const colVals = row.slice(rowFieldCount, rowFieldCount + colFieldCount);
            const measureVals = row.slice(rowFieldCount + colFieldCount);

            const rKey = getKey(rowVals);
            const cKey = getKey(colVals);

            if (!rowKeys.has(rKey)) rowKeys.set(rKey, rowVals);
            if (!colKeys.has(cKey)) colKeys.set(cKey, colVals);

            if (!matrixMap.has(rKey)) matrixMap.set(rKey, new Map());
            matrixMap.get(rKey)!.set(cKey, measureVals);
        });

        // Sort keys (optional, but good for display)
        const sortedRowKeys = Array.from(rowKeys.keys()).sort();
        const sortedColKeys = Array.from(colKeys.keys()).sort();

        // Build Headers
        // Row Headers + Column Headers (flattened)
        // For simplicity, we just use 1 row of headers
        // [Row Fields..., Col1_Val, Col2_Val...]
        // If multiple values, maybe Col1_Val1, Col1_Val2...
        
        const finalHeaders = [...config.rows.map(r => r.label)];
        
        sortedColKeys.forEach(cKey => {
            const colVals = colKeys.get(cKey)!;
            const colLabel = colVals.join('-');
            
            if (config.values.length > 1) {
                config.values.forEach(v => {
                    finalHeaders.push(`${colLabel} - ${v.label}`);
                });
            } else {
                finalHeaders.push(colLabel);
            }
        });

        // Build Data
        const finalData: (string | number)[][] = [];

        sortedRowKeys.forEach(rKey => {
            const rowVals = rowKeys.get(rKey)!;
            const rowData: (string | number)[] = [...rowVals];

            const rowCells = matrixMap.get(rKey);

            sortedColKeys.forEach(cKey => {
                const cellVals = rowCells?.get(cKey);
                
                if (config.values.length === 0) {
                    // Default count
                     rowData.push(cellVals ? cellVals[0] : 0);
                } else {
                    config.values.forEach((_, idx) => {
                        rowData.push(cellVals ? cellVals[idx] : 0);
                    });
                }
            });

            finalData.push(rowData);
        });

        return {
            headers: finalHeaders,
            data: finalData
        };
    }
}
