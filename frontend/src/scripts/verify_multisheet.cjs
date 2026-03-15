const puppeteer = require('puppeteer');

(async () => {
    const browser = await puppeteer.launch({
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox'],
        protocolTimeout: 60000
    });
    const page = await browser.newPage();
    await page.setViewport({ width: 1200, height: 800 });

    try {
        // **[2026-03-11]** 变更原因：需要稳定复现“grid-data 非 JSON 导致网格卡死”问题
        // **[2026-03-11]** 变更目的：通过拦截请求模拟 404，验证前端 fallback 是否生效
        await page.setRequestInterception(true);
        page.on('request', (request) => {
            const url = request.url();
            if (url.includes('/api/grid-data')) {
                request.respond({
                    status: 404,
                    contentType: 'text/plain',
                    body: 'Not Found'
                });
                return;
            }
            request.continue();
        });

        // **[2026-03-11]** 变更原因：端口常变化导致脚本误报
        // **[2026-03-11]** 变更目的：统一从环境变量读取端口并允许外部覆盖
        const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
        const BASE_URL = process.env.SMOKE_BASE_URL || `http://localhost:${PORT}`;
        console.log(`Navigating to app (${BASE_URL})...`);
        await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

        await page.waitForSelector('select.status-select', { timeout: 12000 });
        const selectedTable = await page.evaluate(() => {
            const select = document.querySelector('select.status-select');
            if (!(select instanceof HTMLSelectElement)) return '';
            const options = Array.from(select.options).map(o => o.value).filter(Boolean);
            if (options.length === 0) return '';
            select.value = options[0];
            select.dispatchEvent(new Event('change', { bubbles: true }));
            return options[0];
        });
        if (!selectedTable) {
            throw new Error('No available table to test fallback path.');
        }
        console.log(`Selected table: ${selectedTable}`);

        await page.waitForFunction(() => {
            const status = document.querySelector('.status-debug');
            const loadingText = document.body.innerText || '';
            return Boolean(status && !loadingText.includes('Loading Grid Metadata...'));
        }, { timeout: 15000 });

        await page.waitForSelector('canvas', { timeout: 15000 });
        console.log('PASS: grid-data 404 fallback verified (canvas rendered).');

    } catch (e) {
        console.error("Test failed:", e);
        await page.screenshot({ path: 'src/scripts/verify_multisheet_error.png' });
        process.exit(1);
    } finally {
        await browser.close();
    }
})();
