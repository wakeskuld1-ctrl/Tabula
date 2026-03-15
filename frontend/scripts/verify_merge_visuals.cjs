const puppeteer = require('puppeteer');
const fs = require('fs');
const path = require('path');

(async () => {
  const browser = await puppeteer.launch({
    headless: "new",
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--window-size=1280,800'],
    defaultViewport: { width: 1280, height: 800 }
  });
  const page = await browser.newPage();
  
  // Capture console logs
  page.on('console', msg => console.log('PAGE LOG:', msg.text()));
  page.on('pageerror', err => console.error('PAGE ERROR:', err.toString()));

  try {
    // **[2026-02-26]** 变更原因：端口硬编码导致脚本不可移植
    // **[2026-02-26]** 变更目的：统一从环境变量读取端口
    // **[2026-02-26]** 变更原因：保持日志与访问一致
    // **[2026-02-26]** 变更目的：便于排查
    const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
    const BASE_URL = `http://localhost:${PORT}`;
    console.log(`Navigating to ${BASE_URL}...`);
    await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

    console.log('Waiting for grid exposure...');
    try {
        await page.waitForFunction(() => !!window.grid, { timeout: 5000 });
    } catch (e) {
        console.log("window.grid not found immediately. Trying to select a table first.");
    }

    // Attempt to click the first table in the sidebar
    // Dump page text content
    const bodyText = await page.evaluate(() => document.body.innerText);
    console.log("Page Text Content:", bodyText);

    console.log("Looking for tables...");
     await page.waitForFunction(() => !!window.app);
     console.log("Selecting table 'test_upload_fix' via window.app...");
     
     await page.evaluate(() => {
         window.app.selectTable('test_upload_fix');
     });
     
     const tableClicked = true;

/*
     const tableClicked = await page.evaluate(() => {
         // Find any element with text 'test_upload_fix' or any table like text
         const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null, false);
         let node;
         while(node = walker.nextNode()) {
             if (node.textContent.includes('test_upload_fix') || node.textContent.includes('customer') || node.textContent.includes('PingCode')) {
                  console.log("Found table text:", node.textContent);
                  // Click the parent div
                  let parent = node.parentElement;
                  while(parent && parent.tagName !== 'DIV') {
                      parent = parent.parentElement;
                  }
                  if (parent) {
                      console.log("Clicking parent div");
                      parent.click();
                      return true;
                  }
             }
         }
         
         // Fallback to cursor-pointer check
         const sidebar = document.querySelector('.w-64');
         if (!sidebar) {
             console.log("Sidebar .w-64 not found");
             return false;
         }
         
         const items = Array.from(sidebar.querySelectorAll('div'));
         for (const item of items) {
             if (item.classList.contains('cursor-pointer')) {
                  console.log("Clicking cursor-pointer div:", item.textContent);
                  item.click();
                  return true;
             }
         }
         
         return false;
     });
*/
   if (tableClicked) {
      console.log("Table clicked. Waiting for load...");
      await new Promise(r => setTimeout(r, 2000));
      
      console.log('Waiting for grid exposure (retry)...');
      try {
          await page.waitForFunction(() => {
              if (document.body.innerText.includes("Loading...")) return false;
              return !!window.grid;
          }, { timeout: 15000 });
      } catch (e) {
          console.log("Timeout waiting for window.grid");
          const text = await page.evaluate(() => document.body.innerText);
          console.log("Current page text:", text.substring(0, 500));
          throw e;
      }
  } else {
      console.log("No table found/clicked. Grid might be empty.");
  }

  const scriptDir = __dirname;
  await page.screenshot({ path: path.join(scriptDir, 'step1_initial.png') });

  console.log('Setting selection via window.grid...');
   await page.evaluate(() => {
       if (!window.grid) throw new Error("window.grid is missing");
       if (!window.CompactSelection) throw new Error("window.CompactSelection is missing");
       
       const range = { x: 1, y: 1, width: 2, height: 2 };
       const sel = {
           current: {
               cell: [1, 1],
               range: range,
               rangeStack: []
           },
           columns: window.CompactSelection.empty(),
           rows: window.CompactSelection.empty()
       };
       
       window.grid.setSelection(sel);
   });
  
  await new Promise(r => setTimeout(r, 500));
  await page.screenshot({ path: path.join(scriptDir, 'step2_selected.png') });
  
  console.log('Merging selection...');
  const result = await page.evaluate(async () => {
      await window.grid.mergeSelection();
      return "Merge triggered";
  });
  console.log(result);
  
  await new Promise(r => setTimeout(r, 1000)); // Wait for refresh
  await page.screenshot({ path: path.join(scriptDir, 'step3_merged.png') });
    
    console.log('Test completed. Check screenshots in frontend/scripts/');
    
  } catch (e) {
    console.error('Test Error:', e);
  } finally {
    await browser.close();
  }
})();
