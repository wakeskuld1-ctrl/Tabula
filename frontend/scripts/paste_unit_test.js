import fs from "fs";
import path from "path";

const filePath = path.join(process.cwd(), "src", "components", "GlideGrid.tsx");
const content = fs.readFileSync(filePath, "utf-8");

const requiredTexts = [
    "粘贴失败：",
    "Paste ignored",
    "Batch paste non-JSON",
    "Optimistic paste non-JSON",
    "data-testid=\"glide-grid\""
];

const missing = requiredTexts.filter(t => !content.includes(t));

if (missing.length > 0) {
    console.error(`缺少粘贴处理关键文案或日志: ${missing.join(", ")}`);
    process.exit(1);
}

if (content.includes("return false;")) {
    console.error("粘贴回调仍包含返回 false 的逻辑");
    process.exit(1);
}

console.log("粘贴处理单元测试通过");
