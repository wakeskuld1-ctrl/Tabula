async function verifyBackendSessions() {
    try {
        console.log("Verifying backend loaded sessions from SQLite...");
        
        // 1. List tables to find a table with sessions
        const tablesResponse = await fetch('http://localhost:3000/api/tables');
        const tablesData = await tablesResponse.json();
        
        if (tablesData.status !== 'ok') {
            console.error("Failed to list tables");
            return;
        }
        
        const tables = tablesData.tables;
        console.log(`Found ${tables.length} tables.`);

        let foundSessions = false;
        
        for (const table of tables) {
            const tableName = table.table_name;
            try {
                const sessionsResponse = await fetch(`http://localhost:3000/api/sessions?table_name=${tableName}`);
                const sessionsData = await sessionsResponse.json();
                
                if (sessionsData.status === 'ok') {
                    const sessions = sessionsData.sessions;
                    if (sessions.length > 0) {
                        console.log(`Table '${tableName}' has ${sessions.length} sessions loaded.`);
                        console.log("Sample session:", sessions[0].session_id, sessions[0].name);
                        foundSessions = true;
                    }
                }
            } catch (e) {
                // Ignore errors for tables without sessions
            }
        }
        
        if (foundSessions) {
            console.log("SUCCESS: Backend successfully loaded sessions from SQLite!");
        } else {
            console.warn("WARNING: No sessions found via API. Migration might have failed or no sessions existed.");
        }

    } catch (error) {
        console.error("Error verifying backend sessions:", error.message);
    }
}

verifyBackendSessions();
