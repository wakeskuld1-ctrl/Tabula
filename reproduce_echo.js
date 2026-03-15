
const fs = require('fs');
const path = require('path');

const BASE_URL = 'http://127.0.0.1:3000';
const TABLE_NAME = 'test_echo';

async function run() {
    try {
        // 1. Create a dummy CSV file/table
        console.log("1. Creating test table...");
        if (!fs.existsSync('data')) fs.mkdirSync('data');
        fs.writeFileSync('data/test_echo.csv', 'id,name\n1,Alice\n2,Bob');
        
        // Register it
        await fetch(`${BASE_URL}/api/register_table`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                table_name: TABLE_NAME,
                file_path: process.cwd() + '\\data\\test_echo.csv',
                source_type: 'csv'
            })
        });

        // 2. Hydrate/Create Session
        console.log("2. Hydrating session...");
        await fetch(`${BASE_URL}/api/hydrate`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                table_name: TABLE_NAME,
                file_path: process.cwd() + '\\data\\test_echo.csv'
            })
        });

        // Get initial data
        let res = await fetch(`${BASE_URL}/api/grid-data?table_name=${TABLE_NAME}&page=1&page_size=10`);
        let data = await res.json();
        console.log("Initial data row 1 name:", data.data[0][1]); // Alice

        // 3. Update Cell
        const newVal = "Alice_" + Date.now();
        console.log(`3. Updating cell to '${newVal}'...`);
        
        res = await fetch(`${BASE_URL}/api/update_cell`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                table_name: TABLE_NAME,
                row_idx: 0,
                col_name: 'name',
                col_idx: 1,
                old_value: 'Alice',
                new_value: newVal
            })
        });
        
        data = await res.json();
        if (data.status !== 'ok') {
            console.error("Update failed:", data);
            return;
        }
        
        const sessionId = data.session_id;
        console.log("Update success. Session ID:", sessionId);

        // 4. Fetch Grid Data IMMEDIATELY (Simulating frontend fetch)
        console.log("4. Fetching grid data immediately...");
        res = await fetch(`${BASE_URL}/api/grid-data?table_name=${TABLE_NAME}&page=1&page_size=10&session_id=${sessionId}`);
        data = await res.json();
        
        const fetchedVal = data.data[0][1];
        console.log("Fetched data row 1 name:", fetchedVal);
        
        if (fetchedVal === newVal) {
            console.log("SUCCESS: Data echoed correctly from memory.");
        } else {
            console.error("FAILURE: Data mismatch! Expected", newVal, "got", fetchedVal);
            console.log("This indicates the backend is serving stale data (likely from file).");
        }

    } catch (e) {
        console.error("Error:", e);
    }
}

run();
