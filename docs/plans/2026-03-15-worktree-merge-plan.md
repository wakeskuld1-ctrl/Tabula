# Worktree Merge & Conflict Resolution Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 `D:\Rust\metadata\.worktrees` 下的改动安全合入 master，冲突优先采用新版本，同时保留旧版本来源（不清理 worktree）。

**Architecture:** 以“每个 worktree 先临时提交 -> master 逐个 cherry-pick”的方式合并；发生冲突时采用 worktree 版本；旧版本通过保留 worktree 不做清理。合并后统一回填 `.trae/CHANGELOG_TASK.md`。

**Tech Stack:** Git, PowerShell, Node/Vite, Rust

---

### Task 1: 校验 master 与 worktree 状态（不做改动）

**Files:**
- Inspect: `D:\Rust\metadata` (git status / worktree list)

**Step 1: 记录 master 当前状态**

Run: `git status -sb`
Expected: 列出当前脏文件与未跟踪文件（用于后续 WIP）

**Step 2: 记录 worktree 列表**

Run: `git worktree list`
Expected: 列出 `.worktrees` 下各分支

**Step 3: 确认冲突处理规则**

- 冲突以“worktree 新版本”为主
- 旧版本保留在对应 worktree（不清理）

**Step 4: 记录到任务日志**

更新 `.trae/CHANGELOG_TASK.md` 记录本次执行开始

---

### Task 2: 让 master 变为可 cherry-pick 的干净状态

**Files:**
- Modify: `D:\Rust\metadata` (git index / commit)

**Step 1: 方案A - WIP 提交 master 当前改动**

Run: `git add -A`
Run: `git commit -m "chore: wip before worktree merge"`
Expected: master 变干净，允许 cherry-pick

**Step 2: 若提交失败（无改动）则跳过**

Run: `git status -sb`
Expected: 若已干净则继续下一步

---

### Task 3: 合并 `formula-docs-plan` worktree

**Files:**
- Source: `D:\Rust\metadata\.worktrees\formula-docs-plan`
- Modify: `D:\Rust\metadata` (cherry-pick)

**Step 1: 确认 worktree 提交存在**

Run: `git log -1 --oneline` (worktree 路径)
Expected: 找到 `4689312` 或对应临时提交

**Step 2: master 上 cherry-pick**

Run: `git cherry-pick <commit>`
Expected: 如有冲突，选择 worktree 版本并解决

**Step 3: 记录结果**

更新 `.trae/CHANGELOG_TASK.md`

---

### Task 4: 合并 `readjsonorthrow-unify` worktree

**Files:**
- Source: `D:\Rust\metadata\.worktrees\readjsonorthrow-unify`
- Modify: `D:\Rust\metadata`

**Step 1: worktree 内创建临时提交（如有未提交）**

Run: `git add -A`
Run: `git commit -m "chore: snapshot readjsonorthrow-unify"`
Expected: 产生可 cherry-pick 的提交

**Step 2: master 上 cherry-pick**

Run: `git cherry-pick <commit>`
Expected: 冲突优先采用 worktree 新版本

**Step 3: 记录结果**

更新 `.trae/CHANGELOG_TASK.md`

---

### Task 5: 合并 `cache-manager-optimization` worktree

**Files:**
- Source: `D:\Rust\metadata\.worktrees\cache-manager-optimization`
- Modify: `D:\Rust\metadata`

**Step 1: worktree 内创建临时提交（如有未提交）**

Run: `git add -A`
Run: `git commit -m "chore: snapshot cache-manager-optimization"`
Expected: 产生可 cherry-pick 的提交

**Step 2: master 上 cherry-pick**

Run: `git cherry-pick <commit>`
Expected: 冲突优先采用 worktree 新版本

**Step 3: 记录结果**

更新 `.trae/CHANGELOG_TASK.md`

---

### Task 6: 处理仅包含 `winlibs.zip` 的 worktree

**Files:**
- Source: `frontend-test-sessions`, `split-spill-frontend`, `formula-failure-retest`, `split-spill`
- Modify: `D:\Rust\metadata`

**Step 1: 确认仅为 `winlibs.zip`**

Run: `git status -sb` (各 worktree)
Expected: 仅新增 `winlibs.zip`

**Step 2: 决定是否纳入 master**

- 若不纳入：不 cherry-pick，保留 worktree 不清理
- 若纳入：临时提交后 cherry-pick

**Step 3: 记录结果**

更新 `.trae/CHANGELOG_TASK.md`

---

### Task 7: 合并后验证（不做额外改动）

**Files:**
- Verify: `D:\Rust\metadata`

**Step 1: 前端构建验证**

Run: `npm run build` (在 `D:\Rust\metadata\frontend`)
Expected: 构建成功或仅已知历史告警

**Step 2: Rust 基础检查（如需要）**

Run: `cargo check` (在 `D:\Rust\metadata`)
Expected: 通过或记录已知历史问题

**Step 3: 记录结果**

更新 `.trae/CHANGELOG_TASK.md`

---

### Task 8: 完成收尾说明

**Files:**
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: 汇总合并结果与遗留问题**

- 标注冲突采用新版本
- 旧版本保留在 worktree
- 列出未纳入 master 的 worktree（如有）

**Step 2: 结束记录**

确保日志完整
