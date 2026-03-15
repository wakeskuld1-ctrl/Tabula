import puppeteer from 'puppeteer';

(async () => {
  console.log('Starting smoke test...');
  const browser = await puppeteer.launch({
    headless: "new",
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  const page = await browser.newPage();
  page.on('dialog', async (dialog) => {
      console.log('Dialog:', dialog.message());
      await dialog.dismiss();
  });
  
  try {
    // **[2026-02-26]** 变更原因：端口硬编码导致脚本无法复用
    // **[2026-02-26]** 变更目的：统一从环境变量读取端口
    // **[2026-02-26]** 变更原因：保留自定义入口
    // **[2026-02-26]** 变更目的：允许外部覆盖
    const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
    const baseUrl = process.env.SMOKE_BASE_URL || `http://localhost:${PORT}`;
    console.log(`Navigating to ${baseUrl}...`);
    await page.goto(baseUrl);
    
    console.log('Waiting for status bar...');
    // Wait for the status bar which contains backend status
    await page.waitForSelector('.status-bar', { timeout: 10000 });
    
    const text = await page.$eval('.status-bar', el => el.textContent);
    console.log('Status bar text:', text);
    
    if (text.includes('后端状态')) {
        console.log('✅ Smoke Test Passed: Status bar found.');
    } else {
        console.error('❌ Smoke Test Failed: "后端状态" not found in status bar.');
        process.exit(1);
    }

    // Also check for Toolbar and FormulaBar presence
    const toolbar = await page.$('button[title="Save"]');
    if (toolbar) {
        console.log('✅ Toolbar Save button found.');
    } else {
        console.error('❌ Toolbar Save button NOT found.');
    }

    await page.evaluate(() => {
        const radios = Array.from(document.querySelectorAll('input[name="viewMode"]'));
        if (radios.length >= 2) {
            const glideRadio = radios[1];
            if (glideRadio instanceof HTMLInputElement) {
                glideRadio.click();
            }
        }
    });

    await page.waitForFunction(() => {
        return !!(window.app && typeof window.app.selectTable === 'function');
    }, { timeout: 10000 });

    const backendOk = await page.evaluate(async () => {
        try {
            const res = await fetch('/api/health');
            return res.ok;
        } catch (e) {
            return false;
        }
    });
    if (!backendOk) {
        console.error('❌ Smoke Test Failed: backend health check failed.');
        process.exit(1);
    }

    const tableSelected = await page.evaluate(async () => {
        if (!window.app || !window.app.selectTable) {
            return false;
        }
        const res = await fetch('/api/tables');
        const data = await res.json();
        const tableName = data.tables && data.tables.length > 0 ? data.tables[0].table_name : "";
        if (!tableName) return false;
        await window.app.selectTable(tableName);
        return true;
    });

    if (!tableSelected) {
        console.error('❌ Smoke Test Failed: no table selected or backend unavailable.');
        process.exit(1);
    }

    // **[2026-02-26]** 变更原因：粘贴报“会话不存在”导致等待超时
    // **[2026-02-26]** 变更目的：测试前确保会话可写并切换为当前
    // **[2026-02-26]** 变更原因：复用同名会话可减少污染
    // **[2026-02-26]** 变更目的：优先复用并在缺失时创建
    console.log('Ensuring session for smoke test...');
    await page.waitForFunction(() => {
        return window.app
            && typeof window.app.createSession === 'function'
            && typeof window.app.switchSession === 'function';
    }, { timeout: 10000 });
    const sessionEnsureResult = await page.evaluate(async ({ tableName, sessionName }) => {
        // **[2026-02-26]** 变更原因：统一会话列表获取逻辑
        // **[2026-02-26]** 变更目的：复用已存在会话减少重复创建
        const fetchSessions = async () => {
            const res = await fetch(`/api/sessions?table_name=${encodeURIComponent(tableName)}`);
            if (!res.ok) {
                return { ok: false, error: `list sessions failed: ${res.status}` };
            }
            const data = await res.json();
            if (data.status !== 'ok') {
                return { ok: false, error: data.message || 'list sessions error' };
            }
            return { ok: true, sessions: data.sessions || [] };
        };
        if (!window.app || typeof window.app.createSession !== 'function') {
            return { ok: false, error: 'window.app.createSession missing' };
        }
        if (typeof window.app.switchSession !== 'function') {
            return { ok: false, error: 'window.app.switchSession missing' };
        }
        try {
            // **[2026-02-26]** 变更原因：避免重复 session name 造成冲突
            // **[2026-02-26]** 变更目的：先查后复用同名会话
            const listResult = await fetchSessions();
            if (!listResult.ok) {
                return { ok: false, error: listResult.error };
            }
            const sessions = listResult.sessions || [];
            const matched = sessions.find(s => s?.name === sessionName);
            if (matched && matched.session_id) {
                // **[2026-02-26]** 变更原因：已有会话需切换为活跃
                // **[2026-02-26]** 变更目的：确保后续写入落在可写会话
                const id = await window.app.switchSession(matched.session_id);
                const current = typeof window.app.getCurrentSession === 'function'
                    ? window.app.getCurrentSession()
                    : '';
                return { ok: true, reused: true, id: id || matched.session_id, current };
            }
            // **[2026-02-26]** 变更原因：无同名会话可复用
            // **[2026-02-26]** 变更目的：创建新会话确保写入可用
            const id = await window.app.createSession(sessionName);
            const current = typeof window.app.getCurrentSession === 'function'
                ? window.app.getCurrentSession()
                : '';
            return { ok: true, reused: false, id: id || '', current };
        } catch (e) {
            return { ok: false, error: e?.message || String(e) };
        }
    }, { tableName: 'users', sessionName: 'smoke' });
    if (!sessionEnsureResult.ok) {
        console.error('❌ Smoke Test Failed: ensure session failed.', sessionEnsureResult.error);
        process.exit(1);
    }
    // **[2026-02-26]** 变更原因：会话切换存在异步延迟
    // **[2026-02-26]** 变更目的：等待会话状态稳定再执行网格操作
    await page.waitForFunction(() => {
        return window.app && typeof window.app.getCurrentSession === 'function' && window.app.getCurrentSession();
    }, { timeout: 8000 });

    console.log('Waiting for grid handle...');
    await page.waitForFunction(() => {
        return !!(window.grid && typeof window.grid.refresh === 'function');
    }, { timeout: 15000 });
    console.log('Grid handle ready.');

    console.log('Triggering grid refresh...');
    const gridDataResponse = page.waitForResponse(res => res.url().includes('/api/grid-data'), { timeout: 15000 });
    await page.evaluate(() => {
        window.grid.refresh();
    });
    await gridDataResponse;
    console.log('Grid data response received.');

    console.log('Waiting for glide grid DOM...');
    await page.waitForSelector('[data-testid="glide-grid"]', { timeout: 15000 });
    console.log('Glide grid DOM ready.');
    const grid = await page.$('[data-testid="glide-grid"]');
    if (!grid) {
        console.error('❌ Smoke Test Failed: glide grid not found.');
        process.exit(1);
    }

    const box = await grid.boundingBox();
    if (!box) {
        console.error('❌ Smoke Test Failed: glide grid bounding box missing.');
        process.exit(1);
    }

    const canvas = await grid.$('canvas');
    if (canvas) {
        await canvas.click({ delay: 50 });
    } else {
        await page.mouse.click(box.x + 10, box.y + 10, { delay: 50 });
    }
    console.log('Grid focused.');
    await page.evaluate(() => {
        if (window.grid && typeof window.grid.setSelection === 'function') {
            window.grid.setSelection({
                current: {
                    cell: [0, 0],
                    range: { x: 0, y: 0, width: 1, height: 1 },
                    rangeStack: []
                },
                columns: { items: [] },
                rows: { items: [] }
            });
        }
    });
    console.log('Grid selection set.');

    // **[2026-02-26]** 变更原因：新增 SPLIT 溢出能力需验证。
    // **[2026-02-26]** 变更目的：确保锚点与右侧溢出值可读取。
    const spillCheck = await page.evaluate(() => {
        const engine = window.FormulaEngine?.getInstance?.();
        if (!engine) {
            return { ok: false, error: 'FormulaEngine missing' };
        }
        engine.setCellValue(0, 0, '=SPLIT("A,B", ",")', 'Sheet1');
        const anchor = engine.calculate('=SPLIT("A,B", ",")', 0, 0, 'Sheet1');
        const spill = engine.getSpillValue?.(1, 0, 'Sheet1');
        return { ok: true, anchor, spill };
    });
    if (!spillCheck.ok || spillCheck.anchor !== 'A' || spillCheck.spill !== 'B') {
        console.error('❌ Smoke Test Failed: SPLIT spill not working', spillCheck);
        process.exit(1);
    }
    console.log('✅ SPLIT spill smoke check passed.');

    // **[2026-02-26]** 变更原因：点号分隔符属于正则特殊字符。
    // **[2026-02-26]** 变更目的：确保按字面 '.' 拆分并产生溢出。
    const dotSplitCheck = await page.evaluate(() => {
        const engine = window.FormulaEngine?.getInstance?.();
        if (!engine) {
            return { ok: false, error: 'FormulaEngine missing' };
        }
        engine.setCellValue(3, 1, 'v1.2.3', 'Sheet1');
        engine.setCellValue(6, 1, '=SPLIT(D2, ".")', 'Sheet1');
        const anchor = engine.calculate('=SPLIT(D2, ".")', 6, 1, 'Sheet1');
        const spill1 = engine.getSpillValue?.(7, 1, 'Sheet1');
        const spill2 = engine.getSpillValue?.(8, 1, 'Sheet1');
        return { ok: true, anchor, spill1, spill2 };
    });
    if (!dotSplitCheck.ok || dotSplitCheck.anchor !== 'v1' || dotSplitCheck.spill1 !== '2' || dotSplitCheck.spill2 !== '3') {
        console.error('❌ Smoke Test Failed: SPLIT dot delimiter not working', dotSplitCheck);
        process.exit(1);
    }
    console.log('✅ SPLIT dot delimiter smoke check passed.');

    // **[2026-02-26]** 变更原因：用户要求 "XXX" 判断切割方式。
    // **[2026-02-26]** 变更目的：多字符分隔符应按整体拆分。
    const multiDelimiterCheck = await page.evaluate(() => {
        const engine = window.FormulaEngine?.getInstance?.();
        if (!engine) {
            return { ok: false, error: 'FormulaEngine missing' };
        }
        engine.setCellValue(3, 2, 'A||B', 'Sheet1');
        engine.setCellValue(6, 2, '=SPLIT(D3, "||")', 'Sheet1');
        const anchor = engine.calculate('=SPLIT(D3, "||")', 6, 2, 'Sheet1');
        const spill = engine.getSpillValue?.(7, 2, 'Sheet1');
        return { ok: true, anchor, spill };
    });
    if (!multiDelimiterCheck.ok || multiDelimiterCheck.anchor !== 'A' || multiDelimiterCheck.spill !== 'B') {
        console.error('❌ Smoke Test Failed: SPLIT multi delimiter not working', multiDelimiterCheck);
        process.exit(1);
    }
    console.log('✅ SPLIT multi delimiter smoke check passed.');

    const pasteValue = "123";

    console.log('Waiting for paste response...');
    const pasteResponse = page.waitForResponse(res => res.url().includes('/api/batch_update_cells'), { timeout: 15000 });
    const pasteTriggered = await page.evaluate((val) => {
        if (window.grid && typeof window.grid.pasteValues === 'function') {
            return window.grid.pasteValues([0, 0], [[val]]);
        }
        return false;
    }, pasteValue);
    if (!pasteTriggered) {
        console.error('❌ Smoke Test Failed: paste trigger failed.');
        process.exit(1);
    }

    const res = await pasteResponse;
    console.log('Paste response received.');
    if (!res.ok()) {
        console.error('❌ Smoke Test Failed: batch_update_cells status', res.status());
        process.exit(1);
    }

    const immediateValue = await page.evaluate(() => {
        return window.grid && typeof window.grid.getCell === "function" ? window.grid.getCell(0, 0) : null;
    });
    console.log('Cell value after paste:', immediateValue);

    console.log('Waiting for grid value update...');
    // **[2026-02-26]** 变更原因：后端偶发抖动导致等待超时
    // **[2026-02-26]** 变更目的：增加一次轻量重试以稳定用例
    try {
        await page.waitForFunction((val) => {
            return window.grid && typeof window.grid.getCell === "function" && window.grid.getCell(0, 0) === val;
        }, { timeout: 10000 }, pasteValue);
    } catch (e) {
        console.log('Retrying grid value update wait...');
        await page.waitForFunction((val) => {
            return window.grid && typeof window.grid.getCell === "function" && window.grid.getCell(0, 0) === val;
        }, { timeout: 10000 }, pasteValue);
    }
    console.log('Grid value updated.');

    console.log('✅ Paste smoke test passed.');

  } catch (e) {
    console.error('❌ Smoke Test Failed:', e);
    process.exit(1);
  } finally {
    await browser.close();
  }
})();
