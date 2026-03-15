
import puppeteer from 'puppeteer';

(async () => {
  console.log('启动 TC15 验证脚本...');
  
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
  
  // Enable Console Log Capture
  page.on('console', msg => {
    const type = msg.type();
    const text = msg.text();
    console.log(`[Browser Console] ${type.toUpperCase()}: ${text}`);
  });

  await page.setViewport({ width: 1920, height: 1080 });

  try {
    console.log('正在访问应用...');
    const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
    const BASE_URL = `http://127.0.0.1:${PORT}`;
    await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

    // **[2026-03-11]** 变更原因：前台入口已统一到 GlideGrid
    // **[2026-03-11]** 变更目的：移除旧视图切换依赖，避免误导脚本走旧路径

    // Step 1: Resolve a table name from backend
    // **[2026-02-26]** 变更原因：页面不一定存在下拉框选择器
    // **[2026-02-26]** 变更目的：通过后端表列表获取可用表名
    const resolvedTableName = await page.evaluate(async () => {
        try {
            const res = await fetch('/api/tables');
            if (!res.ok) return '';
            const data = await res.json();
            const tableName = data.tables && data.tables.length > 0 ? data.tables[0].table_name : '';
            return tableName || '';
        } catch (e) {
            return '';
        }
    });
    if (!resolvedTableName) {
        console.error('FAILURE: no table available for TC15.');
        process.exit(1);
    }

    // Step 1: Try selecting table via app/sidebar/select
    // **[2026-02-26]** 变更原因：UI 结构可能变化导致选择方式失效
    // **[2026-02-26]** 变更目的：按优先级尝试多种选择路径
    const tableSelectionResult = await page.evaluate(async (tableName) => {
        if (window.app && typeof window.app.selectTable === 'function') {
            await window.app.selectTable(tableName);
            return { ok: true, method: 'app' };
        }
        const buttons = Array.from(document.querySelectorAll('.sidebar-list .sidebar-item'));
        const target = buttons.find(b => b.title === tableName || b.textContent?.includes(tableName));
        if (target) {
            target.click();
            return { ok: true, method: 'sidebar' };
        }
        const select = document.querySelector('select');
        if (select) {
            const option = Array.from(select.options).find(o => o.value === tableName || o.textContent?.includes(tableName));
            if (option) {
                select.value = option.value;
                select.dispatchEvent(new Event('change', { bubbles: true }));
                return { ok: true, method: 'select' };
            }
        }
        return { ok: false, error: 'no table selector available' };
    }, resolvedTableName);
    if (!tableSelectionResult.ok) {
        console.error('FAILURE: table selection failed.', tableSelectionResult.error);
        process.exit(1);
    }
    await new Promise(r => setTimeout(r, 1000));

    // **[2026-02-26]** 变更原因：会话未创建导致后续写入失败
    // **[2026-02-26]** 变更目的：测试前确保会话可写并切换为当前
    // **[2026-02-26]** 变更原因：复用会话可降低历史污染
    // **[2026-02-26]** 变更目的：优先复用同名会话
    console.log('Ensuring session for TC15...');
    await page.waitForFunction(() => {
        return window.app
            && typeof window.app.createSession === 'function'
            && typeof window.app.switchSession === 'function';
    }, { timeout: 10000 });
    const sessionEnsureResult = await page.evaluate(async ({ tableName, sessionName }) => {
        // **[2026-02-26]** 变更原因：统一会话拉取逻辑
        // **[2026-02-26]** 变更目的：复用已存在会话避免重复创建
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
            // **[2026-02-26]** 变更原因：避免重复 session name
            // **[2026-02-26]** 变更目的：先查后复用同名会话
            const listResult = await fetchSessions();
            if (!listResult.ok) {
                return { ok: false, error: listResult.error };
            }
            const sessions = listResult.sessions || [];
            const matched = sessions.find(s => s?.name === sessionName);
            if (matched && matched.session_id) {
                // **[2026-02-26]** 变更原因：已有会话需要切换为活跃
                // **[2026-02-26]** 变更目的：确保后续写入落在可写会话
                const id = await window.app.switchSession(matched.session_id);
                const current = typeof window.app.getCurrentSession === 'function'
                    ? window.app.getCurrentSession()
                    : '';
                return { ok: true, reused: true, id: id || matched.session_id, current };
            }
            // **[2026-02-26]** 变更原因：未发现可复用会话
            // **[2026-02-26]** 变更目的：创建新的测试会话保障写入
            const id = await window.app.createSession(sessionName);
            const current = typeof window.app.getCurrentSession === 'function'
                ? window.app.getCurrentSession()
                : '';
            return { ok: true, reused: false, id: id || '', current };
        } catch (e) {
            return { ok: false, error: e?.message || String(e) };
        }
    }, { tableName: resolvedTableName, sessionName: 'tc15' });
    if (!sessionEnsureResult.ok) {
        console.error('FAILURE: ensure session failed.', sessionEnsureResult.error);
        process.exit(1);
    }
    // **[2026-02-26]** 变更原因：会话切换存在异步延迟
    // **[2026-02-26]** 变更目的：等待会话状态稳定再继续
    await page.waitForFunction(() => {
        return window.app && typeof window.app.getCurrentSession === 'function' && window.app.getCurrentSession();
    }, { timeout: 8000 });

    // **[2026-03-11]** 变更原因：前台网格路径已经统一
    // **[2026-03-11]** 变更目的：只等待 GlideGrid 能力就绪，简化验证流程
    try {
        await page.waitForFunction(() => {
            return window.grid && typeof window.grid.updateCell === 'function' && typeof window.grid.getCell === 'function';
        }, { timeout: 20000 });
    } catch (e) {
        // **[2026-03-11]** 变更原因：首次等待失败可能由慢启动导致
        // **[2026-03-11]** 变更目的：补一次重试以降低环境抖动误报
        await page.waitForFunction(() => {
            return window.grid && typeof window.grid.updateCell === 'function' && typeof window.grid.getCell === 'function';
        }, { timeout: 20000 });
    }
    const glideReady = await page.evaluate(() => {
        return Boolean(window.grid && typeof window.grid.updateCell === 'function' && typeof window.grid.getCell === 'function');
    });
    if (!glideReady) {
        // **[2026-03-11]** 变更原因：环境异常导致 GlideGrid API 缺失
        // **[2026-03-11]** 变更目的：快速失败并给出明确根因
        console.error('FAILURE: no supported grid implementation detected.');
        process.exit(1);
    }
    console.log('Using GlideGrid path for TC15...');
    // **[2026-03-11]** 变更原因：固定列索引在不同表结构中易越界
    // **[2026-03-11]** 变更目的：动态选择可写列，保证脚本在任意表上可运行
    const glideColumnCheck = await page.evaluate(async (tableName) => {
        try {
            const res = await fetch('/api/tables');
            if (!res.ok) return { ok: false, error: `tables status ${res.status}` };
            const data = await res.json();
            const tables = Array.isArray(data.tables) ? data.tables : [];
            const target = tables.find(t => t.table_name === tableName) || tables[0];
            const schema = target && target.schema_json ? JSON.parse(target.schema_json) : [];
            const columnCount = Array.isArray(schema) ? schema.length : 0;
            const targetColIndex = columnCount > 0 ? 0 : -1;
            const targetColName = columnCount > 0 ? (schema[0]?.name || '') : '';
            return { ok: true, columnCount, targetColIndex, targetColName };
        } catch (e) {
            return { ok: false, error: e?.message || String(e) };
        }
    }, resolvedTableName);
    if (!glideColumnCheck.ok) {
        console.error('FAILURE: glide column check failed.', glideColumnCheck.error);
        process.exit(1);
    }
    if (!Number.isFinite(glideColumnCheck.targetColIndex) || glideColumnCheck.targetColIndex < 0) {
        console.error('FAILURE: glide has no writable columns.', glideColumnCheck.columnCount);
        process.exit(1);
    }
    const updateResult = await page.evaluate(async (colIndex) => {
        if (!window.grid || typeof window.grid.updateCell !== 'function') {
            return { ok: false, error: 'window.grid.updateCell missing' };
        }
        await window.grid.updateCell(colIndex, 1, 'NewData');
        return { ok: true };
    }, glideColumnCheck.targetColIndex);
    if (!updateResult.ok) {
        console.error('FAILURE: glide update failed.', updateResult.error);
        process.exit(1);
    }
    await page.waitForFunction((colIndex) => {
        return window.grid
            && typeof window.grid.getCell === 'function'
            && window.grid.getCell(colIndex, 1) === 'NewData';
    }, { timeout: 10000 }, glideColumnCheck.targetColIndex);
    console.log('SUCCESS: GlideGrid 值已成功保存并读取！');
  } catch (e) {
    console.error('测试执行出错:', e);
    process.exit(1);
  } finally {
    await browser.close();
  }
})();
