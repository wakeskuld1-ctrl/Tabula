const http = require('http');

async function executeSql(sql) {
    return new Promise((resolve, reject) => {
        const req = http.request({
            hostname: '127.0.0.1',
            port: 3000,
            path: '/api/execute',
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            }
        }, (res) => {
            let data = '';
            res.on('data', (chunk) => data += chunk);
            res.on('end', () => {
                try {
                    resolve(JSON.parse(data));
                } catch (e) {
                    reject(e);
                }
            });
        });

        req.on('error', reject);
        req.write(JSON.stringify({ sql }));
        req.end();
    });
}

async function verify() {
    try {
        console.log("Verifying migration...");
        
        // 1. Check sessions table
        const sessionsRes = await executeSql("SELECT session_id, table_name FROM sessions LIMIT 5");
        if (sessionsRes.error) {
            console.error("Error querying sessions:", sessionsRes.error);
        } else {
            console.log(`Found ${sessionsRes.rows.length} sessions in SQLite.`);
            console.log("Raw response columns:", sessionsRes.columns);
            console.log("Raw response rows:", sessionsRes.rows);
        }

        // 2. Check sheet_attributes table
        const attrRes = await executeSql("SELECT session_id, attr_type FROM sheet_attributes LIMIT 5");
        if (attrRes.error) {
            console.error("Error querying sheet_attributes:", attrRes.error);
        } else {
            console.log(`Found ${attrRes.rows.length} sheet attributes in SQLite.`);
            console.log("Raw response rows:", attrRes.rows);
        }
        
    } catch (e) {
        console.error("Verification failed:", e);
    }
}

verify();
