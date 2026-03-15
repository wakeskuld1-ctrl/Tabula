
const PivotEngine = {
    processData: function(rows, config, isMatrix) {
        // Mocking the logic from PivotEngine.ts
        if (!isMatrix) {
            const headers = [...config.rows.map(r => r.label), ...config.values.map(v => `${v.agg || 'Count'} of ${v.label}`)];
            if (config.values.length === 0) headers.push('Count');
            return headers;
        }

        const rowFieldCount = config.rows.length;
        const colFieldCount = config.columns.length;
        const rowKeys = new Map();
        const colKeys = new Map();

        const getKey = (vals) => vals.join('|||');

        rows.forEach(row => {
            const rowVals = row.slice(0, rowFieldCount);
            const colVals = row.slice(rowFieldCount, rowFieldCount + colFieldCount);
            
            const rKey = getKey(rowVals);
            const cKey = getKey(colVals);

            if (!rowKeys.has(rKey)) rowKeys.set(rKey, rowVals);
            if (!colKeys.has(cKey)) colKeys.set(cKey, colVals);
        });

        const sortedColKeys = Array.from(colKeys.keys()).sort();
        const finalHeaders = [...config.rows.map(r => r.label)];
        
        sortedColKeys.forEach(cKey => {
            const colVals = colKeys.get(cKey);
            const colLabel = colVals.join('-');
            
            if (config.values.length > 1) {
                config.values.forEach(v => {
                    finalHeaders.push(`${colLabel} - ${v.label}`);
                });
            } else {
                finalHeaders.push(colLabel);
            }
        });

        return finalHeaders;
    }
};

// Test Case 1: Non-Matrix (Flat)
const config1 = {
    rows: [{ label: 'Region' }],
    columns: [],
    values: [{ label: 'Sales', agg: 'Sum' }]
};
console.log('Test 1 (Flat):', PivotEngine.processData([], config1, false));

// Test Case 2: Matrix
const config2 = {
    rows: [{ label: 'Region' }],
    columns: [{ label: 'Year' }],
    values: [{ label: 'Sales', agg: 'Sum' }]
};
const rows2 = [
    ['North', '2023', 100],
    ['South', '2024', 200]
];
console.log('Test 2 (Matrix):', PivotEngine.processData(rows2, config2, true));
