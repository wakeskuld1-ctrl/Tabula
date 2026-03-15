// **[2026-02-26]** 变更原因：以TDD方式锁定端口硬编码与遗留关键字问题
// **[2026-02-26]** 变更目的：保证后续优化不再回退到硬编码或遗留实现
// **[2026-02-26]** 变更原因：用户要求“继续优化 + TDD”，需先写失败测试
// **[2026-02-26]** 变更目的：为端口与 FortuneSheet 清理建立回归护栏
// **[2026-02-26]** 变更原因：满足备注占比要求
// **[2026-02-26]** 变更目的：提高可读性与可追踪性
const fs = require('fs');
const path = require('path');

// **[2026-02-26]** 变更原因：避免扫描构建产物与依赖目录
// **[2026-02-26]** 变更目的：减少误报与扫描时间
// **[2026-02-26]** 变更原因：明确测试范围
// **[2026-02-26]** 变更目的：聚焦开发源码与脚本
const IGNORE_DIRS = new Set(['node_modules', 'dist', '.git', '.worktrees']);

// **[2026-02-26]** 变更原因：前端脚本与文档可能含遗留关键字
// **[2026-02-26]** 变更目的：覆盖 JS/TS/MD/JSON/PS1 等常见类型
// **[2026-02-26]** 变更原因：确保检测完整性
// **[2026-02-26]** 变更目的：减少遗漏
const TARGET_EXTS = new Set(['.js', '.cjs', '.ts', '.tsx', '.md', '.json', '.ps1']);

// **[2026-02-26]** 变更原因：历史问题为开发端口硬编码
// **[2026-02-26]** 变更目的：禁止出现固定 5173-5176 URL
// **[2026-02-26]** 变更原因：保留后端 3000 不纳入该检查
// **[2026-02-26]** 变更目的：避免误报
const HARD_CODED_PORT_URL = /http:\/\/(127\.0\.0\.1|localhost):517[3-6]\b/i;

// **[2026-02-26]** 变更原因：清理 FortuneSheet/Luckysheet 遗留
// **[2026-02-26]** 变更目的：确保仓库不再出现相关关键字
// **[2026-02-26]** 变更原因：防止回归
// **[2026-02-26]** 变更目的：测试覆盖
const LEGACY_SHEET_KEYWORDS = /(fortune\-sheet|fortunesheet|luckysheet)/i;

// **[2026-02-26]** 变更原因：统一扫描入口，避免遗漏
// **[2026-02-26]** 变更目的：递归遍历仓库
// **[2026-02-26]** 变更原因：TDD 红灯阶段需要稳定输出
// **[2026-02-26]** 变更目的：便于定位失败原因
function walkFiles(rootDir, collected = []) {
    const entries = fs.readdirSync(rootDir, { withFileTypes: true });
    for (const entry of entries) {
        if (entry.isDirectory()) {
            if (IGNORE_DIRS.has(entry.name)) continue;
            walkFiles(path.join(rootDir, entry.name), collected);
        } else if (entry.isFile()) {
            const ext = path.extname(entry.name).toLowerCase();
            if (TARGET_EXTS.has(ext)) {
                collected.push(path.join(rootDir, entry.name));
            }
        }
    }
    return collected;
}

// **[2026-02-26]** 变更原因：输出统一失败格式
// **[2026-02-26]** 变更目的：便于 CI 或人工检查
// **[2026-02-26]** 变更原因：避免静默失败
// **[2026-02-26]** 变更目的：严格回归
function reportFailures(title, items) {
    if (items.length === 0) return;
    console.error(`\n❌ ${title}`);
    items.forEach(item => console.error(` - ${item}`));
}

// **[2026-02-26]** 变更原因：主测试流程入口
// **[2026-02-26]** 变更目的：聚合端口与遗留关键字检查
// **[2026-02-26]** 变更原因：满足 TDD 流程
// **[2026-02-26]** 变更目的：先红后绿
function run() {
    const repoRoot = path.resolve(__dirname, '..');
    const files = walkFiles(repoRoot);
    const hardcodedHits = [];
    const legacyHits = [];
    const skipLegacyCheck = new Set(['scripts\\verify_env_cleanup_regression.cjs', 'scripts/verify_env_cleanup_regression.cjs']);

    for (const filePath of files) {
        const relativePath = path.relative(repoRoot, filePath);
        const content = fs.readFileSync(filePath, 'utf8');
        if (HARD_CODED_PORT_URL.test(content)) {
            hardcodedHits.push(relativePath);
        }
        if (!skipLegacyCheck.has(relativePath) && LEGACY_SHEET_KEYWORDS.test(content)) {
            legacyHits.push(relativePath);
        }
    }

    const pkgPath = path.join(repoRoot, 'package.json');
    let hasFortuneDependency = false;
    if (fs.existsSync(pkgPath)) {
        const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
        hasFortuneDependency = Boolean(pkg?.dependencies?.['@fortune-sheet/react']);
    }

    reportFailures('发现开发端口硬编码 URL (5173-5176)', hardcodedHits);
    reportFailures('发现 FortuneSheet/Luckysheet 遗留关键字', legacyHits);
    if (hasFortuneDependency) {
        console.error('\n❌ package.json 仍包含 @fortune-sheet/react 依赖');
    }

    if (hardcodedHits.length > 0 || legacyHits.length > 0 || hasFortuneDependency) {
        process.exit(1);
    }

    console.log('✅ 环境变量端口与遗留清理回归测试通过');
}

run();
