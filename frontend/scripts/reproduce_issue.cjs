// Stress Test Script for Tabula
// Scenarios: Concurrency, Race Condition, Sequential Chain, Large Payload

const BASE_URL = 'http://127.0.0.1:3000/api';

async function updateCell(tableName, sessionId, row, col, val) {
    const res = await fetch(`${BASE_URL}/update_cell`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            table_name: tableName,
            row_idx: row,
            col_idx: col,
            col_name: 'id', // Assuming 'id' is col 0
            old_value: '',
            new_value: String(val),
            session_id: sessionId
        })
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(`Update failed: ${res.status} ${res.statusText} - ${text}`);
    }
    return await res.json();
}

async function getGridData(tableName, sessionId) {
    const res = await fetch(`${BASE_URL}/grid-data?session_id=${sessionId}&table_name=${tableName}&page=1&page_size=100`);
    if (!res.ok) throw new Error(`Get Grid failed: ${res.status}`);
    return await res.json();
}

async function run() {
    console.log('🚀 Starting Backend Stress Test...');
    
    // 1. Setup
    const tableName = `stress_test_${Date.now()}`;
    console.log(`\n[Setup] Creating table ${tableName}...`);
    let res = await fetch(`${BASE_URL}/create_table`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ table_name: tableName })
    });
    let json = await res.json();
    let currentSessionId = json.session.session_id;
    console.log(`[Setup] Initial Session: ${currentSessionId}`);

    // --- Scenario 1: Concurrency Storm ---
    console.log('\n⚡ [Test 1] Concurrency Storm (20 parallel updates)');
    console.log('Sending 20 requests simultaneously to the SAME base session...');
    // Note: Since each update forks the session, parallel updates to the SAME session_id 
    // effectively ask for 20 different branches starting from the same point.
    
    const stormPromises = [];
    for(let i=0; i<20; i++) {
        stormPromises.push(
            updateCell(tableName, currentSessionId, i, 0, `Storm_${i}`)
                .then(r => ({status: 'fulfilled', val: r, idx: i}))
                .catch(e => ({status: 'rejected', val: e, idx: i}))
        );
    }
    
    const stormResults = await Promise.all(stormPromises);
    const successCount = stormResults.filter(r => r.status === 'fulfilled').length;
    console.log(`[Test 1] Completed. Success: ${successCount}/20`);
    
    if (successCount > 0) {
        // Just pick the last successful one to continue, 
        // essentially "choosing a timeline"
        const lastSuccess = stormResults.find(r => r.status === 'fulfilled').val;
        console.log(`[Test 1] Picking session timeline: ${lastSuccess.session_id}`);
        currentSessionId = lastSuccess.session_id;
    } else {
        console.error('❌ [Test 1] All concurrent requests failed!');
    }

    // --- Scenario 2: Race Condition (Same Cell) ---
    console.log('\n🏎️ [Test 2] Race Condition (2 requests, same cell, same base session)');
    const raceValA = "RACE_A";
    const raceValB = "RACE_B";
    
    // Both try to modify Row 0, Col 0 from the current session
    const raceResults = await Promise.allSettled([
        updateCell(tableName, currentSessionId, 0, 0, raceValA),
        updateCell(tableName, currentSessionId, 0, 0, raceValB)
    ]);
    
    let raceSessionA = null;
    let raceSessionB = null;

    raceResults.forEach((r, idx) => {
        if (r.status === 'fulfilled') {
            console.log(`[Test 2] Req ${idx} Success -> Session: ${r.value.session_id}`);
            if (idx === 0) raceSessionA = r.value.session_id;
            if (idx === 1) raceSessionB = r.value.session_id;
        } else {
            console.log(`[Test 2] Req ${idx} Failed: ${r.reason}`);
        }
    });

    // If both succeeded, we have two divergent timelines (A and B). 
    // We should check if they contain the correct values respectively.
    if (raceSessionA) {
        const dataA = await getGridData(tableName, raceSessionA);
        console.log(`[Test 2] Timeline A Value: ${dataA.data[0][0]} (Expected: ${raceValA})`);
        currentSessionId = raceSessionA; // Continue with A
    }
    if (raceSessionB) {
        const dataB = await getGridData(tableName, raceSessionB);
        console.log(`[Test 2] Timeline B Value: ${dataB.data[0][0]} (Expected: ${raceValB})`);
    }

    // --- Scenario 3: Formula Chain (Sequential) ---
    console.log('\n🔗 [Test 3] Formula Chain (5 sequential updates)');
    // Strictly sequential: Update -> Get New Session -> Update -> ...
    let chainSessionId = currentSessionId;
    for(let i=1; i<=5; i++) {
        try {
            const r = await updateCell(tableName, chainSessionId, 0, 0, `Seq_${i}`);
            chainSessionId = r.session_id;
            process.stdout.write(`Step ${i} OK (${r.session_id.substring(0,8)}) -> `);
        } catch (e) {
            console.error(`\n❌ [Test 3] Failed at step ${i}: ${e}`);
            break;
        }
    }
    console.log(`\n[Test 3] Final Session: ${chainSessionId}`);
    const chainData = await getGridData(tableName, chainSessionId);
    console.log(`[Test 3] Final Value: ${chainData.data[0][0]} (Expected: Seq_5)`);
    currentSessionId = chainSessionId;

    // --- Scenario 4: Payload Stress ---
    console.log('\n🐘 [Test 4] Payload Stress (10MB string)');
    const bigPayload = 'X'.repeat(10 * 1024 * 1024); // 10MB
    const start = Date.now();
    try {
        // Update Row 1, Col 0
        const bigRes = await updateCell(tableName, currentSessionId, 1, 0, bigPayload);
        const dur = Date.now() - start;
        console.log(`[Test 4] Write Success in ${dur}ms. New Session: ${bigRes.session_id}`);
        
        // Read back
        const readStart = Date.now();
        const bigGrid = await getGridData(tableName, bigRes.session_id);
        const readDur = Date.now() - readStart;
        
        // Find the row (might be offset if data is sparse or page size matters, but we requested page size 100)
        // Row 1 is the second row usually, but grid-data might return sparse array or dense? 
        // Assuming dense or matching index.
        const readVal = bigGrid.data[1][0]; 
        
        if (readVal === bigPayload) {
            console.log(`[Test 4] Read Verification SUCCESS (${readVal.length} bytes) in ${readDur}ms`);
        } else {
            console.log(`[Test 4] Read Verification FAILED. Got length: ${readVal ? readVal.length : 'undefined'}`);
        }
    } catch (e) {
        console.error(`❌ [Test 4] Failed: ${e}`);
    }

    console.log('\n✅ Stress Test Complete.');
}

run().catch(console.error);
