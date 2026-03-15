# Legacy Worktree Notice Design

**Date:** 2026-03-15

## 背景
为避免后续 AI 或协作者在历史 worktree 中误改、误合并，需要在这些 worktree 的每个代码文件头部写入“遗留项说明”。说明必须明确：这些 worktree 属于历史保留版本，担心影响而保留，不需要更新操作。

## 目标
- 在指定 worktree 内的**每个可编辑代码文件头部**写入统一的遗留项说明
- 对 JSON 文件进行标注：对象 JSON 写入 `__legacy_notice`；数组/非对象 JSON 不改原文件，仅生成同目录 `filename.legacy.notice.md`
- 保证说明内容包含时间、背景、原因、禁改指令，且位于文件头部

## 范围
**目标 worktree：**
- D:\Rust\metadata\.worktrees\cache-manager-optimization
- D:\Rust\metadata\.worktrees\formula-docs-plan
- D:\Rust\metadata\.worktrees\frontend-test-sessions
- D:\Rust\metadata\.worktrees\readjsonorthrow-unify
- D:\Rust\metadata\.worktrees\split-spill-frontend
- D:\Rust\metadata\frontend\.worktrees\formula-failure-retest
- D:\Rust\metadata\frontend\.worktrees\split-spill

**覆盖文件类型：**
- 代码/脚本/配置（可注释）：`.rs/.ts/.tsx/.js/.jsx/.css/.scss/.html/.ps1/.py/.sh/.go/.java/.cs/.cpp/.c/.h/.sql/.toml/.yml/.yaml`
- JSON：`.json`（含 lock 文件）

**排除：**
- 二进制与不可直接编辑文件（如 `.zip/.png/.wasm/.exe/.dll` 等）
- 生成目录（如 `node_modules/`、`target/`、`dist/`、`.git/`、`.tmp*`、`.esbuild-bin/`）

## 注释规范
- 必须位于文件头部（若有 shebang，则插在第二行）
- 统一“Legacy Worktree Notice”结构，包含：时间、背景、原因、禁改指令、工作区来源
- 使用文件所属语言的单行注释或块注释形式

## JSON 处理规则
- 顶层为对象：新增 `"__legacy_notice"` 字段（字符串，含 Markdown 格式说明）
- 顶层为数组/非对象：不改原文件，生成 `filename.legacy.notice.md`（同目录）
- 若文件无法解析为 JSON：视为不可编辑，跳过

## 风险与规避
- 风险：大规模文件变更影响后续 diff 审阅
- 规避：仅新增“头部说明”或顶层 `__legacy_notice`，避免改动业务逻辑

## 验收标准
- 所有目标 worktree 中的代码文件头部存在遗留项说明
- JSON 对象新增 `__legacy_notice`
- 数组 JSON 生成 `filename.legacy.notice.md`
- 不触碰不可编辑文件
