const puppeteer = require('puppeteer');

(async () => {
    console.log('Starting E2E verification for Oracle Pushdown...');
    const browser = await puppeteer.launch({
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });
    const page = await browser.newPage();

    try {
        console.log('Navigating to app...');
        await page.goto('http://localhost:3000', { waitUntil: 'networkidle0' });

        // Wait for Monaco Editor to be ready
        console.log('Waiting for editor...');
        await page.waitForFunction(() => window.monaco && window.monaco.editor.getModels().length > 0, { timeout: 10000 });

        // Set SQL Query (Correct TPCC Join)
        const sql = `SELECT t1.W_NAME, t2.D_NAME 
FROM BMSQL_WAREHOUSE t1 
JOIN BMSQL_DISTRICT t2 ON t1.W_ID = t2.D_W_ID 
WHERE t1.W_ID = 1`;

        console.log('Setting SQL:', sql);
        await page.evaluate((query) => {
            window.monaco.editor.getModels()[0].setValue(query);
        }, sql);

        // Click Run Button
        console.log('Clicking Run...');
        const runBtnSelector = 'button[onclick="executeQuery()"]';
        await page.waitForSelector(runBtnSelector);
        await page.click(runBtnSelector);

        // Wait for results
        console.log('Waiting for execution (5s)...');
        await new Promise(r => setTimeout(r, 5000));

        // Take screenshot
        console.log('Taking screenshot...');
        await page.screenshot({ path: 'e2e_result.png', fullPage: true });

        // Verify Logs for Pushdown
        console.log('Checking logs...');
        const logs = await page.evaluate(async () => {
            const response = await fetch('http://localhost:3000/api/logs');
            const data = await response.json();
            return data.logs || [];
        });

        const pushdownLog = logs.find(l => 
            l.includes('[Oracle PushDown] Generated Oracle SQL') && 
            l.includes('WHERE') && 
            l.includes(':1')
        );

        if (pushdownLog) {
            console.log('✅ SUCCESS: Pushdown confirmed!');
            console.log('Log Entry:', pushdownLog);
        } else {
            console.error('❌ FAILURE: No pushdown log found.');
            console.log('Recent Logs:', logs.slice(-10));
            process.exit(1);
        }

    } catch (e) {
        console.error('❌ ERROR:', e);
        process.exit(1);
    } finally {
        await browser.close();
        console.log('Browser closed.');
    }
})();
