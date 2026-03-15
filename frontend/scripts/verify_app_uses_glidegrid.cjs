const fs = require('fs');
const path = require('path');

const appPath = path.resolve(__dirname, '../src/App.tsx');
const source = fs.readFileSync(appPath, 'utf8');

const hasGlideImport = /import\s+\{[^}]*\bGlideGrid\b[^}]*\}\s+from\s+['"]\.\/components\/GlideGrid['"]/.test(source);
const hasWasmImport = /import\s+WasmGrid\s+from\s+['"]\.\/components\/WasmGrid['"]/.test(source);
const hasGlideRender = /<GlideGrid[\s>]/.test(source);
const hasWasmRender = /<WasmGrid[\s>]/.test(source);
const hasSavePlaceholder = /onSave=\{\(\)\s*=>\s*setDebugInfo\('保存动作已触发'\)\}/.test(source);
const hasAddSheetPlaceholder = /onAddSheet=\{\(\)\s*=>\s*setDebugInfo\('当前版本暂不支持新建表'\)\}/.test(source);
const hasDeleteSheetPlaceholder = /onDeleteSheet=\{\(sheet\)\s*=>\s*setDebugInfo\(`当前版本暂不支持删除表: \$\{sheet\}`\)\}/.test(source);

const failures = [];

if (!hasGlideImport) failures.push('App.tsx 未引入 GlideGrid');
if (!hasGlideRender) failures.push('App.tsx 未渲染 GlideGrid');
if (hasWasmImport) failures.push('App.tsx 仍在引入 WasmGrid');
if (hasWasmRender) failures.push('App.tsx 仍在渲染 WasmGrid');
if (hasSavePlaceholder) failures.push('保存按钮仍是占位实现');
if (hasAddSheetPlaceholder) failures.push('新增表按钮仍是占位实现');
if (hasDeleteSheetPlaceholder) failures.push('删除表按钮仍是占位实现');

if (failures.length > 0) {
  console.error('[FAIL] verify_app_uses_glidegrid');
  for (const failure of failures) {
    console.error(` - ${failure}`);
  }
  process.exit(1);
}

console.log('[PASS] verify_app_uses_glidegrid');
