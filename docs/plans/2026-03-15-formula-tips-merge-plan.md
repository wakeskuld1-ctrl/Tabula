# Formula Tips Worktree Merge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 formula-tips worktree 的全部变更分组提交并合并到主仓库。

**Architecture:** 以分组提交的方式降低审阅和回滚成本，最终通过 cherry-pick 合入主仓库。

**Tech Stack:** Git, Node/Vite, Vitest

---

### Task 1: 盘点变更范围

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\*`

**Step 1: 列出状态**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips status -sb`
Expected: 列出已修改与未跟踪文件

**Step 2: 记录分组清单**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips ls-files --others --exclude-standard`
Expected: 仅未跟踪文件清单

**Step 3: Commit**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips status -sb`
Expected: 仅确认，无提交

---

### Task 2: 合并文档（docs/plans + frontend/docs/plans）

**Files:**
- Create/Modify: `D:\Rust\metadata\.worktrees\formula-tips\docs\plans\*`
- Create/Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\docs\plans\*`

**Step 1: Stage 文档**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips add docs/plans frontend/docs/plans`
Expected: 文档进入暂存区

**Step 2: Commit**

```bash
git -C D:\Rust\metadata\.worktrees\formula-tips commit -m "docs: collect formula tips plans"
```
Expected: Commit 成功

---

### Task 3: 合并功能实现（frontend/src）

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.tsx`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\App.css`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\components\**`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\**`

**Step 1: Stage 功能代码**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips add frontend/src`
Expected: 源码进入暂存区

**Step 2: Commit**

```bash
git -C D:\Rust\metadata\.worktrees\formula-tips commit -m "feat: collect formula tips and fill handle updates"
```
Expected: Commit 成功

---

### Task 4: 合并测试与脚本

**Files:**
- Create/Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\**`
- Create/Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\scripts\**`

**Step 1: Stage 测试与脚本**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips add frontend/src/utils/__tests__ frontend/scripts`
Expected: 测试与脚本进入暂存区

**Step 2: Commit**

```bash
git -C D:\Rust\metadata\.worktrees\formula-tips commit -m "test: collect formula tips tests and scripts"
```
Expected: Commit 成功

---

### Task 5: 合并配置变更

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\tsconfig.json`
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\vite.config.ts`

**Step 1: Stage 配置**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips add frontend/tsconfig.json frontend/vite.config.ts`
Expected: 配置进入暂存区

**Step 2: Commit**

```bash
git -C D:\Rust\metadata\.worktrees\formula-tips commit -m "chore: update frontend config for formula tips"
```
Expected: Commit 成功

---

### Task 6: 验证关键测试（最小）

**Files:**
- Test: `D:\Rust\metadata\.worktrees\formula-tips\frontend\src\utils\__tests__\**`

**Step 1: Run vitest**

Run: `npx vitest run`
Expected: PASS

---

### Task 7: 合并到主仓库（cherry-pick）

**Files:**
- Modify: `D:\Rust\metadata\*`

**Step 1: 列出 formula-tips 新提交**

Run: `git -C D:\Rust\metadata\.worktrees\formula-tips log --oneline --reverse origin/master..HEAD`
Expected: 输出本次分组提交列表

**Step 2: 依序 cherry-pick 到主仓库**

```bash
git -C D:\Rust\metadata cherry-pick <sha1>
```
Expected: 每个提交成功落入主仓库

---

### Task 8: 更新任务日志

**Files:**
- Modify: `D:\Rust\metadata\.worktrees\formula-tips\frontend\.trae\CHANGELOG_TASK.md`
- Modify: `D:\Rust\metadata\.trae\CHANGELOG_TASK.md`

**Step 1: 追加任务记录**

Append 按模板新增条目，说明收口内容与后续事项。

**Step 2: Commit**

```bash
git -C D:\Rust\metadata\.worktrees\formula-tips commit -m "chore: update task journal for formula tips merge"
```
Expected: Commit 成功
