# Frontend Test Session Reliability Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在前端测试脚本中自动创建/复用会话，避免“会话不存在”导致的超时或失败。

**Architecture:** 在每个 Puppeteer 脚本中增加会话确保逻辑，优先复用已有会话，必要时创建并切换；关键等待点加入一次轻量重试。

**Tech Stack:** Node.js, Puppeteer, Vite

---

### Task 1: 更新 smoke_test.js 的会话确保逻辑

**Files:**
- Modify: `frontend/scripts/smoke_test.js`

**Step 1: Write the failing test**

```bash
VITE_DEV_SERVER_PORT=5174 node scripts/smoke_test.js
```

**Step 2: Run test to verify it fails**

Expected: 出现“会话不存在”或等待超时。

**Step 3: Write minimal implementation**

```js
await page.waitForFunction(() => window.app?.createSession && window.app?.switchSession);
await page.evaluate(async () => {
  // 复用/创建会话并切换为当前
});
```

**Step 4: Run test to verify it passes**

```bash
VITE_DEV_SERVER_PORT=5174 node scripts/smoke_test.js
```

**Step 5: Commit**

说明：根据当前约束不进行提交，如需提交会单独请求授权。

---

### Task 2: 更新 verify_tc15.js 的会话确保逻辑

**Files:**
- Modify: `frontend/verify_tc15.js`

**Step 1: Write the failing test**

```bash
VITE_DEV_SERVER_PORT=5174 node verify_tc15.js
```

**Step 2: Run test to verify it fails**

Expected: 等待 WasmGrid 或数据加载超时。

**Step 3: Write minimal implementation**

```js
await page.waitForFunction(() => window.app?.createSession && window.app?.switchSession);
await page.evaluate(async () => {
  // 复用/创建会话并切换为当前
});
```

**Step 4: Run test to verify it passes**

```bash
VITE_DEV_SERVER_PORT=5174 node verify_tc15.js
```

**Step 5: Commit**

说明：根据当前约束不进行提交，如需提交会单独请求授权。

---

### Task 3: 更新 verify_multisession.js 的会话确保逻辑

**Files:**
- Modify: `frontend/verify_multisession.js`

**Step 1: Write the failing test**

```bash
VITE_DEV_SERVER_PORT=5174 node verify_multisession.js
```

**Step 2: Run test to verify it fails**

Expected: 会话创建或数据加载阶段卡住。

**Step 3: Write minimal implementation**

```js
await page.waitForFunction(() => window.app?.createSession && window.app?.switchSession);
await page.evaluate(async () => {
  // 复用/创建会话并切换为当前
});
```

**Step 4: Run test to verify it passes**

```bash
VITE_DEV_SERVER_PORT=5174 node verify_multisession.js
```

**Step 5: Commit**

说明：根据当前约束不进行提交，如需提交会单独请求授权。

---

### Task 4: 运行 lint/typecheck 与回归验证

**Files:**
- Test: `frontend/package.json`

**Step 1: Run build as typecheck**

```bash
npm run build
```

**Step 2: Verify regression script**

```bash
node scripts/verify_env_cleanup_regression.cjs
```

**Step 3: Commit**

说明：根据当前约束不进行提交，如需提交会单独请求授权。
