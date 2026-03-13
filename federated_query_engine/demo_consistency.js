const puppeteer = require('puppeteer');
const path = require('path');

(async () => {
    const browser = await puppeteer.launch({ 
        headless: "new",
        args: ['--no-sandbox', '--disable-setuid-sandbox'],
        defaultViewport: { width: 1280, height: 800 }
    });
    const page = await browser.newPage();
    
    // Capture logs
    page.on('console', msg => console.log('PAGE LOG:', msg.text()));

    try {
        console.log("Navigating to frontend...");
        await page.goto('http://localhost:3000', { waitUntil: 'networkidle0' });

        // 1. Upload yashan_mock.csv
        console.log("Uploading yashan_mock.csv...");
        const inputUploadHandle = await page.$('input[type=file]');
        const mockFilePath = path.resolve(__dirname, 'yashan_mock.csv');
        await inputUploadHandle.uploadFile(mockFilePath);
        
        // Wait for upload processing (frontend should register it)
        await new Promise(r => setTimeout(r, 2000));
        
        // Check if table appeared in tree
        const treeText = await page.evaluate(() => document.getElementById('tree-root').innerText);
        if (treeText.includes('yashan_mock')) {
            console.log("SUCCESS: yashan_mock table registered.");
        } else {
            console.warn("WARNING: yashan_mock not found in tree immediately.");
        }

        // DEBUG: Check yashan_mock schema/data
        console.log("DEBUG: Querying yashan_mock...");
        await page.evaluate(() => {
            if (window.editor) window.editor.setValue("SELECT * FROM yashan_mock");
        });
        await page.click('button[onclick="executeQuery()"]');
        await new Promise(r => setTimeout(r, 2000));
        let debugRows = await page.evaluate(() => document.getElementById('rows-val').innerText);
        console.log(`DEBUG: yashan_mock rows: ${debugRows}`);


        // 2. Run Consistency Check (Identical)
        console.log("Running SQL: Oracle vs YashanMock (Expect Identical)...");
        // Force CAST to ensure type matching with Oracle (assuming Oracle is VARCHAR/TEXT)
        const sqlIdentical = "SELECT cfg_name, CAST(cfg_value AS VARCHAR) as cfg_value FROM BMSQL_CONFIG EXCEPT SELECT cfg_name, CAST(cfg_value AS VARCHAR) as cfg_value FROM yashan_mock";
        
        // Set Editor Content and Run
        await page.evaluate(async (sql) => {
            if (window.editor) {
                window.editor.setValue(sql);
            }
            // Clear previous result indicators
            document.getElementById('rows-val').innerText = "-1";
            document.getElementById('upload-status').innerText = "";
            
            // Execute directly to ensure ordering
            await window.executeQuery();
        }, sqlIdentical);

        // Wait for result
        await new Promise(r => setTimeout(r, 2000));
        
        // Check row count and status
        let rowCount = await page.evaluate(() => document.getElementById('rows-val').innerText);
        let statusMsg = await page.evaluate(() => document.getElementById('upload-status').innerText);
        console.log(`Identical Check Result Rows: ${rowCount}, Status: ${statusMsg}`);
        
        if (rowCount === "0") {
            console.log("VERIFIED: Tables are identical (0 diff rows).");
        } else {
            console.log("FAILED: Tables are NOT identical.");
        }

        // 3. Upload yashan_diff.csv
        console.log("Uploading yashan_diff.csv...");
        const diffFilePath = path.resolve(__dirname, 'yashan_diff.csv');
        await inputUploadHandle.uploadFile(diffFilePath);
        await new Promise(r => setTimeout(r, 2000));

        // 4. Run Consistency Check (Different)
        console.log("Running SQL: Oracle vs YashanDiff (Expect Differences)...");
        const sqlDiff = "SELECT cfg_name, CAST(cfg_value AS VARCHAR) as cfg_value FROM BMSQL_CONFIG EXCEPT SELECT cfg_name, CAST(cfg_value AS VARCHAR) as cfg_value FROM yashan_diff";
        
        await page.evaluate(async (sql) => {
            if (window.editor) {
                window.editor.setValue(sql);
            }
            await window.executeQuery();
        }, sqlDiff);

        await new Promise(r => setTimeout(r, 2000));
        
        rowCount = await page.evaluate(() => document.getElementById('rows-val').innerText);
        console.log(`Diff Check Result Rows: ${rowCount}`);
        
        if (rowCount !== "0") {
            console.log("VERIFIED: Tables are different (Found diff rows).");
        } else {
            console.log("FAILED: Expected differences but found none.");
        }

    } catch (e) {
        console.error("Test failed:", e);
    } finally {
        await browser.close();
    }
})();
