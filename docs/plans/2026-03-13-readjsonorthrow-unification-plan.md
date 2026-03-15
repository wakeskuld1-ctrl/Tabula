# readJsonOrThrow Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace all frontend `res.json()` usages with a shared `readJsonOrThrow` helper across runtime code and verification scripts.

**Architecture:** Introduce a single helper module under `frontend/src/utils/` that exports `readJsonOrThrow` plus a Puppeteer-friendly installer for `window.__readJsonOrThrow`. Runtime TypeScript imports the helper directly; Node scripts use direct imports (ESM or dynamic import for CJS); Puppeteer `page.evaluate` uses the injected window helper.

**Tech Stack:** TypeScript, ES modules, Node 18+ fetch/Response, Puppeteer.

---

### Task 1: Add shared helper module

**Files:**
- Create: `frontend/src/utils/readJsonOrThrow.js`
- Create: `frontend/src/utils/readJsonOrThrow.d.ts`

**Step 1: Write a failing test (quick Node check)**

```bash
node --input-type=module -e "import { readJsonOrThrow } from './frontend/src/utils/readJsonOrThrow.js'; const res = new Response('not-json', { status: 200, headers: { 'content-type': 'application/json' } }); try { await readJsonOrThrow(res, 'test'); console.log('UNEXPECTED'); } catch (e) { console.log('EXPECTED'); }"
```

Expected: prints `EXPECTED` once the helper is implemented (will fail before file exists).

**Step 2: Implement minimal helper**

```js
export async function readJsonOrThrow(res, context) {
  const raw = await res.text();
  if (!res.ok) {
    const preview = raw ? raw.slice(0, 240) : '';
    throw new Error(`${context} HTTP ${res.status}: ${preview || res.statusText}`);
  }
  if (!raw || raw.trim().length === 0) {
    throw new Error(`${context} parse failed: empty response (status ${res.status})`);
  }
  try {
    return JSON.parse(raw);
  } catch (err) {
    throw new Error(`${context} parse failed: invalid json (status ${res.status})`);
  }
}

export function installReadJsonOrThrowToWindow() {
  window.__readJsonOrThrow = async (res, context) => {
    const raw = await res.text();
    if (!res.ok) {
      const preview = raw ? raw.slice(0, 240) : '';
      throw new Error(`${context} HTTP ${res.status}: ${preview || res.statusText}`);
    }
    if (!raw || raw.trim().length === 0) {
      throw new Error(`${context} parse failed: empty response (status ${res.status})`);
    }
    try {
      return JSON.parse(raw);
    } catch (err) {
      throw new Error(`${context} parse failed: invalid json (status ${res.status})`);
    }
  };
}
```

Also add `readJsonOrThrow.d.ts`:

```ts
export function readJsonOrThrow(res: Response, context: string): Promise<any>;
export function installReadJsonOrThrowToWindow(): void;
```

**Step 3: Run the Node check again**

Run: (same as Step 1)  
Expected: prints `EXPECTED`

**Step 4: Commit**

```bash
git add frontend/src/utils/readJsonOrThrow.js frontend/src/utils/readJsonOrThrow.d.ts
git commit -m "feat: add shared readJsonOrThrow helper"
```

---

### Task 2: Update runtime TypeScript usage (GlideGrid + GridAPI)

**Files:**
- Modify: `frontend/src/components/GlideGrid.tsx`
- Modify: `frontend/src/utils/GridAPI.ts`

**Step 1: Write the failing test (type check)**

Run: `npm run build`  
Expected: fails after removing local helper but before adding import.

**Step 2: Update GlideGrid**

Replace local helper definitions with import:

```ts
import { readJsonOrThrow } from "@/utils/readJsonOrThrow.js";
```

Then replace `res.json()` calls (including `batch_update_cells`) with:

```ts
const json = await readJsonOrThrow(res, "batch_update_cells");
```

Keep existing `res.ok` + `content-type` checks unchanged.

**Step 3: Update GridAPI**

Import the helper:

```ts
import { readJsonOrThrow } from "@/utils/readJsonOrThrow.js";
```

Replace `return await res.json()` with:

```ts
return await readJsonOrThrow(res, "execute_sql");
```

and similarly for `update_cell` and `batch_update_cells` contexts.

**Step 4: Run build again**

Run: `npm run build`  
Expected: success (no TypeScript errors)

**Step 5: Commit**

```bash
git add frontend/src/components/GlideGrid.tsx frontend/src/utils/GridAPI.ts
git commit -m "refactor: use readJsonOrThrow in runtime api"
```

---

### Task 3: Update Puppeteer scripts + Node verification scripts

**Files:**
- Modify: `frontend/scripts/smoke_test.js`
- Modify: `frontend/verify_multisession.js`
- Modify: `frontend/verify_numeric_edit.js`
- Modify: `frontend/verify_tc15.js`
- Modify: `frontend/scripts/reproduce_issue.cjs`
- Modify: `frontend/scripts/verify_pivot_ui.cjs`
- Modify: `frontend/scripts/verify_cross_sheet_formulas.cjs`
- Modify: `frontend/scripts/verify_create_table.cjs`
- Modify: `frontend/src/scripts/verify_state_integration.cjs`

**Step 1: Write the failing test (smoke run)**

Run: `node frontend/scripts/smoke_test.js`  
Expected: fails before helper injection or import is added (manual environment required).

**Step 2: Add helper injection for Puppeteer**

At page creation, add:

```js
import { installReadJsonOrThrowToWindow, readJsonOrThrow } from "../src/utils/readJsonOrThrow.js";
// ...
await page.evaluateOnNewDocument(installReadJsonOrThrowToWindow);
```

Then inside `page.evaluate`, replace:

```js
const data = await res.json();
```

with:

```js
const data = await window.__readJsonOrThrow(res, "context");
```

**Step 3: Update Node-only scripts**

For ESM `.js` scripts:

```js
import { readJsonOrThrow } from "../src/utils/readJsonOrThrow.js";
```

For `.cjs` scripts inside async IIFE:

```js
const { readJsonOrThrow, installReadJsonOrThrowToWindow } = await import("../src/utils/readJsonOrThrow.js");
```

Replace `res.json()` with `readJsonOrThrow(res, "context")`.

**Step 4: Rerun smoke script**

Run: `node frontend/scripts/smoke_test.js`  
Expected: passes (manual environment required).

**Step 5: Commit**

```bash
git add frontend/scripts frontend/verify_*.js frontend/src/scripts
git commit -m "refactor: unify readJsonOrThrow in frontend scripts"
```
