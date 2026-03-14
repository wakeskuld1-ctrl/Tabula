// **[2026-03-14]** 变更原因：Vite 报 Missing catch/finally
// **[2026-03-14]** 变更目的：最小化复现 GlideGrid 语法错误
// **[2026-03-14]** 变更原因：遵循 TDD 先红后绿
// **[2026-03-14]** 变更目的：先让编译失败再修复
// **[2026-03-14]** 变更原因：不引入测试框架
// **[2026-03-14]** 变更目的：使用 node + tsc 轻量验证
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// **[2026-03-14]** 变更原因：仅需语法检查避免类型噪音
// **[2026-03-14]** 变更目的：通过 transpileModule 捕获语法错误
const targetPath = path.resolve(__dirname, "../src/components/GlideGrid.tsx");
const source = fs.readFileSync(targetPath, "utf8");

const result = ts.transpileModule(source, {
  fileName: targetPath,
  compilerOptions: {
    target: ts.ScriptTarget.ES2020,
    module: ts.ModuleKind.ESNext,
    jsx: ts.JsxEmit.ReactJSX
  },
  reportDiagnostics: true
});

const errors = (result.diagnostics ?? []).filter(
  (diag) => diag.category === ts.DiagnosticCategory.Error
);

if (errors.length > 0) {
  const formatted = ts.formatDiagnosticsWithColorAndContext(errors, {
    getCurrentDirectory: () => process.cwd(),
    getCanonicalFileName: (file) => file,
    getNewLine: () => "\n"
  });
  throw new Error(`GlideGrid syntax errors:\n${formatted}`);
}

console.log("glidegrid syntax tests passed");
