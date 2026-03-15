const puppeteer = require('puppeteer');
const path = require('path');
const fs = require('fs');

(async () => {
    const browser = await puppeteer.launch({
        headless: true, // Headless for speed
        defaultViewport: { width: 1280, height: 800 },
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });

    const page = await browser.newPage();
    
    try {
        // **[2026-02-26]** 变更原因：端口硬编码导致脚本不可移植
        // **[2026-02-26]** 变更目的：统一从环境变量读取端口
        // **[2026-02-26]** 变更原因：减少维护成本
        // **[2026-02-26]** 变更目的：保持脚本一致性
        const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
        const BASE_URL = `http://localhost:${PORT}`;
        console.log(`Navigating to app (${BASE_URL})...`);
        await page.goto(BASE_URL, { waitUntil: 'domcontentloaded' });

        // Select table
        console.log("Selecting table 'users'...");
        await page.waitForSelector('select', { timeout: 5000 });
        await page.select('select', 'users');
        
        // Wait for Grid
        await page.waitForFunction(() => window.grid !== undefined);
        console.log("Grid ready.");

        // --- Test 1: Grid Selection -> Formula Bar Sync ---
        console.log("\n--- Test 1: Grid Selection -> Formula Bar Sync ---");
        
        // Programmatically select cell (0, 0)
        await page.evaluate(() => {
            window.grid.setSelection({
                current: {
                    cell: [0, 0],
                    range: { x: 0, y: 0, width: 1, height: 1 },
                    rangeStack: []
                }
            });
        });
        
        // Wait for React to update
        await new Promise(r => setTimeout(r, 500));
        
        // Find input in Formula Bar.
        // It's the input sibling to the 'fx' div.
        const formulaBarValue = await page.evaluate(() => {
            const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
            if (fxDiv && fxDiv.parentElement) {
                const input = fxDiv.parentElement.querySelector('input');
                return input ? input.value : "INPUT_NOT_FOUND";
            }
            return "FX_NOT_FOUND";
        });
        
        console.log(`Formula Bar Value: "${formulaBarValue}"`);
        
        if (formulaBarValue === "FX_NOT_FOUND" || formulaBarValue === "INPUT_NOT_FOUND") {
             throw new Error("Could not find Formula Bar input");
        }
        
        // --- Test 2: Formula Bar Edit -> Grid Sync ---
        console.log("\n--- Test 2: Formula Bar Edit -> Grid Sync ---");
        
        const testValue = "=SUM(1,2)";
        
        // Type into Formula Bar
        await page.evaluate((val) => {
            const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
            if (fxDiv && fxDiv.parentElement) {
                const input = fxDiv.parentElement.querySelector('input');
                if (input) {
                    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
                    nativeInputValueSetter.call(input, val);
                    const ev = new Event('input', { bubbles: true});
                    input.dispatchEvent(ev);
                }
            }
        }, testValue);
        
        await new Promise(r => setTimeout(r, 200));
        await page.keyboard.press('Enter');
        await new Promise(r => setTimeout(r, 1000)); // Wait for commit
        
        // Verify persisted value by re-selecting
        await page.evaluate(() => {
            window.grid.setSelection({
                current: { cell: [0, 1], range: { x: 0, y: 1, width: 1, height: 1 }, rangeStack: [] }
            });
        });
        await new Promise(r => setTimeout(r, 500));
        await page.evaluate(() => {
            window.grid.setSelection({
                current: { cell: [0, 0], range: { x: 0, y: 0, width: 1, height: 1 }, rangeStack: [] }
            });
        });
        await new Promise(r => setTimeout(r, 500));
        
        const finalValue = await page.evaluate(() => {
            const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
            return fxDiv.parentElement.querySelector('input').value;
        });
        
        console.log(`Final Value in Formula Bar: "${finalValue}"`);
        if (finalValue !== testValue) {
             console.error(`FAIL: Expected "${testValue}", got "${finalValue}"`);
             // Don't throw yet, continue to suggestion test
        } else {
             console.log("PASS: Bidirectional sync successful.");
        }

        // --- Test 3: Formula Bar Suggestions ---
        console.log("\n--- Test 3: Formula Bar Suggestions ---");
        
        // 1. Focus input
        await page.evaluate(() => {
            const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
            const input = fxDiv.parentElement.querySelector('input');
            input.focus();
            input.value = ''; 
            const ev = new Event('input', { bubbles: true});
            input.dispatchEvent(ev);
        });
        
        // 2. Type '=' then 'S' using the reliable hack
        await page.evaluate(() => {
            const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
            if (fxDiv && fxDiv.parentElement) {
                const input = fxDiv.parentElement.querySelector('input');
                if (input) {
                    input.focus();
                    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
                    nativeInputValueSetter.call(input, '=S');
                    const ev = new Event('input', { bubbles: true});
                    input.dispatchEvent(ev);
                }
            }
        });
        
        await new Promise(r => setTimeout(r, 1000));
        
        // Listen to console (moved up)
        
        // Debug: Check FormulaEngine
        const functionCount = await page.evaluate(() => {
            if ((window).FormulaEngine) {
                const funcs = (window).FormulaEngine.getInstance().getSupportedFunctions();
                return { count: funcs.length, sample: funcs.slice(0, 5) };
            }
            return "FormulaEngine not found";
        });
        console.log("Supported Functions:", functionCount);
        
        // Debug: Check Input Layout
        const inputLayout = await page.evaluate(() => {
             const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
             const input = fxDiv.parentElement.querySelector('input');
             const rect = input.getBoundingClientRect();
             return { x: rect.x, y: rect.y, width: rect.width, height: rect.height, top: rect.top, bottom: rect.bottom };
        });
        console.log("Input Layout:", inputLayout);
        
        // Debug: Check Input Value
        const inputValue = await page.evaluate(() => {
            const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
            return fxDiv.parentElement.querySelector('input').value;
        });
        console.log(`Input Value after typing: "${inputValue}"`);

        // Check visibility via DOM
        let suggestionsVisible = await page.evaluate(() => {
            const divs = Array.from(document.querySelectorAll('div'));
            return divs.some(d => d.innerText.includes("Suggested Formulas"));
        });
        console.log("Suggestions Visible check:", suggestionsVisible);
        
        if (!suggestionsVisible) {
             console.log("Suggestions not found. Trying FX Popup...");
             // Try clicking FX button
             await page.evaluate(() => {
                 const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
                 if (fxDiv) fxDiv.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
             });
             await new Promise(r => setTimeout(r, 500));
             
             const fxPopupVisible = await page.evaluate(() => {
                 const divs = Array.from(document.querySelectorAll('div'));
                 return divs.some(d => d.innerText.includes("Insert Function"));
             });
             console.log("FX Popup Visible:", fxPopupVisible);
             
             if (fxPopupVisible) {
                 console.log("FX Popup works, but Auto-Suggestions failed.");
                 // Maybe suggestions list was empty?
                 // We saw 'Supported Functions' sample above.
             } else {
                 console.log("Neither Suggestions nor FX Popup worked. Portal or Coords issue.");
             }
             
             await page.screenshot({ path: 'src/scripts/verify_formula_bar_features_missing.png' });
        }
        
        console.log(`Suggestions Visible: ${suggestionsVisible}`);
        
        if (!suggestionsVisible) {
            throw new Error("FAIL: Suggestions dropdown not visible");
        }
        
        // 4. Check content of suggestions (should contain SUM, SEARCH, etc.)
        const suggestionItems = await page.evaluate(() => {
             const header = Array.from(document.querySelectorAll('div')).find(d => d.innerText === "Suggested Formulas");
             if (!header || !header.parentElement) return [];
             // Items are siblings or children of parent?
             // Based on code:
             // <div>
             //    <div header>Suggested Formulas</div>
             //    <div item>...</div>
             // </div>
             // So items are children of the header's parent.
             const parent = header.parentElement;
             return Array.from(parent.children).map(c => c.innerText).filter(t => t !== "Suggested Formulas");
        });
        
        console.log("Suggestion Items:", suggestionItems);
        
        if (!suggestionItems.some(s => s.includes('SUM'))) {
             throw new Error("FAIL: 'SUM' not found in suggestions");
        }
        
        // 5. Select 'SUM' (Down arrow + Enter) or Click
        // Let's use Keyboard ArrowDown then Enter
        await page.keyboard.press('ArrowDown'); // Select first (if index 0 is default, maybe just Enter?)
        // Code sets selectedIndex = 0 by default.
        // Let's press Enter to select the first one (which should be close to S... actually 'SEARCH' comes before 'SUM'? S... SE... SU...)
        // Actually 'S' suggestions: SEARCH, SECOND, SIN, SLN, SLOPE, SMALL, SQRT, STDEV, SUBSTITUTE, SUBTOTAL, SUM...
        // Wait, if it's alphabetical, SUM is further down.
        // Let's type '=SU' to narrow it down.
        
        await page.keyboard.type('U');
        await new Promise(r => setTimeout(r, 500));
        
        const suggestionItems2 = await page.evaluate(() => {
             const header = Array.from(document.querySelectorAll('div')).find(d => d.innerText === "Suggested Formulas");
             if (!header || !header.parentElement) return [];
             const parent = header.parentElement;
             return Array.from(parent.children).map(c => c.innerText).filter(t => t !== "Suggested Formulas");
        });
        console.log("Suggestion Items (after 'SU'):", suggestionItems2);
        
        // Now SUM should be near top.
        // Press Enter to select first one (likely SUBSTITUTE or SUBTOTAL or SUM)
        // Let's check which one is highlighted.
        // The highlighted one has background #e6f7ff
        const selectedSuggestion = await page.evaluate(() => {
             const header = Array.from(document.querySelectorAll('div')).find(d => d.innerText === "Suggested Formulas");
             const parent = header.parentElement;
             const selected = Array.from(parent.children).find(c => getComputedStyle(c).backgroundColor === 'rgb(230, 247, 255)'); // #e6f7ff
             return selected ? selected.innerText : null;
        });
        console.log("Selected Suggestion:", selectedSuggestion);
        
        await page.keyboard.press('Enter');
        await new Promise(r => setTimeout(r, 500));
        
        // 6. Check Input Value
        const valueAfterSelection = await page.evaluate(() => {
             const fxDiv = Array.from(document.querySelectorAll('div')).find(d => d.innerText === 'fx');
             return fxDiv.parentElement.querySelector('input').value;
        });
        
        console.log(`Value after selection: "${valueAfterSelection}"`);
        
        if (!valueAfterSelection.includes('(')) {
             throw new Error("FAIL: Suggestion did not apply correctly (missing '(')");
        }
        
        console.log("PASS: Suggestions functional.");
        
    } catch (e) {
        console.error("Test failed:", e);
        await page.screenshot({ path: 'src/scripts/verify_formula_bar_features_error.png' });
        process.exit(1);
    } finally {
        await browser.close();
    }
})();
