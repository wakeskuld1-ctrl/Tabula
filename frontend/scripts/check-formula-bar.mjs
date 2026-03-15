/*
### Change Log
* - 2026-03-15: Reason=Need a minimal parse check for FormulaBar; Purpose=fail fast on JSX syntax errors (TDD red)
* - 2026-03-15: Reason=Follow bugfix TDD rule; Purpose=provide a reproducible failing test before fix
*/

/*
### Rationale
* - This script is a minimal "test" that ensures the FormulaBar TSX parses.
* - We use esbuild (already in Vite deps) so the check is fast and local.
* - It should FAIL before the bugfix and PASS after the fix.
*/

import path from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { transformSync } from "esbuild";

/*
### Change Log
* - 2026-03-15: Reason=Need stable path resolution; Purpose=avoid cwd-dependent failures
*/
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const targetPath = path.resolve(
    __dirname,
    "../src/components/layout/FormulaBar.tsx"
);

/*
### Change Log
* - 2026-03-15: Reason=Expose syntax errors early; Purpose=signal parse failure as non-zero exit
*/
const source = readFileSync(targetPath, "utf8");

try {
    transformSync(source, {
        loader: "tsx",
        jsx: "automatic",
        sourcemap: false
    });
    console.log("FormulaBar parse OK.");
} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error("FormulaBar parse FAILED:", message);
    process.exit(1);
}
