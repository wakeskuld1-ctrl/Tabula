# Design: Unify readJsonOrThrow Usage

Date: 2026-03-13
Status: Approved

## Goal
Unify JSON parsing and error handling for all frontend runtime code and related verification scripts by replacing direct `res.json()` calls with a single shared `readJsonOrThrow` helper.

## Scope
- Runtime code: `frontend/src/components/GlideGrid.tsx`, `frontend/src/utils/GridAPI.ts`
- Node scripts: `frontend/scripts/**`, `frontend/verify_*.js`, `frontend/src/scripts/**`
- In-page `fetch` calls inside Puppeteer `page.evaluate(...)`

## Architecture
Create a single shared helper under `frontend/src/utils/readJsonOrThrow.js` with a paired type declaration `readJsonOrThrow.d.ts`.
- Runtime TypeScript imports the helper directly.
- Node scripts import the same helper file.
- Puppeteer scripts inject the helper into `window.__readJsonOrThrow` via `page.evaluateOnNewDocument` so `page.evaluate` can call it.

## Data Flow
1) `fetch(...)` returns `Response`
2) `readJsonOrThrow(res, context)` reads text, validates JSON, checks `res.ok`
3) Caller handles returned JSON or thrown error as before

## Error Handling
`readJsonOrThrow`:
- Reads raw text for better diagnostics
- Throws on non-OK status with context + preview
- Throws on invalid/empty JSON with context + preview

Existing caller behavior (alerts, retries, fallbacks) is preserved; only JSON parsing is centralized.

## Testing
No new automated tests required for this refactor.
- Optional: run existing Puppeteer/verify scripts as smoke checks if desired.

## Alternatives Considered
- Per-file helper (rejected: inconsistent, drifts over time)
- Separate runtime vs. script helpers (rejected: duplicated logic)
