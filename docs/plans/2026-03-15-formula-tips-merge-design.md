# Formula Tips Worktree Merge Design

## 目标
将 D:\Rust\metadata\.worktrees\formula-tips 的全部变更（文档/功能/测试/脚本/配置）有序收口并合并到主仓库 D:\Rust\metadata，避免遗漏并便于审阅与回滚。

## 范围
- 文档：docs/plans/*、frontend/docs/plans/*
- 功能实现：frontend/src/**
- 测试与脚本：frontend/src/utils/__tests__/*、frontend/scripts/*
- 配置：frontend/tsconfig.json、frontend/vite.config.ts

## 方案与权衡
### 方案A（推荐）
- 分 4 组提交：文档 -> 功能 -> 测试/脚本 -> 配置
- 优点：提交清晰、审阅友好、回滚粒度小
- 缺点：提交数量较多

### 方案B
- 按目录一次性提交
- 优点：速度快
- 缺点：审阅困难、回滚风险大

### 方案C
- 只合并核心功能，文档/脚本后置
- 优点：影响面小
- 缺点：规范与实现脱节，后续追溯困难

## 冲突处理原则
- 发生冲突时优先采用新版本内容
- 旧版本不清理，保留痕迹便于复盘

## 验证策略
- 最小验证：npx vitest run（覆盖新增/相关测试）
- 如需完整验证：npm run build

## 输出
- 4 个分组提交落入主仓库
- task-journal 记录本次收口
