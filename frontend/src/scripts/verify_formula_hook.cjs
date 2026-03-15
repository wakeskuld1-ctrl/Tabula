const puppeteer = require('puppeteer');
const path = require('path');

(async () => {
    const browser = await puppeteer.launch({
        headless: false,
        defaultViewport: null,
        args: [
            '--start-maximized',
            '--no-sandbox',
            '--disable-setuid-sandbox',
            '--no-proxy-server'
        ]
    });

    const page = await browser.newPage();
    
    // Helper to take screenshots
    const takeScreenshot = async (name) => {
        const screenshotPath = path.join(__dirname, `${name}.png`);
        await page.screenshot({ path: screenshotPath, fullPage: true });
        console.log(`Saved screenshot: ${screenshotPath}`);
    };

    try {
        // **[2026-02-26]** 变更原因：端口硬编码导致脚本不可移植
        // **[2026-02-26]** 变更目的：统一从环境变量读取端口
        // **[2026-02-26]** 变更原因：保持日志一致性
        // **[2026-02-26]** 变更目的：便于排查
        const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
        const BASE_URL = `http://127.0.0.1:${PORT}`;
        console.log(`Navigating to app (${BASE_URL})...`);
        await page.goto(BASE_URL, { waitUntil: 'networkidle2', timeout: 60000 });

        // Select a table if not selected
        console.log("Checking for table selection...");
        try {
            await page.waitForSelector('select', { timeout: 10000 });
            const options = await page.$$eval('select option', opts => opts.map(o => o.value).filter(v => v));
            if (options.length > 0) {
                console.log(`Selecting table: ${options[0]}`);
                await page.select('select', options[0]);
            } else {
                console.warn("No tables found in selector!");
            }
        } catch (e) {
            console.log("Select element not found or timeout, assuming Grid might be loaded directly.");
        }

        // Wait for grid
        console.log("Waiting for canvas...");
        await page.waitForSelector('canvas', { timeout: 60000 });
        
        // Ensure window.grid is available
        console.log("Waiting for window.grid...");
        await page.waitForFunction(() => window.grid !== undefined, { timeout: 10000 });
        console.log('Grid detected.');

        // Focus canvas
        console.log("Focusing canvas...");
        await page.click('canvas');
        await new Promise(r => setTimeout(r, 500));

        // Test Case 1: Trigger Edit Mode and Check Suggestions
        console.log('\n--- Test Case 1: Trigger Edit Mode and Check Suggestions ---');
        
        // Select cell (2, 5)
        await page.evaluate(() => {
            window.grid.setSelection({
                current: {
                    cell: [2, 5],
                    range: { x: 2, y: 5, width: 1, height: 1 },
                    rangeStack: []
                },
                columns: { items: [] },
                rows: { items: [] }
            });
        });
        
        await new Promise(r => setTimeout(r, 500));

        // Press Enter to start editing
        console.log("Pressing Enter...");
        await page.keyboard.press('Enter');
        await new Promise(r => setTimeout(r, 1000));

        // Verify input exists
        const inputSelector = 'input.gdg-input';
        let inputExists = await page.$(inputSelector);
        
        if (!inputExists) {
            console.log("Input not found after Enter, trying to type '=' directly...");
            await page.keyboard.type('=');
            await new Promise(r => setTimeout(r, 1000));
            inputExists = await page.$(inputSelector);
        }

        if (!inputExists) {
             console.error("FAIL: Could not enter edit mode. Input element not found.");
             // Try double clicking relative to canvas
             const canvasBox = await page.$eval('canvas', c => {
                 const rect = c.getBoundingClientRect();
                 return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
             });
             // Click somewhat inside top left (assuming (2,5) is visible)
             // Grid usually has headers. Let's try 100, 100 relative to canvas.
             console.log("Trying double click at 200, 200...");
             await page.mouse.dblclick(canvasBox.x + 200, canvasBox.y + 200);
             await new Promise(r => setTimeout(r, 1000));
             inputExists = await page.$(inputSelector);
        }

        if (inputExists) {
             console.log("Edit mode active. Clearing and typing '='...");
             // Clear input (Ctrl+A, Backspace)
             await page.click(inputSelector);
             await page.keyboard.down('Control');
             await page.keyboard.press('A');
             await page.keyboard.up('Control');
             await page.keyboard.press('Backspace');
             await page.type(inputSelector, '=');
        } else {
             throw new Error("Failed to enter edit mode.");
        }
        
        await new Promise(r => setTimeout(r, 1000));
        await takeScreenshot('hook_test_1_typing_eq');

        // Check for suggestions
        // In FormulaEditor.tsx: <div>Suggested Formulas</div>
        const suggestionsSelector = 'div'; // We need to find the specific div
        const suggestionsVisible = await page.evaluate(() => {
            const divs = Array.from(document.querySelectorAll('div'));
            return divs.some(d => d.innerText.includes('Suggested Formulas'));
        });

        if (suggestionsVisible) {
            console.log('PASS: Suggestions popup appeared.');
        } else {
            console.error('FAIL: Suggestions popup did NOT appear.');
        }

        // Test Case 2: Filter Suggestions
        console.log('\n--- Test Case 2: Filter Suggestions ---');
        await page.keyboard.type('SU'); // Should filter to SUM, SUBSTITUTE, etc.
        await new Promise(r => setTimeout(r, 500));
        
        // Verify SUM is visible
        const sumVisible = await page.evaluate(() => {
            const divs = Array.from(document.querySelectorAll('div'));
            return divs.some(d => d.innerText === 'SUM');
        });
        
        if (sumVisible) {
            console.log('PASS: "SUM" suggestion found.');
        } else {
            console.error('FAIL: "SUM" suggestion NOT found.');
        }
        await takeScreenshot('hook_test_2_filtered');

        // Test Case 3: Keyboard Navigation & Application
        console.log('\n--- Test Case 3: Keyboard Navigation & Application ---');
        // Assuming SUM is first or near top. Press Down to select next?
        // Let's just press Enter on the current selection (usually the first one, or we can press Down once).
        await page.keyboard.press('ArrowDown');
        await new Promise(r => setTimeout(r, 200));
        await page.keyboard.press('Enter');
        await new Promise(r => setTimeout(r, 500));

        // Check input value. Should be '=SUM(' or similar depending on what was selected.
        // Note: applySuggestion appends '('
        const inputValue = await page.$eval(inputSelector, el => el.value);
        console.log(`Input value after Enter: "${inputValue}"`);
        
        if (inputValue.includes('(')) {
             console.log('PASS: Suggestion applied successfully.');
        } else {
             console.error('FAIL: Suggestion NOT applied correctly.');
        }
        await takeScreenshot('hook_test_3_applied');

        // Test Case 4: FX Button Popup
        console.log('\n--- Test Case 4: FX Button Popup ---');
        // Click the 'fx' button. 
        // Use mouse.down/up because component uses onMouseDown
        const fxBtn = await page.$('div[title="Insert Function"]');
        if (fxBtn) {
            const box = await fxBtn.boundingBox();
            await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
            await page.mouse.down();
            await new Promise(r => setTimeout(r, 100));
            await page.mouse.up();
            console.log("Clicked FX button via mouse down/up");
        } else {
            console.error("FAIL: FX button not found");
        }
        
        await new Promise(r => setTimeout(r, 1000));
        
        const popupVisible = await page.evaluate(() => {
            const spans = Array.from(document.querySelectorAll('span'));
            return spans.some(s => s.innerText.trim() === 'Insert Function');
        });

        if (popupVisible) {
             console.log('PASS: FX Popup appeared.');
        } else {
             console.error('FAIL: FX Popup did NOT appear.');
        }
        await takeScreenshot('hook_test_4_fx_popup');

    } catch (error) {
        console.error('Test failed:', error);
        await takeScreenshot('hook_test_failure');
    } finally {
        await browser.close();
    }
})();
