# Vitest 非 Suite 文件过滤设计

## 背景
当前执行 `npx vitest run` 会扫描 `tests/` 与 `scripts/` 目录下的脚本型文件，这些文件并非 vitest 的 `describe/it` 结构，导致报错 "No test suite found"，使整个测试失败。

## 目标
- 让 `npx vitest run` 只执行真正的 vitest 单测文件
- 保留脚本型测试的可单独运行能力（不强行改造成 vitest suite）
- 不引入新的依赖或复杂配置

## 非目标
- 不改动脚本型测试的实现逻辑
- 不在本次处理性能/覆盖率问题

## 方案对比（已选）
- 方案 A：在 vitest 配置里排除 `tests/` 与 `scripts/` 路径，仅包含 `src/**/*.test.ts` / `src/**/*.spec.ts`
  - 优点：改动最小、结果明确、不会影响脚本测试
  - 缺点：脚本测试需要单独命令执行
- 方案 B：给脚本文件补最小 `describe/it`
  - 缺点：污染脚本结构
- 方案 C：拆分多套测试命令
  - 缺点：改动入口较多

## 设计要点
- 将 vitest 的 include/exclude 规则配置到 `vite.config.ts` 中
- 仅包含 `src/**/*.test.ts`、`src/**/*.spec.ts`
- 排除 `tests/**`、`scripts/**` 与 `**/*.test.cjs` 等脚本型文件

## 兼容性与风险
- 生产构建不受影响
- 脚本型测试不会被 vitest 执行，需要保留手工或独立脚本执行流程

## 测试计划
- 先复现 `npx vitest run` 报错
- 更新配置后重新运行 `npx vitest run`，确认通过
