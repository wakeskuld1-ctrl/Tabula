# Legacy Worktree Notice Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在指定 worktree 的每个代码文件头部写入遗留项说明，并为 JSON 文件添加 `__legacy_notice` 或生成 `filename.legacy.notice.md`。

**Architecture:** 通过脚本扫描指定 worktree、按文件类型选择注释样式；对 JSON 进行结构判断与最小化插入；对非对象 JSON 创建旁路说明文件；跳过不可编辑文件。

**Tech Stack:** PowerShell, Python

---

### Task 1: 准备注释模板与规则映射

**Files:**
- Create: `D:\Rust\metadata\docs\plans\2026-03-15-legacy-worktree-notice-implementation-plan.md`

**Step 1: 定义统一说明内容（英文 ASCII）**

示例：
- 标题：`Legacy Worktree Notice (2026-03-15)`
- 原因：保留历史修复/实验版本，担心影响而保留
- 背景：这些 worktree 属于遗留项，不参与主线更新
- 指令：禁止修改/更新/合并，除非明确指示
- 来源：worktree 名称与路径

**Step 2: 定义注释样式映射**

- `//`：`.rs/.ts/.tsx/.js/.jsx/.go/.java/.cs/.cpp/.c/.h`
- `#`：`.py/.sh/.ps1/.toml/.yml/.yaml`
- `/* */`：`.css/.scss`
- `<!-- -->`：`.html`
- `--`：`.sql`

---

### Task 2: 编写脚本并做 dry-run

**Files:**
- Modify: 目标 worktree 文件

**Step 1: 编写脚本（Python inline）**

功能：
- 扫描 7 个 worktree
- 跳过 `node_modules/`、`target/`、`dist/`、`.git/`、`.tmp*`、`.esbuild-bin/`
- 对源码文件插入头部注释（shebang 情况插第二行）
- 对 JSON：对象插入 `__legacy_notice`，数组/非对象创建 `.legacy.notice.md`
- 若文件无法 UTF-8 解析，视为不可编辑并跳过

**Step 2: Dry-run 模式输出统计**

Run: 打印候选文件数、跳过数、将修改的文件数
Expected: 输出统计，不写入

---

### Task 3: 执行批量写入

**Files:**
- Modify: 指定 worktree 下的代码/JSON 文件

**Step 1: 正式执行写入**

Run: 同脚本但 `apply=true`
Expected: 头部注释与 JSON 标注写入完成

**Step 2: 抽样检查 3-5 个文件**

Run: `Get-Content` 查看头部说明是否正确
Expected: 说明位于头部，内容完整

---

### Task 4: 记录日志

**Files:**
- Modify: `.trae/CHANGELOG_TASK.md`

**Step 1: 记录本次变更范围与风险**

- 写入已处理 worktree 列表
- 标注 JSON 处理策略与跳过策略
- 记录潜在风险（大规模改动）

---

### Task 5: 建议验证

**Files:**
- Verify: 目标 worktree

**Step 1: 轻量校验**

Run: 抽样 grep/rg 查找 `Legacy Worktree Notice`
Expected: 目标文件能查到说明

**Step 2: JSON 结构校验（抽样）**

Run: `python -c "import json; json.load(open('file.json','r',encoding='utf-8'))"`
Expected: JSON 可解析
