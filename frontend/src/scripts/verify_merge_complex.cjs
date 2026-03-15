const puppeteer = require('puppeteer');
const path = require('path');
const fs = require('fs');

(async () => {
    const browser = await puppeteer.launch({
        headless: true, // Set to false to see what's happening
        defaultViewport: { width: 1280, height: 800 },
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });

    const page = await browser.newPage();
    
    // Helper to take screenshot with timestamp
    const takeScreenshot = async (name) => {
        const filename = `${name}.png`;
        const filepath = path.join(__dirname, filename);
        await page.screenshot({ path: filepath });
        console.log(`Screenshot saved: ${filepath}`);
    };

    try {
        console.log("Navigating to app...");
        
        // Listen to dialogs (alerts) and dismiss them
        page.on('dialog', async dialog => {
            console.log(`Dialog detected: ${dialog.message()}`);
            await dialog.dismiss();
        });

        // Listen to console logs
        page.on('console', msg => console.log('PAGE LOG:', msg.text()));
        
        // **[2026-02-26]** 变更原因：端口硬编码导致脚本不可移植
        // **[2026-02-26]** 变更目的：统一从环境变量读取端口
        // **[2026-02-26]** 变更原因：保持脚本一致性
        // **[2026-02-26]** 变更目的：降低维护成本
        const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
        const BASE_URL = `http://localhost:${PORT}`;
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
            console.log("Select element not found or timeout, assuming Grid might be loaded directly or error.");
        }

        // Wait for Grid to load
        console.log("Waiting for canvas...");
        try {
            await page.waitForSelector('canvas', { timeout: 60000 });
        } catch (e) {
            console.error("Canvas not found, taking debug screenshot");
            await takeScreenshot('error_no_canvas');
            throw e;
        }
        console.log("Grid loaded.");
        await takeScreenshot('1_initial_load');

        // Wait for window.grid to be exposed
        console.log("Waiting for window.grid...");
        await page.waitForFunction(() => !!window.grid);
        console.log("window.grid is ready.");

        // Use a random start row to avoid conflicts with previous runs (since backend persists data)
        const START_ROW = 100 + Math.floor(Math.random() * 1000);
        console.log(`Using Test Start Row: ${START_ROW}`);

        // --- Scenario 1: Merge Empty Area & Edit ---
        console.log("--- Scenario 1: Empty Area Merge & Edit ---");
        
        // Select an empty area (Row START_ROW, Col 1-2)
        await page.evaluate((startRow) => {
            const grid = window.grid;
            if (grid) {
                // Select B{startRow+1}:C{startRow+1}
                grid.setSelection({
                    current: {
                        cell: [1, startRow],
                        range: { x: 1, y: startRow, width: 2, height: 1 },
                        rangeStack: []
                    },
                    columns: { items: [] },
                    rows: { items: [] }
                });
            }
        }, START_ROW);
        await new Promise(r => setTimeout(r, 500));
        await takeScreenshot('2_selection_empty_area');

        // Click Merge Button
        console.log("Clicking Merge button...");
        await page.evaluate(async () => {
             const grid = window.grid;
             if (grid) await grid.mergeSelection();
        });
        await new Promise(r => setTimeout(r, 1000)); // Wait for merge
        await takeScreenshot('3_merged_empty_area');

        // Edit the merged cell
        console.log("Editing merged cell...");
        await page.evaluate(async (startRow) => {
             const grid = window.grid;
             if (grid) {
                 // Update cell (Row startRow, Col 1)
                 await grid.updateCell(1, startRow, "MergedValue");
             }
        }, START_ROW);
        await new Promise(r => setTimeout(r, 1000)); // Wait for update
        await takeScreenshot('4_edited_empty_merge');

        // --- Scenario 2: Background Color Inheritance ---
        console.log("--- Scenario 2: Background Color Inheritance ---");
        
        // Set background color
        const COLOR_ROW = START_ROW + 2;
        await page.evaluate(async (row) => {
             const grid = window.grid;
             if (grid) {
                 await grid.updateStyle(3, row, { bg_color: "#ffcccc" }); // Light Red
             }
        }, COLOR_ROW);
        await new Promise(r => setTimeout(r, 500));
        
        // Select range (2x2)
        await page.evaluate((row) => {
            const grid = window.grid;
            if (grid) {
                grid.setSelection({
                    current: {
                        cell: [3, row],
                        range: { x: 3, y: row, width: 2, height: 2 },
                        rangeStack: []
                    },
                    columns: { items: [] },
                    rows: { items: [] }
                });
            }
        }, COLOR_ROW);
        await new Promise(r => setTimeout(r, 500));
        await takeScreenshot('5_selection_color_test');

        // Merge
        await page.evaluate(async () => {
             const grid = window.grid;
             if (grid) await grid.mergeSelection();
        });
        await new Promise(r => setTimeout(r, 1000));
        await takeScreenshot('6_merged_color_test');
        
        // --- Scenario 3: Font Style Inheritance ---
        console.log("--- Scenario 3: Font Style Inheritance ---");
        
        // Set Bold
        const FONT_ROW = START_ROW + 4;
        await page.evaluate(async (row) => {
             const grid = window.grid;
             if (grid) {
                 await grid.updateStyle(5, row, { bold: true, color: "#0000ff" }); // Bold Blue
                 await grid.updateCell(5, row, "BoldText");
             }
        }, FONT_ROW);
        await new Promise(r => setTimeout(r, 500));

        // Select range
        await page.evaluate((row) => {
            const grid = window.grid;
            if (grid) {
                grid.setSelection({
                    current: {
                        cell: [5, row],
                        range: { x: 5, y: row, width: 2, height: 1 },
                        rangeStack: []
                    },
                    columns: { items: [] },
                    rows: { items: [] }
                });
            }
        }, FONT_ROW);
        await new Promise(r => setTimeout(r, 500));
        
        // Merge
        await page.evaluate(async () => {
             const grid = window.grid;
             if (grid) await grid.mergeSelection();
        });
        await new Promise(r => setTimeout(r, 1000));
        await takeScreenshot('7_merged_font_test');


        console.log("Test completed.");

    } catch (error) {
        console.error("Test failed:", error);
    } finally {
        await browser.close();
    }
})();
