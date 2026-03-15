
export interface GridDataResponse {
    status: string;
    message?: string;
    data: any[][];
    columns: string[];
    column_types: string[];
    total_rows: number;
    metadata: Record<string, any>;
    formula_columns: any[];
}

// ### Change Log
// - 2026-03-15: Reason=Align API error shape; Purpose=attach status/body for callers
const buildHttpError = (label: string, status: number, body: string) => {
    const error = new Error(`${label} failed: ${status} ${body}`);
    // ### Change Log
    // - 2026-03-15: Reason=Callers need status; Purpose=preserve http code for UI messaging
    (error as Error & { status?: number }).status = status;
    // ### Change Log
    // - 2026-03-15: Reason=Keep server payload; Purpose=surface backend error details
    (error as Error & { body?: string }).body = body;
    return error;
};

async function executeSql(sql: string): Promise<any> {
    const res = await fetch('/api/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sql })
    });
    if (!res.ok) {
        const text = await res.text();
        console.error(`[GridAPI] SQL Execution Failed: ${res.status} ${text}`);
        throw new Error(`SQL execution failed: ${res.status} ${res.statusText} - ${text}`);
    }
    return await res.json();
}

export async function fetchGridData(tableName: string, page: number, pageSize: number): Promise<GridDataResponse> {
    const offset = (page - 1) * pageSize;
    
    // Parallel execution for count and data
    const [countRes, dataRes] = await Promise.all([
        executeSql(`SELECT COUNT(*) FROM "${tableName}"`),
        executeSql(`SELECT * FROM "${tableName}" LIMIT ${pageSize} OFFSET ${offset}`)
    ]);

    const totalRows = countRes.rows?.[0]?.[0] ? parseInt(countRes.rows[0][0]) : 0;
    
    return {
        status: 'ok',
        data: dataRes.rows || [],
        columns: dataRes.columns || [],
        column_types: Array(dataRes.columns?.length || 0).fill('utf8'),
        total_rows: totalRows,
        metadata: {},
        formula_columns: []
    };
}

export async function fetchFilterValues(tableName: string, column: string, searchText: string, limit = 200, offset = 0) {
    let sql = `SELECT DISTINCT "${column}" FROM "${tableName}"`;
    if (searchText) {
        sql += ` WHERE CAST("${column}" AS TEXT) LIKE '%${searchText}%'`;
    }
    sql += ` ORDER BY "${column}" LIMIT ${limit} OFFSET ${offset}`;

    const res = await executeSql(sql);
    const values = res.rows?.map((r: any[]) => r[0]) || [];
    return { status: 'ok', values };
}

export async function updateCell(payload: any) {
    const res = await fetch('/api/update_cell', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(`Update failed: ${res.status} ${text}`);
    }
    return await res.json();
}

export async function batchUpdateCells(payload: any) {
    const res = await fetch('/api/batch_update_cells', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(`Batch update failed: ${res.status} ${text}`);
    }
    return await res.json();
}

// ### Change Log
// - 2026-03-15: Reason=Time machine needs unified routing; Purpose=align versions fetch with GridAPI
export async function fetchVersions(tableName: string) {
    const res = await fetch(`/api/versions?table_name=${encodeURIComponent(tableName)}`);
    if (!res.ok) {
        const text = await res.text();
        throw buildHttpError("Fetch versions", res.status, text);
    }
    return await res.json();
}

// ### Change Log
// - 2026-03-15: Reason=Style range route alignment; Purpose=centralize update_style_range calls
export async function updateStyleRange(payload: any) {
    const res = await fetch('/api/update_style_range', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
    });
    if (!res.ok) {
        const text = await res.text();
        throw buildHttpError("Update style range", res.status, text);
    }
    return await res.json();
}

// ### Change Log
// - 2026-03-15: Reason=Pivot writes can exceed schema; Purpose=centralize ensure_columns route
export async function ensureColumns(payload: any) {
    const res = await fetch('/api/ensure_columns', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
    });
    if (!res.ok) {
        const text = await res.text();
        throw buildHttpError("Ensure columns", res.status, text);
    }
    return await res.json();
}
