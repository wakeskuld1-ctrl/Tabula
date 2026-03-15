# Formula Tips (Always-On) Design

**Date:** 2026-03-15

## 背景
用户要求在公式栏下方常驻显示公式 tips（持续可见），降低上手成本，不依赖输入触发。

## 目标
- 公式栏下方常驻显示 tips
- 支持输入过滤（有输入时过滤，无输入显示默认 Top N）
- 数据来源为 `frontend/src/data/formula_help.json`

## 范围
- 前端 UI：`frontend/src/components/layout/FormulaBar.tsx`
- 工具函数：`frontend/src/utils/formulaHelp.ts`
- 样式：`frontend/src/App.css`
- 测试：`frontend/src/utils/__tests__/formulaHelp.test.ts`

## 设计
### UI 结构
- 公式栏（原有）保持一行布局，新增一行“常驻 tips 面板”
- tips 面板包含：标题 + 列表区（可滚动）
- 列表项展示：函数名、语法、示例、用途/参数说明（中英）

### 数据与过滤
- 读取 `formula_help.json`
- 输入为空：显示 Top N
- 输入非空：使用 `filterFormulaHelpItems` 过滤
- 若无匹配：显示空态提示

### 说明文本
- 使用已有 `APP_LABELS.formulaHelp.*` 文案

## 风险
- `formula_help.json` 内可能存在历史乱码，展示时可能不够美观
- 常驻列表可能带来信息密度偏高，需通过滚动区域控制高度

## 验收
- 公式栏下方常驻显示 tips
- 输入内容时可过滤
- 空态提示正确
- 样式不遮挡主表格
