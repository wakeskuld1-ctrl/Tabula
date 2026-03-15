// **[2026-02-16]** 变更原因：需要端到端验证 Utf8 聚合的 API 行为。
// **[2026-02-16]** 变更目的：覆盖 register_table -> update_cell -> execute 的完整链路。
const fs = require('fs');
const path = require('path');

// **[2026-02-16]** 变更原因：避免硬编码服务地址导致脚本不可复用。
// **[2026-02-16]** 变更目的：允许通过环境变量覆盖测试服务地址。
const BASE_URL = process.env.BASE_URL || 'http://localhost:3000/api';

async function runTest() {
    // **[2026-02-16]** 变更原因：E2E 用例需要清晰的运行起点。
    // **[2026-02-16]** 变更目的：输出提示并便于定位失败步骤。
    console.log('Starting UTF8 Aggregate Verification...');

    // **[2026-02-16]** 变更原因：统一 HTTP 调用避免重复代码。
    // **[2026-02-16]** 变更目的：简化测试主体逻辑。
    const get = async (url) => {
        const res = await fetch(url);
        if (!res.ok) {
            throw new Error(`GET ${url} failed: ${res.statusText}`);
        }
        return await res.json();
    };

    // **[2026-02-16]** 变更原因：多个 API 使用 JSON POST。
    // **[2026-02-16]** 变更目的：统一错误处理与返回值。
    const post = async (url, body) => {
        const res = await fetch(url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body)
        });
        if (!res.ok) {
            const txt = await res.text();
            throw new Error(`POST ${url} failed: ${res.status} ${txt}`);
        }
        return await res.json();
    };

    // **[2026-02-16]** 变更原因：脚本需先确保后端可达。
    // **[2026-02-16]** 变更目的：避免后续步骤误判为功能失败。
    try {
        await get(`${BASE_URL}/health`);
        console.log('✅ Backend is reachable.');
    } catch (e) {
        console.error('❌ Backend not reachable. Is it running?');
        return;
    }

    // **[2026-02-16]** 变更原因：测试需要 Utf8 列含数字/非数字/空值。
    // **[2026-02-16]** 变更目的：触发 TRY_CAST/NULLIF 的忽略非数字语义。
    const fileDir = path.join(__dirname, 'data');
    if (!fs.existsSync(fileDir)) {
        fs.mkdirSync(fileDir, { recursive: true });
    }
    const filePath = path.join(fileDir, `verify_utf8_${Date.now()}.csv`);
    fs.writeFileSync(filePath, 'amount\n123\n999\n\n');

    // **[2026-02-16]** 变更原因：update_cell 依赖 metadata 注册记录。
    // **[2026-02-16]** 变更目的：确保自动水合能够找到数据源。
    const tableName = `verify_sales_${Date.now()}`;
    const registerRes = await post(`${BASE_URL}/register_table`, {
        file_path: filePath,
        table_name: tableName,
        source_type: 'csv'
    });
    if (registerRes.status !== 'ok') {
        console.error('❌ Register table failed:', registerRes);
        return;
    }
    console.log('✅ Table registered.');

    // **[2026-02-16]** 变更原因：需要通过 update_cell 写入非数字值。
    // **[2026-02-16]** 变更目的：验证更新后 SUM/AVG 仍忽略非数字。
    const updateRes = await post(`${BASE_URL}/update_cell`, {
        session_id: null,
        table_name: tableName,
        row_idx: 1,
        col_idx: 0,
        col_name: 'amount',
        old_value: '999',
        new_value: 'abc'
    });
    if (updateRes.status !== 'ok') {
        console.error('❌ Update cell failed:', updateRes);
        return;
    }
    console.log('✅ Cell updated.');

    // **[2026-02-16]** 变更原因：需要验证别名与 DISTINCT 的聚合重写。
    // **[2026-02-16]** 变更目的：覆盖 SUM/AVG/AVG DISTINCT 的执行链路。
    const queryRes = await post(`${BASE_URL}/execute`, {
        sql: `SELECT SUM(s.amount) AS total, AVG(s.amount) AS avg, AVG(DISTINCT s.amount) AS avg_distinct FROM ${tableName} s`
    });
    if (queryRes.error !== null) {
        console.error('❌ Execute failed:', queryRes.error);
        return;
    }
    const rows = queryRes.rows || [];
    if (!rows.length) {
        console.error('❌ Execute returned no rows:', queryRes);
        return;
    }
    const total = parseFloat(rows[0][0]);
    const avg = parseFloat(rows[0][1]);
    const avgDistinct = parseFloat(rows[0][2]);
    if (Number.isNaN(total) || Number.isNaN(avg) || Number.isNaN(avgDistinct)) {
        console.error('❌ Execute returned non-numeric output:', rows[0]);
        return;
    }
    if (Math.abs(total - 123) > 1e-9 || Math.abs(avg - 123) > 1e-9 || Math.abs(avgDistinct - 123) > 1e-9) {
        console.error('❌ Aggregate result mismatch:', rows[0]);
        return;
    }
    console.log('✅ UTF8 aggregate results are correct.');

    // **[2026-02-16]** 变更原因：避免本地测试数据堆积。
    // **[2026-02-16]** 变更目的：保持仓库 data 目录整洁。
    try {
        fs.unlinkSync(filePath);
    } catch (e) {
        console.warn('⚠️ Failed to remove temp file:', filePath);
    }
}

runTest();
