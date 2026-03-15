const puppeteer = require('puppeteer');
const fs = require('fs');
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
    page.on('console', msg => console.log('PAGE LOG:', msg.text()));
    page.on('pageerror', err => console.log('PAGE ERROR:', err.toString()));
    
    // Helper to take screenshots
    const takeScreenshot = async (name) => {
        const screenshotPath = path.join(__dirname, `${name}.png`);
        await page.screenshot({ path: screenshotPath, fullPage: true });
        console.log(`Saved screenshot: ${screenshotPath}`);
    };

    try {
        // **[2026-02-26]** 变更原因：端口硬编码导致脚本不可移植
        // **[2026-02-26]** 变更目的：统一从环境变量读取端口
        // **[2026-02-26]** 变更原因：保留日志一致性
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
            console.log("Select element not found or timeout, assuming Grid might be loaded directly or error.");
        }

        // Wait for grid
        console.log("Waiting for canvas...");
        await page.waitForSelector('canvas', { timeout: 60000 });
        
        // Ensure window.grid is available
        console.log("Waiting for window.grid...");
        await page.waitForFunction(() => window.grid !== undefined, { timeout: 10000 });
        console.log('Grid detected.');

        // Random Start Row to avoid conflicts
        const START_ROW = 2000 + Math.floor(Math.random() * 1000);
        console.log('Test Start Row:', START_ROW);

        // --- T1: Basic Edit & Persistence ---
        console.log('\n--- T1: Basic Edit & Persistence ---');
        // Edit a cell
        await page.evaluate(async (row) => {
            await window.grid.updateCell(0, row, 'TestValue_' + row);
        }, START_ROW);
        await new Promise(r => setTimeout(r, 500)); // Wait for persistence
        await takeScreenshot('T1_basic_edit');

        // --- T2: Formula Dependency ---
        console.log('\n--- T2: Formula Dependency ---');
        // A = 10, B = =A*2
        const ROW_FORMULA = START_ROW + 5;
        await page.evaluate(async (row) => {
            // GlideGrid cols: 0=A, 1=B, 2=C...
            await window.grid.updateCell(1, row, '10'); // Col 1 (B)
            // Formula referencing B{row+1}
            const formula = '=B' + (row + 1) + '*2'; 
            console.log('Setting formula:', formula);
            await window.grid.updateCell(2, row, formula); // Col 2 (C)
        }, ROW_FORMULA);
        await new Promise(r => setTimeout(r, 1000));
        await takeScreenshot('T2_formula');

        // --- T3: Merge & Unmerge ---
        console.log('\n--- T3: Merge & Unmerge ---');
        const ROW_MERGE = START_ROW + 10;
        
        await page.evaluate(async (row) => {
            // Select range: col 1, row -> col 2, row+1
            window.grid.setSelection({
                current: {
                    cell: [1, row],
                    range: { x: 1, y: row, width: 2, height: 2 },
                    rangeStack: []
                },
                columns: { items: [] },
                rows: { items: [] }
            });
        }, ROW_MERGE);
        
        await new Promise(r => setTimeout(r, 500));
        
        // Click Merge button via DOM
        const mergeClicked = await page.evaluate(() => {
            const buttons = Array.from(document.querySelectorAll('button'));
            const btn = buttons.find(b => b.innerText.includes('Merge'));
            if (btn) {
                btn.click();
                return true;
            }
            return false;
        });

        if (mergeClicked) {
            console.log('Clicked Merge button');
        } else {
             // Fallback to API if button missing
             console.log('Merge button not found, trying API...');
             await page.evaluate(async () => {
                 if (window.grid.mergeSelection) await window.grid.mergeSelection();
             });
        }
        
        await new Promise(r => setTimeout(r, 1000));
        await takeScreenshot('T3_merge_created');

        // Test Empty Cell Merge (T3.b)
        const ROW_MERGE_EMPTY = START_ROW + 15;
        await page.evaluate(async (row) => {
             window.grid.setSelection({
                current: {
                    cell: [1, row],
                    range: { x: 1, y: row, width: 3, height: 3 },
                    rangeStack: []
                },
                columns: { items: [] },
                rows: { items: [] }
            });
        }, ROW_MERGE_EMPTY);
        await new Promise(r => setTimeout(r, 500));
        
        const mergeEmptyClicked = await page.evaluate(() => {
            const buttons = Array.from(document.querySelectorAll('button'));
            const btn = buttons.find(b => b.innerText.includes('Merge'));
            if (btn) {
                btn.click();
                return true;
            }
            return false;
        });

        if (mergeEmptyClicked) {
             console.log('Clicked Merge button for empty cells');
        } else {
             await page.evaluate(async () => {
                 if (window.grid.mergeSelection) await window.grid.mergeSelection();
             });
        }
        await new Promise(r => setTimeout(r, 1000));
        
        // Edit the merged empty cell
        await page.evaluate(async (row) => {
            await window.grid.updateCell(1, row, 'MergedEmptyEdited');
        }, ROW_MERGE_EMPTY);
        await new Promise(r => setTimeout(r, 500));
        await takeScreenshot('T3_merge_empty_edited');


        // --- T4: Styles ---
        console.log('\n--- T4: Styles ---');
        const ROW_STYLE = START_ROW + 20;
        await page.evaluate(async (row) => {
             window.grid.setSelection({
                current: {
                    cell: [1, row],
                    range: { x: 1, y: row, width: 1, height: 1 },
                    rangeStack: []
                },
                columns: { items: [] },
                rows: { items: [] }
            });
        }, ROW_STYLE);
        
        // Click Bold - assuming 'B' or 'Bold'
        const boldClicked = await page.evaluate(() => {
            const buttons = Array.from(document.querySelectorAll('button'));
            // Look for button with 'B' or bold style
            const btn = buttons.find(b => b.innerText === 'B' || b.innerText === 'Bold' || b.style.fontWeight === 'bold');
            if (btn) {
                btn.click();
                return true;
            }
            return false;
        });

        if (boldClicked) {
             console.log('Clicked Bold button');
        } else {
            console.log('Bold button not found, trying style update manually');
            await page.evaluate(async (row) => {
                 // Simulate style update via API if possible
                 if (window.grid.updateStyle) {
                     await window.grid.updateStyle(1, row, { bold: true });
                 }
            }, ROW_STYLE);
        }
        
        await page.evaluate(async (row) => {
            await window.grid.updateCell(1, row, 'StyledText');
        }, ROW_STYLE);
        await new Promise(r => setTimeout(r, 500));
        await takeScreenshot('T4_styles');

        console.log('Tests completed successfully.');

    } catch (e) {
        console.error('Test failed:', e);
        await takeScreenshot('error_failure');
        process.exit(1);
    } finally {
        await browser.close();
    }
})();
