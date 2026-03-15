const puppeteer = require('puppeteer');

(async () => {
  console.log("Starting Cross-Sheet Formula E2E Verification...");

  const browser = await puppeteer.launch({
    headless: "new",
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  const page = await browser.newPage();

  try {
    // 0. Fetch Schema Info first to get correct column names
    // Note: Since we are running in node, we can use fetch (Node 18+) or just use page.evaluate later.
    // Let's use page.evaluate to fetch from backend via the browser context to avoid CORS/network issues if any.
    // But we need to navigate first.
    
    // 1. Load the App
    console.log("Navigating to app...");
    
    page.on('console', msg => console.log('PAGE LOG:', msg.text()));
    page.on('pageerror', err => console.log('PAGE ERROR:', err.toString()));
    
    const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
    const URL = `http://localhost:${PORT}`;
    await page.goto(URL, { waitUntil: 'networkidle0' });

    // Fetch schema info
    const schemaInfo = await page.evaluate(async () => {
        const res = await fetch('/api/tables');
        const data = await res.json();
        const usersTable = data.tables.find(t => t.table_name === 'users');
        const ordersTable = data.tables.find(t => t.table_name === 'orders');
        
        if (!usersTable || !ordersTable) return null;
        
        const usersCol = JSON.parse(usersTable.schema_json)[0].name;
        const ordersCol = JSON.parse(ordersTable.schema_json)[0].name;
        
        return { usersCol, ordersCol };
    });
    
    if (!schemaInfo) {
        throw new Error("Could not find 'users' or 'orders' table info");
    }
    
    console.log(`Using columns: users.${schemaInfo.usersCol}, orders.${schemaInfo.ordersCol}`);

    // 2. Ensure 'users' table is loaded (to populate FormulaEngine)
    console.log("Selecting 'users' table...");
    await page.evaluate(() => {
        if (window.app && window.app.selectTable) {
            window.app.selectTable('users');
        } else {
            console.error("window.app.selectTable not found!");
        }
    });

    // Wait for Grid to load
    console.log("Waiting for grid (users)...");
    await page.waitForSelector('canvas');
    await new Promise(r => setTimeout(r, 2000)); // Wait for data load & HF sync

    // 3. Ensure 'orders' table is loaded
    console.log("Selecting 'orders' table...");
    await page.evaluate(() => {
        window.app.selectTable('orders');
    });

    // Wait for Grid to load
    console.log("Waiting for grid (orders)...");
    await new Promise(r => setTimeout(r, 2000)); // Wait for data load & HF sync

    // 4. Verify Sheets in SheetBar
    console.log("Waiting for sheet tabs...");
    await page.waitForSelector('.sheet-tab');
    
    const tabs = await page.$$('.sheet-tab');
    console.log(`Found ${tabs.length} tabs.`);
    const tabNames = [];
    for (const tab of tabs) {
        tabNames.push(await page.evaluate(el => el.textContent, tab));
    }
    console.log("Tabs:", tabNames);

    if (!tabNames.includes('users') || !tabNames.includes('orders')) {
        throw new Error("Missing expected tabs (users, orders)");
    }

    // 5. Set Formula in 'orders' (current) to reference 'users'
    // Switch back to users
    await page.evaluate(() => window.app.selectTable('users'));
    await new Promise(r => setTimeout(r, 1000));

    console.log("Setting users!A1 = 100");
    await page.evaluate(async () => {
        if (window.grid && window.grid.updateCell) {
            await window.grid.updateCell(0, 0, "100");
        } else {
            throw new Error("window.grid.updateCell not available");
        }
    });
    
    // Wait for update & session switch
    await new Promise(r => setTimeout(r, 2000));

    // Debug: Check what's actually in the grid/cache
    const usersA1Value = await page.evaluate(() => {
        // We can access the internal cache via the component instance if exposed, 
        // or just check the FormulaEngine state if we can reach it.
        // Assuming FormulaEngine is a singleton we can reach:
        // Note: FormulaEngine is not exposed on window, but we can try to return what's visible if we could select the cell.
        // Better: let's inspect the network response via request interception? 
        // Or simpler: use the exposed calculate method to see what FormulaEngine has.
        if (window.app && window.app.formulaEngine) {
             return window.app.formulaEngine.calculate("=A1", 0, 0, "users");
        }
        return "N/A";
    });
    console.log(`Debug users!A1 via FormulaEngine: ${usersA1Value}`);

    // Switch to orders
    await page.evaluate(() => window.app.selectTable('orders'));
    await new Promise(r => setTimeout(r, 1000));

    console.log("Setting orders!A1 = =users!A1 * 2");
    await page.evaluate(async () => {
        if (window.grid && window.grid.updateCell) {
            await window.grid.updateCell(0, 0, "=users!A1 * 2");
        } else {
            throw new Error("window.grid.updateCell not available");
        }
    });

    // Switch to orders table
    console.log("Switching to orders table...");
    await page.evaluate(() => {
        const tabs = Array.from(document.querySelectorAll('.sheet-tab'));
        const orderTab = tabs.find(t => t.textContent.includes('orders'));
        if (orderTab) {
            // @ts-ignore
            orderTab.click();
        } else {
            throw new Error("Orders tab not found");
        }
    });
    
    // Wait for data load
    await new Promise(r => setTimeout(r, 2000));

    // Debug: Check FormulaEngine state
    await page.evaluate(() => {
        console.log("Debug: Checking FormulaEngine state...");
        // @ts-ignore
        const engine = window.app.formulaEngine;
        if (engine) {
            // Use public debug methods if available, or try to access hf if we must (but prefer exposed methods)
            // We added getSheetNames and getRawValue to FormulaEngine
            if (engine.getSheetNames) {
                 console.log("Debug: Sheet Names:", engine.getSheetNames());
            }
            
            if (engine.getRawValue) {
                const usersVal = engine.getRawValue(0, 0, 'users');
                console.log(`Debug: users!A1 raw value in engine: ${usersVal}`);
                
                const ordersVal = engine.getRawValue(0, 0, 'orders');
                console.log(`Debug: orders!A1 raw value in engine: ${ordersVal}`);
            } else {
                 console.log("Debug: getRawValue not found on engine");
            }
        } else {
            console.log("Debug: formulaEngine instance not found on window.app");
        }
    });

    // Verify Formula Result in orders!A1
    const result = await page.evaluate(() => {
        // @ts-ignore
        const engine = window.FormulaEngine.getInstance();
        return engine.calculate('=users!A1 * 2', 0, 0, 'orders');
    });

    console.log("Formula Result:", result);

    if (result === '200') {
        console.log("SUCCESS: Cross-sheet formula calculated correctly!");
    } else {
        console.error(`FAILURE: Expected 200, got ${result}`);
        // Debug: check users!A1
        const userVal = await page.evaluate(() => {
             // @ts-ignore
             const engine = window.FormulaEngine.getInstance();
             return engine.calculate('=users!A1', 0, 0, 'orders');
        });
        console.log("Debug users!A1 from orders:", userVal);
        process.exit(1);
    }

  } catch (e) {
    console.error("Test Error:", e);
    process.exit(1);
  } finally {
    await browser.close();
  }
})();
