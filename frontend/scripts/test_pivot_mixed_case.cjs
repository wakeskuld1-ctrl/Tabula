const http = require('http');

function runQuery(sql, callback) {
    const query = JSON.stringify({ sql });
    const req = http.request({
        hostname: 'localhost',
        port: 3000,
        path: '/api/execute',
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Content-Length': query.length
        }
    }, (res) => {
        let data = '';
        res.on('data', (chunk) => { data += chunk; });
        res.on('end', () => {
            try {
                const json = JSON.parse(data);
                callback(null, json);
            } catch (e) {
                callback(e, null);
            }
        });
    });
    req.on('error', (e) => callback(e, null));
    req.write(query);
    req.end();
}

// 1. Create table with mixed case column
console.log("Creating table MixedOrders...");
// Use IF NOT EXISTS or drop first to avoid error on rerun
runQuery('DROP TABLE IF EXISTS "MixedOrders"', (err, res) => {
    runQuery('CREATE TABLE "MixedOrders" ("NewCol" INT, "region" VARCHAR)', (err, res) => {
        if (err) return console.error("Create failed:", err);
        
        // 2. Insert data
        console.log("Inserting data...");
        runQuery("INSERT INTO \"MixedOrders\" VALUES (100, 'North'), (200, 'South')", (err, res) => {
            if (err) return console.error("Insert failed:", err);

            // 3. Test Pivot Query WITH ALIASES
            console.log("Testing Pivot Query with ALIASES...");
            const pivotSql = 'SELECT "NewCol" AS "New Column", COUNT("region") AS "Count of Region" FROM "MixedOrders" GROUP BY "NewCol" LIMIT 100';
            runQuery(pivotSql, (err, res) => {
                if (err) return console.error("Pivot failed:", err);
                if (res.error) {
                    console.error("❌ Pivot query with ALIASES failed:", res.error);
                } else {
                    console.log("✅ Pivot query with ALIASES successful!");
                    console.log("Pivot Columns:", res.columns);
                    
                    if (res.columns.includes("New Column") && res.columns.includes("Count of Region")) {
                         console.log("✅ Column Aliases verified correctly.");
                    } else {
                         console.error("❌ Column Aliases mismatch! Got:", res.columns);
                    }
                }
            });
        });
    });
});
