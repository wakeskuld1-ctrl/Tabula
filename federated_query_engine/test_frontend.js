const puppeteer = require('puppeteer');

(async () => {
    // Helper to replace deprecated waitForTimeout
    const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));

    console.log("Launching browser...");
    const browser = await puppeteer.launch({ 
        headless: "new", 
        args: ['--no-sandbox', '--disable-setuid-sandbox'] 
    });
    const page = await browser.newPage();
    
    // Capture browser console logs to debug frontend issues
    page.on('console', msg => console.log('BROWSER LOG:', msg.text()));
    page.on('pageerror', err => console.log('BROWSER ERROR:', err));

    try {
        console.log("Navigating to http://127.0.0.1:3000...");
        await page.goto('http://127.0.0.1:3000', { timeout: 60000 });

        // Wait for tree root to load
        console.log("Waiting for tree root...");
        await page.waitForSelector('#tree-root');

        // 1. Click "New Connection" button using ID
        console.log("Opening connection modal...");
        await page.waitForSelector('#btn-add-connection');
        await page.click('#btn-add-connection');

        // 2. Select Oracle
        console.log("Configuring Oracle connection...");
        await page.waitForSelector('#dbType', { visible: true });
        await page.select('#dbType', 'oracle');

        // Wait for form update (UI toggle)
        await delay(500);

        // 3. Fill details with CORRECT IDs
        console.log("Filling connection details...");
        await page.waitForSelector('#dbHost', { visible: true });
        
        // Clear and type Host
        await page.click('#dbHost', { clickCount: 3 });
        await page.keyboard.press('Backspace');
        await page.type('#dbHost', '192.168.23.3');

        // Port (should be default 1521 but good to check/set)
        await page.click('#dbPort', { clickCount: 3 });
        await page.keyboard.press('Backspace');
        await page.type('#dbPort', '1521');

        // User
        await page.click('#dbUser', { clickCount: 3 });
        await page.keyboard.press('Backspace');
        await page.type('#dbUser', 'tpcc');

        // Pass
        await page.click('#dbPass', { clickCount: 3 });
        await page.keyboard.press('Backspace');
        await page.type('#dbPass', 'tpcc');

        // Service
        await page.click('#dbService', { clickCount: 3 });
        await page.keyboard.press('Backspace');
        await page.type('#dbService', 'cyccbdata');

        // Ensure Table Name is EMPTY for connection mode
        await page.evaluate(() => document.getElementById('dbTable').value = '');

        // 4. Click Connect Button
        console.log("Clicking Connect button...");
        // Use the specific onclick attribute to target the correct button
        await page.waitForSelector('button[onclick="connectDatabase()"]');
        await page.click('button[onclick="connectDatabase()"]');

        // 5. Wait for success (Modal should hide)
        console.log("Waiting for connection to save...");
        await page.waitForFunction(() => {
            const modal = document.getElementById('db-connect-modal');
            return modal && modal.classList.contains('hidden');
        }, { timeout: 10000 });
        console.log("Modal closed. Connection saved.");

        // Debug: Fetch connections from API to see what backend has
        const conns = await page.evaluate(async () => {
            try {
                const res = await fetch('http://127.0.0.1:3000/api/connections');
                const data = await res.json();
                return data;
            } catch (e) {
                return { error: e.toString() };
            }
        });
        console.log("DEBUG: Connections on backend:", JSON.stringify(conns, null, 2));

        // 6. Verify Connection in Tree
        // Connection Name format: user@host:port/service
        const expectedConnName = "tpcc@192.168.23.3:1521/cyccbdata";
        console.log(`Looking for connection: ${expectedConnName}`);
        await delay(2000); // Wait for tree to stabilize

        // Find and expand "Oracle" folder if needed
        // Click the TOGGLE icon specifically
        const oracleToggle = await page.evaluateHandle(() => {
             const toggles = Array.from(document.querySelectorAll('#tree-root .tree-toggle'));
             // Find the one inside the Oracle header
             return toggles.find(t => t.parentElement.innerText.includes('Oracle'));
        });

        if (oracleToggle.asElement()) {
            console.log("Found Oracle folder toggle.");
            
            let connFound = await page.evaluate((name) => {
                return document.body.innerText.includes(name);
            }, expectedConnName);

            if (!connFound) {
                console.log("Connection not visible, expanding Oracle folder...");
                // Use evaluate to force click in DOM context
                await page.evaluate(el => el.click(), oracleToggle);
                await delay(2000); // Wait for toggle animation/render
            }
        } else {
            throw new Error("Oracle folder toggle not found");
        }

        // Debug: Print tree content
        const treeContent = await page.evaluate(() => document.getElementById('tree-root').innerText);
        console.log("Tree Content after expansion:\n", treeContent);
        
        const treeHTML = await page.evaluate(() => document.getElementById('tree-root').innerHTML);
        console.log("Tree HTML after expansion:\n", treeHTML);

        // 7. Click the connection to expand (Lazy Load)
        console.log("Expanding connection node...");
        const connHeader = await page.evaluateHandle((name) => {
            const headers = Array.from(document.querySelectorAll('#tree-root .tree-node'));
            return headers.find(h => h.innerText.includes(name));
        }, expectedConnName);

        if (connHeader.asElement()) {
            await page.evaluate(el => el.click(), connHeader);
            console.log("Clicked connection node.");
        } else {
            throw new Error(`Connection node '${expectedConnName}' not found`);
        }

        // 8. Wait for a table OR schema to appear
        console.log("Waiting for tables/schemas to load...");
        await delay(3000); // Give it time to fetch

        // Check if we have a Schema folder (e.g. "TPCC")
        const schemaName = "TPCC";
        const schemaHeader = await page.evaluateHandle((name) => {
            const headers = Array.from(document.querySelectorAll('#tree-root .tree-node'));
            return headers.find(h => h.innerText.includes(name));
        }, schemaName);

        if (schemaHeader.asElement()) {
            console.log(`Found Schema folder '${schemaName}', expanding...`);
            await page.evaluate(el => el.click(), schemaHeader);
            await delay(1000);
        }

        const tableFound = await page.evaluate(() => {
            // Check for specific table
            return document.body.innerText.includes("BMSQL_CONFIG");
        });

        if (tableFound) {
            console.log("SUCCESS: Tables listed under connection!");
        } else {
            console.log("WARNING: No tables found.");
            // Print tree content for debug
            const treeText = await page.evaluate(() => document.getElementById('tree-root').innerText);
            console.log("Tree Content:\n", treeText);
            throw new Error("Tables not loaded");
        }

    } catch (e) {
        console.error("Test Failed:", e);
        process.exit(1);
    } finally {
        await browser.close();
    }
})();
