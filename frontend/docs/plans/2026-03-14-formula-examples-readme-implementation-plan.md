# 公式样例 README Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在 README 中补齐已支持公式样例（中英双语），按类型分组。

**Architecture:** 仅修改 README 文档，新增“公式样例 / Formula Examples”章节。

**Tech Stack:** Markdown

---

### Task 1: 更新 README 公式样例

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/README.md`

**Step 1: Add Formula Examples section**

```markdown
## 公式样例 / Formula Examples

### 聚合类 / Aggregates
- `=SUM(A:A)` — 对 A 列求和 / Sum values in column A
- `=COUNT(A:A)` — 统计 A 列非空数量 / Count non-empty values in column A
- `=AVG(A:A)` — 计算 A 列平均值 / Average values in column A
- `=MAX(A:A)` — A 列最大值 / Max value in column A
- `=MIN(A:A)` — A 列最小值 / Min value in column A

### 查找类 / Lookup
- `=XLOOKUP(A2,"orders","order_id","amount",0)` — 从 orders 表按 order_id 查 amount / Lookup amount by order_id
- `=VLOOKUP(A2,"orders","amount","order_id")` — 与 XLOOKUP 等价的简写 / Equivalent lookup for amount

### 算术类 / Arithmetic
- `=A1+B1` — A1 与 B1 相加 / Add A1 and B1
- `=A1-B1` — A1 与 B1 相减 / Subtract B1 from A1
- `=A1*B1` — A1 与 B1 相乘 / Multiply A1 and B1
- `=A1/B1` — A1 除以 B1 / Divide A1 by B1
- `=(A1+B1)/C1` — 组合运算示例 / Combined arithmetic example
```

**Step 2: Manual check**
- 确认 README 样例覆盖已支持公式
- 确认中英说明准确

---

### Task 2: 更新任务日志

**Files:**
- Modify: `D:/Rust/metadata/.worktrees/readjsonorthrow-unify/frontend/.trae/CHANGELOG_TASK.md`

**Step 1: Append entry**

```markdown
## 2026-03-14
### 修改内容
- README 增加公式样例（中英双语），按类型分组
### 修改原因
- 补齐已有支持公式的使用示例，降低上手成本
### 方案还差什么?
- [ ] 是否需要同步到 UI 的样例面板
### 潜在问题
- [ ] 样例与实际支持函数可能存在偏差
### 关闭项?
- 公式样例已补齐
```

---

### Task 3: Commit

```bash
git add README.md .trae/CHANGELOG_TASK.md docs/plans/2026-03-14-formula-examples-readme-implementation-plan.md
git commit -m "docs: add formula examples to readme"
```
