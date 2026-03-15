const fs = require('fs');
const path = require('path');

const BASE_URL = 'http://localhost:3000/api';

async function runTest() {
    console.log("Starting Migration & Atomic Create Table Verification...");

    // Helper for fetch
    const get = async (url) => {
        const res = await fetch(url);
        if (!res.ok) throw new Error(`GET ${url} failed: ${res.statusText}`);
        return await res.json();
    };
    
    const post = async (url, body) => {
        const res = await fetch(url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body)
        });
        if (!res.ok) {
             const txt = await res.text();
             throw new Error(`POST ${url} failed: ${res.status} ${txt}`);
        }
        return await res.json();
    };

    // 1. Check connectivity
    try {
        await get(`${BASE_URL}/health`);
        console.log("✅ Backend is reachable.");
    } catch (e) {
        console.error("❌ Backend not reachable. Is it running?");
        return;
    }

    // 2. List existing sessions for 'users' table (should be loaded from SQLite)
    try {
        console.log("\n[Test 1] Listing sessions for 'users'...");
        const res = await get(`${BASE_URL}/sessions?table_name=users`);
        console.log("Sessions:", res.sessions.length);
        if (res.sessions.length > 0) {
            console.log("✅ Existing sessions loaded successfully.");
            console.log("Sample Session:", res.sessions[0].session_id);
        } else {
            console.log("⚠️ No sessions found for 'users'. Did migration run?");
        }
    } catch (e) {
        console.error("❌ Failed to list sessions:", e.message);
    }

    // 3. Create a NEW Table (Atomic Transaction Test)
    const newTableName = `test_table_${Date.now()}`;
    console.log(`\n[Test 2] Creating new table '${newTableName}'...`);
    
    try {
        const createRes = await post(`${BASE_URL}/create_table`, {
            table_name: newTableName
        });
        
        if (createRes.status === 'ok') {
            console.log("✅ Create Table API success.");
            const session = createRes.session;
            console.log("New Session ID:", session.session_id);
            console.log("Lance Path:", session.lance_path);
            
            // Verify file exists
            if (fs.existsSync(session.lance_path)) {
                console.log("✅ Lance file created on disk.");
            } else {
                console.error("❌ Lance file NOT found on disk!");
            }
            
            // 4. Verify it's in the list
            const listRes = await get(`${BASE_URL}/sessions?table_name=${newTableName}`);
            const found = listRes.sessions.find(s => s.session_id === session.session_id);
            if (found) {
                console.log("✅ New session found in session list (Persisted to SQLite).");
            } else {
                console.error("❌ New session NOT found in list!");
            }
            
            // 5. Verify Query
            // Since it's registered, we should be able to query it
            const queryRes = await post(`${BASE_URL}/execute`, {
                sql: `SELECT * FROM ${newTableName}`
            });
            if (queryRes.execution_time_ms !== undefined) {
                 console.log("✅ Query successful (Empty result expected).");
            } else {
                 console.error("❌ Query failed:", queryRes);
            }

        } else {
            console.error("❌ Create Table failed:", createRes);
        }
    } catch (e) {
        console.error("❌ Create Table Request failed:", e.message);
    }
}

runTest();
