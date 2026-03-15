import puppeteer from 'puppeteer';

(async () => {
    console.log('Launching browser...');
    const browser = await puppeteer.launch({
        headless: false,
        args: ['--no-sandbox', '--disable-setuid-sandbox'],
        defaultViewport: { width: 1280, height: 800 }
    });
    const page = await browser.newPage();
    
    const PORT = 5174;
    const BASE_URL = `http://127.0.0.1:${PORT}`;
    
    // Capture logs
    page.on('console', msg => {
        const text = msg.text();
        if (text.toLowerCase().includes('shim') || 
            text.toLowerCase().includes('error') || 
            text.toLowerCase().includes('fail') ||
            text.includes('GlideGrid')) {
            console.log(`[PAGE LOG] ${msg.type()}: ${text}`);
        }
    });

    page.on('pageerror', err => {
        console.error(`[PAGE ERROR] ${err.toString()}`);
    });

    try {
        console.log(`Navigating to ${BASE_URL}...`);
        await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

        // Select Table
        console.log('Selecting table "users"...');
        try {
            await page.waitForSelector('select', { timeout: 5000 });
            await page.select('select', 'users');
        } catch (e) {
            console.log('Select not found, maybe already selected or loaded.');
        }
        
        // Wait for Grid
        console.log('Waiting for Grid...');
        try {
            await page.waitForSelector('.gdg-wrapper canvas', { timeout: 10000 });
            console.log('Grid canvas found.');
        } catch (e) {
            console.error('Grid canvas NOT found!');
        }
        
        // Wait for data load
        await new Promise(r => setTimeout(r, 3000));
        
        // Test Freeze
        console.log('Testing Freeze button...');
        try {
            const freezeBtn = await page.waitForSelector('button[title="Freeze Panes"]', { timeout: 3000 });
            if (freezeBtn) {
                await freezeBtn.click();
                console.log('Clicked Freeze.');
                await new Promise(r => setTimeout(r, 1000));
            }
        } catch (e) {
            console.error('Freeze button not found!');
        }
        
        // Test Edit (Mock)
        console.log('Testing Edit (Mock)...');
        // Click on a cell in the grid (e.g. 100, 200 coordinates)
        await page.mouse.click(200, 200);
        await new Promise(r => setTimeout(r, 500));
        await page.keyboard.type('TestEdit');
        await page.keyboard.press('Enter');
        console.log('Typed "TestEdit" and pressed Enter.');
        
        // Wait to see if any error alert pops up
        await new Promise(r => setTimeout(r, 2000));
        
        // Test Filter (Shim)
        console.log('Testing Filter button...');
        try {
            const filterBtn = await page.waitForSelector('button[title="Toggle Filter"]', { timeout: 3000 });
            if (filterBtn) {
                await filterBtn.click();
                console.log('Clicked Filter Toggle.');
                await new Promise(r => setTimeout(r, 1000));
                
                // Click a header to open filter menu (assuming headers are at top)
                // Coordinates approx (100, 40)
                await page.mouse.click(100, 40);
                console.log('Clicked Header (100, 40).');
                
                // Wait for filter menu
                await new Promise(r => setTimeout(r, 2000));
            }
        } catch (e) {
            console.error('Filter button not found!');
        }

        console.log('Verification Complete.');
    } catch (e) {
        console.error('Test Failed:', e);
    } finally {
        await browser.close();
    }
})();
