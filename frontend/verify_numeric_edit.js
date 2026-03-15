
import puppeteer from 'puppeteer';

(async () => {
  console.log('启动数值类型编辑验证脚本 (Verify Numeric Editing)...');
  
  let browser;
  try {
    browser = await puppeteer.launch({ 
      headless: false,
      defaultViewport: null, 
      args: ['--start-maximized', '--no-sandbox', '--disable-setuid-sandbox'] 
    });
  } catch (e) {
    console.error('无法启动浏览器:', e);
    process.exit(1);
  }

  const page = await browser.newPage();
  
  page.on('console', msg => {
    const type = msg.type();
    const text = msg.text();
    if (!text.includes('HMR') && !text.includes('React Router')) {
        console.log(`[Browser Console] ${type.toUpperCase()}: ${text}`);
    }
  });

  await page.setViewport({ width: 1920, height: 1080 });

  page.on('dialog', async dialog => {
    console.log('[Browser Dialog] ' + dialog.type() + ': ' + dialog.message());
    await dialog.dismiss();
  });

  try {
    // **[2026-02-26]** 变更原因：端口硬编码导致脚本不可移植
    // **[2026-02-26]** 变更目的：统一从环境变量读取端口
    // **[2026-02-26]** 变更原因：保持日志与访问一致
    // **[2026-02-26]** 变更目的：便于排查
    const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
    const BASE_URL = `http://127.0.0.1:${PORT}`;
    console.log(`1. 访问应用 (${BASE_URL})...`);
    await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

    console.log('2. 切换到 Glide 视图...');
    await page.evaluate(() => {
        const btns = Array.from(document.querySelectorAll('button'));
        const glideBtn = btns.find(b => b.innerText.includes('Glide'));
        if (glideBtn) glideBtn.click();
    });
    
    await new Promise(r => setTimeout(r, 1000));

    // Select 'orders' table
    console.log('3. 选择数据表 (orders)...');
    const selectSelector = 'select';
    await page.waitForSelector(selectSelector);
    
    const tableFound = await page.evaluate((sel) => {
        const select = document.querySelector(sel);
        const options = Array.from(select.options);
        // Try 'orders' or 'test_upload_fix'
        let targetIndex = options.findIndex(o => o.text.includes('orders'));
        if (targetIndex === -1) targetIndex = options.findIndex(o => o.text.includes('test_upload_fix'));
        
        if (targetIndex >= 0 && select.selectedIndex !== targetIndex) {
            select.selectedIndex = targetIndex;
            select.dispatchEvent(new Event('change', { bubbles: true }));
            return options[targetIndex].text;
        }
        return select.options[select.selectedIndex].text;
    }, selectSelector);

    console.log(`   当前表: ${tableFound}`);
    await new Promise(r => setTimeout(r, 2000)); 

    // 3.5 Create a Session (Sandbox) because editing requires an active session
    console.log('3.5 创建沙盘 (Create Session)...');
    
    // Find "+ 创建沙盘" button
    const createBtn = await page.evaluateHandle(() => {
        const btns = Array.from(document.querySelectorAll('button'));
        return btns.find(b => b.textContent.includes('创建沙盘'));
    });
    
    if (createBtn) {
        await createBtn.click();
        await new Promise(r => setTimeout(r, 500));
        
        // Find input for name
        await page.type('input[placeholder="新沙盘名称"]', 'NumericTestSession');
        await new Promise(r => setTimeout(r, 200));
        
        // Find "确定" button
        const confirmBtn = await page.evaluateHandle(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            return btns.find(b => b.textContent.includes('确定'));
        });
        
        if (confirmBtn) {
            await confirmBtn.click();
            console.log('   点击确定创建沙盘...');
            // Wait for session creation and reload
            await new Promise(r => setTimeout(r, 2000));
        } else {
             console.log('   Warning: Confirm button not found');
        }
    } else {
        console.log('   Warning: Create Session button not found (maybe already in session?)');
    }

    console.log('4. 执行数值编辑操作 (Row 1, Col "amount")...');
    
    // Adjusted coordinates based on estimation
    // Header ~80px + GridHeader ~30px = ~110px. Row height ~32px.
    // Row 0 center ~ 126px. Let's try y=140 to be safe.
    // Col 2 (region) seems to be around x=400. 
    // Col 3 (amount) should be x=550.
    const x = 550; 
    const y = 140; 
    
    console.log(`   Clicking at (${x}, ${y})...`);
    await page.mouse.click(x, y, { clickCount: 1 }); // Focus first
    await new Promise(r => setTimeout(r, 200));
    await page.mouse.click(x, y, { clickCount: 2 }); // Double click to edit
    await new Promise(r => setTimeout(r, 500));

    // Check if textarea exists
    let isEditing = await page.evaluate(() => {
        const el = document.querySelector('textarea');
        return el !== null && window.getComputedStyle(el).display !== 'none';
    });

    if (!isEditing) {
        console.warn('   Warning: Could not detect edit input. Trying different coordinates...');
        // Try slightly different Y
        await page.mouse.click(x, y + 20, { clickCount: 2 });
        await new Promise(r => setTimeout(r, 500));
        isEditing = await page.evaluate(() => document.querySelector('textarea') !== null);
    }

    if (isEditing) {
        const testValue = "777";
        console.log(`   输入新数值: ${testValue}`);
        
        // Ensure focus
        await page.focus('textarea');
        
        await page.keyboard.down('Control');
        await page.keyboard.press('A');
        await page.keyboard.up('Control');
        await page.keyboard.press('Backspace');
        
        await page.keyboard.type(testValue);
        await new Promise(r => setTimeout(r, 200));
        await page.keyboard.press('Enter');
        
        // Also try to click away to ensure commit (blur)
        // Click on header (x=400, y=50)
        await page.mouse.click(400, 50);
        
        console.log('   已提交编辑，等待保存...');
        await new Promise(r => setTimeout(r, 2000)); 
    } else {
        console.error('   Error: Failed to enter edit mode.');
    }

    console.log('5. 刷新页面验证持久化...');
    await page.reload({ waitUntil: 'networkidle0' });
    await new Promise(r => setTimeout(r, 2000));

    console.log('   重新选择数据表 (orders)...');
    await page.waitForSelector('select');
    await page.evaluate(() => {
        const select = document.querySelector('select');
        const options = Array.from(select.options);
        const targetIndex = options.findIndex(o => o.text.includes('orders'));
        if (targetIndex >= 0) {
            select.selectedIndex = targetIndex;
            select.dispatchEvent(new Event('change', { bubbles: true }));
        }
    });
    await new Promise(r => setTimeout(r, 2000));
    
    // Verify current session
    const currentSessionName = await page.evaluate(() => {
        const selects = document.querySelectorAll('select');
        if (selects.length > 1) {
             return selects[1].options[selects[1].selectedIndex].text;
        }
        return "";
    });
    console.log(`   当前 Session: ${currentSessionName}`);

    console.log('6. 验证数值回显...');
    
    const rowData = await page.evaluate(async () => {
         const table = "orders";
         const selects = document.querySelectorAll('select');
         const sessionId = selects[1]?.value;
         
         if (!sessionId) return null;
         
         const res = await fetch(`/api/grid-data?session_id=${sessionId}&table_name=${table}&page=1&page_size=100`);
         const json = await res.json();
         return json.data;
    });

    if (rowData && rowData[1]) {
        const row = rowData[1];
        console.log('   Row 1 Data:', JSON.stringify(row));
         const found = row.some(cell => String(cell) === "777" || String(cell) === "777.0");
         if (found) {
            console.log('   SUCCESS: Found "777" in the returned grid data!');
        } else {
            console.error('   FAILURE: Did NOT find "777" in Row 1 data.');
            console.error('   Actual data:', JSON.stringify(row));
            process.exit(1);
        }
    } else {
        console.error('   FAILURE: No data returned from grid for verification.');
        process.exit(1);
    }
    
    // 6. Verify Switch to Default (Read-Only)
    console.log('6. 验证切换回默认/只读沙盘...');
    
    // Switch to default session
    await page.evaluate(() => {
        const selects = document.querySelectorAll('select');
        if (selects.length > 1) {
             const sessionSelect = selects[1];
             sessionSelect.selectedIndex = 0; // Index 0 is "(默认/只读)"
             sessionSelect.dispatchEvent(new Event('change', { bubbles: true }));
        }
    });
    
    await new Promise(r => setTimeout(r, 2000));
    
    // Verify data is back to original
    console.log('   检查默认沙盘数据...');
    const defaultRowData = await page.evaluate(async () => {
         const table = "orders";
         const selects = document.querySelectorAll('select');
         const sessionId = selects[1]?.value; // Should be the default one now
         
         // Explicitly pass session_id to ensure we get that session's data
         const url = sessionId 
             ? `/api/grid-data?session_id=${sessionId}&table_name=${table}&page=1&page_size=100`
             : `/api/grid-data?table_name=${table}&page=1&page_size=100`;

         const res = await fetch(url);
         const json = await res.json();
         return json.data;
    });

    if (defaultRowData && defaultRowData[1]) {
        const row = defaultRowData[1];
        const found777 = row.some(cell => String(cell) === "777" || String(cell) === "777.0");
        if (!found777) {
            console.log('   SUCCESS: "777" is NOT present in Default session (Data reverted to original).');
        } else {
            console.error('   FAILURE: Found "777" in Default session! Switch to default failed.');
            process.exit(1);
        }
    } else {
        console.error('   FAILURE: Could not fetch row data for verification.');
        process.exit(1);
    }
    
    console.log('验证完成，数值已正确回显且沙盘切换正常。');
    
    await browser.close();

  } catch (e) {
    console.error('测试失败:', e);
    if (browser) await browser.close();
    process.exit(1);
  }
})();
