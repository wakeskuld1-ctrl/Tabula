// **[2026-02-26]** 变更原因：缺少透视表 UI 回归覆盖；变更目的：以 TDD 方式验证输出位置与字段可见性。
// **[2026-02-26]** 变更原因：需要复现“字段不可见/输出选择难发现”；变更目的：自动化可重复执行。
// **[2026-02-26]** 变更原因：CI/本地都要跑；变更目的：无头浏览器脚本保持一致入口。
const puppeteer = require('puppeteer');

(async () => {
  // **[2026-02-26]** 变更原因：日志不足以定位问题；变更目的：明确测试阶段与失败点。
  console.log('Starting pivot UI TDD test...');

  // **[2026-02-26]** 变更原因：需要稳定视口；变更目的：保证布局可预测以便选择器定位。
  const browser = await puppeteer.launch({
    headless: "new",
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--window-size=1280,800'],
    defaultViewport: { width: 1280, height: 800 }
  });

  // **[2026-02-26]** 变更原因：页面交互需要独立上下文；变更目的：隔离测试状态。
  const page = await browser.newPage();

  // **[2026-02-26]** 变更原因：需要捕获 Schema 错误提示；变更目的：在自动化中判断创建失败。
  const dialogMessages = [];
  // **[2026-02-26]** 变更原因：弹窗会阻断流程；变更目的：自动关闭并记录提示内容。
  page.on('dialog', async (dialog) => {
    const message = dialog.message();
    dialogMessages.push(message);
    console.log('Dialog:', message);
    await dialog.dismiss();
  });

  // **[2026-02-26]** 变更原因：前端报错不易定位；变更目的：打印浏览器错误。
  page.on('pageerror', err => console.error('PAGE ERROR:', err.toString()));
  // **[2026-02-26]** 变更原因：需要判断 SQL 是否带引号；变更目的：定位大小写字段报错来源。
  const sqlLogs = [];
  // **[2026-02-26]** 变更原因：追踪前端日志；变更目的：辅助定位 UI 未出现原因。
  page.on('console', msg => {
    const text = msg.text();
    if (text.startsWith('Generated SQL:')) {
      sqlLogs.push(text);
    }
    console.log('PAGE LOG:', text);
  });

  try {
    // **[2026-02-26]** 变更原因：端口硬编码不可复用；变更目的：统一读取环境变量。
    const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
    // **[2026-02-26]** 变更原因：需支持外部覆盖；变更目的：允许指定基础 URL。
    const BASE_URL = process.env.SMOKE_BASE_URL || `http://localhost:${PORT}`;
    console.log(`Navigating to ${BASE_URL}...`);
    await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

    // **[2026-02-26]** 变更原因：页面未完全就绪会导致误判；变更目的：等待状态栏出现。
    await page.waitForSelector('.status-bar', { timeout: 10000 });

    // **[2026-02-26]** 变更原因：测试依赖全局 app 暴露；变更目的：确保可通过脚本选表。
    await page.waitForFunction(() => {
      return window.app && typeof window.app.selectTable === 'function';
    }, { timeout: 10000 });

    // **[2026-02-26]** 变更原因：确保当前表已选中；变更目的：透视表字段请求基于当前表。
    const tableSelected = await page.evaluate(async () => {
      try {
        const res = await fetch('/api/tables');
        const data = await res.json();
        const tables = Array.isArray(data.tables) ? data.tables : [];
        const preferred = tables.find(t => t?.table_name === 'orders');
        const target = preferred?.table_name || tables[0]?.table_name || '';
        if (!target) return { ok: false, error: 'no table' };
        await window.app.selectTable(target);
        return { ok: true, name: target };
      } catch (e) {
        return { ok: false, error: e?.message || String(e) };
      }
    });
    if (!tableSelected.ok) {
      console.error('❌ Pivot UI Test Failed: select table failed.', tableSelected.error);
      process.exit(1);
    }
    console.log(`Selected table: ${tableSelected.name}`);

    // **[2026-02-26]** 变更原因：部分 Puppeteer 版本不支持 waitForTimeout；变更目的：用 setTimeout 替代保持兼容。
    await new Promise(resolve => setTimeout(resolve, 800));

    // **[2026-02-26]** 变更原因：必须打开透视面板；变更目的：验证输出位置与字段区域。
    await page.click('button[title="Insert Pivot Table"]');

    // **[2026-02-26]** 变更原因：确保面板可见；变更目的：避免误判为加载失败。
    await page.waitForSelector('.pivot-sidebar', { timeout: 8000 });

    // **[2026-02-26]** 变更原因：TDD 期望定位输出区域；变更目的：稳定选择器必须存在。
    await page.waitForSelector('[data-testid="pivot-output-mode"]', { timeout: 5000 });

    // **[2026-02-26]** 变更原因：需要验证字段列表可见；变更目的：确保字段项真实渲染。
    await page.waitForSelector('[data-testid="pivot-field-list"]', { timeout: 5000 });
    // **[2026-02-26]** 变更原因：字段加载存在异步请求；变更目的：等待字段项实际渲染。
    await page.waitForFunction(() => {
      return document.querySelectorAll('[data-testid="pivot-field-item"]').length > 0;
    }, { timeout: 8000 });

    // **[2026-02-26]** 变更原因：字段不可见问题需验证对比度；变更目的：检查文字与背景不相同。
    const contrastOk = await page.$eval('[data-testid="pivot-field-item"]', (el) => {
      const style = window.getComputedStyle(el);
      const color = style.color;
      const bg = style.backgroundColor;
      return !!color && !!bg && color !== bg;
    });
    if (!contrastOk) {
      console.error('❌ Pivot UI Test Failed: field text color equals background.');
      process.exit(1);
    }

    // **[2026-02-26]** 变更原因：输出模式默认值需明确；变更目的：保证新建 Sheet 为默认选项。
    const outputDefault = await page.$eval('[data-testid="pivot-output-new-sheet"]', (el) => {
      return el instanceof HTMLInputElement ? el.checked : false;
    });
    if (!outputDefault) {
      console.error('❌ Pivot UI Test Failed: default output mode is not new-sheet.');
      process.exit(1);
    }

    // **[2026-02-26]** 变更原因：需复现“点击创建报错”；变更目的：捕获大小写字段导致的 Schema error。
    const targetFieldLabel = await page.evaluate(async (tableName) => {
      try {
        const res = await fetch('/api/execute', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ sql: `SELECT * FROM "${tableName}" LIMIT 1` })
        });
        const data = await res.json();
        const cols = Array.isArray(data.columns) ? data.columns : [];
        const preferred = cols.find((col) => String(col).startsWith('NewCol_'));
        return preferred || cols[0] || '';
      } catch (e) {
        return '';
      }
    }, tableSelected.name);
    if (!targetFieldLabel) {
      console.error('❌ Pivot UI Test Failed: no field label resolved for pivot create.');
      process.exit(1);
    }

    // **[2026-02-26]** 变更原因：拖拽需精确目标；变更目的：选择字段与 Values 区域生成透视。
    const fieldIndex = await page.$$eval('[data-testid="pivot-field-item"]', (items, target) => {
      return items.findIndex(item => item.textContent?.trim() === target);
    }, targetFieldLabel);
    if (fieldIndex < 0) {
      console.error('❌ Pivot UI Test Failed: target field not found.', targetFieldLabel);
      process.exit(1);
    }

    const fieldHandles = await page.$$('[data-testid="pivot-field-item"]');
    const targetField = fieldHandles[fieldIndex];
    const valueZone = await page.$('[data-testid="pivot-drop-values"]');
    if (!targetField || !valueZone) {
      console.error('❌ Pivot UI Test Failed: drag handles not found.');
      process.exit(1);
    }

    // **[2026-02-26]** 变更原因：拖拽是核心交互；变更目的：模拟从字段到 Values 的放置。
    const dragAndDrop = async (source, target) => {
      const sourceBox = await source.boundingBox();
      const targetBox = await target.boundingBox();
      if (!sourceBox || !targetBox) {
        throw new Error('drag target missing');
      }
      await page.mouse.move(sourceBox.x + sourceBox.width / 2, sourceBox.y + sourceBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(targetBox.x + targetBox.width / 2, targetBox.y + targetBox.height / 2, { steps: 12 });
      await page.mouse.up();
    };

    await dragAndDrop(targetField, valueZone);
    await page.click('.update-btn');
    await new Promise(resolve => setTimeout(resolve, 1200));

    // **[2026-02-26]** 变更原因：创建失败会弹窗；变更目的：阻止 Schema error 进入回归。
    const schemaDialog = dialogMessages.find(message => message.includes('Schema error'));
    if (schemaDialog) {
      console.error('❌ Pivot UI Test Failed: schema error dialog detected.', schemaDialog);
      process.exit(1);
    }

    // **[2026-02-26]** 变更原因：需要验证 SQL 引号；变更目的：避免大小写字段解析失败。
    const generatedSql = sqlLogs.find(text => text.includes('Generated SQL:'));
    if (generatedSql && generatedSql.includes(targetFieldLabel) && !generatedSql.includes(`"${targetFieldLabel}"`)) {
      console.error('❌ Pivot UI Test Failed: SQL identifier missing quotes.', generatedSql);
      process.exit(1);
    }

    console.log('✅ Pivot UI TDD Test Passed.');
  } catch (err) {
    // **[2026-02-26]** 变更原因：异常需明确输出；变更目的：便于定位失败阶段。
    console.error('❌ Pivot UI Test Failed with exception:', err?.message || err);
    process.exit(1);
  } finally {
    // **[2026-02-26]** 变更原因：测试结束需释放资源；变更目的：避免残留进程。
    await browser.close();
  }
})();
