# Release Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在不修改 frontend 的前提下，完成仓库发布前整理与清理，保证目录整洁、检查通过、产物可控。

**Architecture:** 以“最小改动 + 可追溯”为原则，先清点并界定可清理项，再做结构归整与代码规范化，最后执行统一验证命令，确保发布质量。

**Tech Stack:** Rust (cargo fmt/clippy/check/test), PowerShell, Git

---

### Task 1: 盘点并分类清理范围

**Files:**
- Modify: `README.md:1-120`（若需补充发布说明，优先改已有文档）
- Modify: `.gitignore:1-120`（如需新增忽略项）

**Step 1: Write the failing test**

无自动化测试可覆盖“目录整洁性”，此任务不新增测试。

**Step 2: Run test to verify it fails**

跳过（无对应测试）。

**Step 3: Write minimal implementation**

- 列出根目录与非 frontend 模块中的候选清理项（如压缩包、生成报告、临时脚本产物）
- 标记“删除 / 迁移 / 保留”的理由与依据
- 若需要新增忽略项，更新 `.gitignore`

**Step 4: Run test to verify it passes**

跳过（无对应测试）。

**Step 5: Commit**

```bash
git add .gitignore README.md
git commit -m "chore: document release cleanup scope"
```

> 提示：提交需用户明确指令才执行。

---

### Task 2: 结构归整与产物清理

**Files:**
- Modify: `doc/` 或 `docs/` 下已有文档（优先移动至已有目录）
- Delete: 明确可再生成的产物文件（待盘点清单）

**Step 1: Write the failing test**

无自动化测试可覆盖“目录归整”，此任务不新增测试。

**Step 2: Run test to verify it fails**

跳过（无对应测试）。

**Step 3: Write minimal implementation**

- 将散落根目录的说明性文件移动到已存在的 `doc/` 或 `docs/` 结构下
- 删除明确可再生成的构建产物或临时文件
- 对每处变更添加“原因/目的/日期”的 Markdown 备注

**Step 4: Run test to verify it passes**

跳过（无对应测试）。

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: organize repository structure for release"
```

> 提示：提交需用户明确指令才执行。

---

### Task 3: 代码规范化与验证

**Files:**
- Modify: `federated_query_engine/src/**`（如 fmt/clippy 需要修复）
- Modify: `metadata_store/src/**`（如 fmt/clippy 需要修复）
- Modify: `src/**`（如 fmt/clippy 需要修复）

**Step 1: Write the failing test**

以 `cargo clippy` 报错作为“失败断言”。

**Step 2: Run test to verify it fails**

Run: `cargo clippy --all-targets --all-features -D warnings`

Expected: 若存在问题，输出警告并以非零退出。

**Step 3: Write minimal implementation**

- 按 clippy 与 fmt 结果逐条修复
- 所有修改处附上“原因/目的/日期”的 Markdown 备注

**Step 4: Run test to verify it passes**

Run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -D warnings
cargo check
cargo test
```

Expected: 全部通过。

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: normalize rust code style"
```

> 提示：提交需用户明确指令才执行。
