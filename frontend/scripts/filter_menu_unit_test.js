import fs from "fs";
import path from "path";

const filePath = path.join(process.cwd(), "src", "components", "GlideGrid.tsx");
const content = fs.readFileSync(filePath, "utf-8");

const requiredSnippets = [
    "openFilterMenuAt",
    "safeCol === 0 ? 0",
    "parseJsonResponseSafely(res)"
];

const missing = [
    ...requiredSnippets.filter(t => !content.includes(t))
];

const hasUnsafeFilterJsonParsing = /const\s+res\s*=\s*await\s+fetch\(url\);\s*const\s+json\s*=\s*await\s+res\.json\(\);/s.test(content);
if (hasUnsafeFilterJsonParsing) {
    missing.push("筛选值拉取仍使用不安全的 res.json()");
}

if (missing.length > 0) {
    console.error(`缺少筛选弹层关键文案: ${missing.join(", ")}`);
    process.exit(1);
}

console.log("筛选弹层样式单元测试通过");
