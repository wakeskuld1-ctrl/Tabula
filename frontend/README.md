# Frontend UI

This is the React frontend for the Federated Query Engine.

## Setup

1. Install dependencies:
   ```bash
   npm install
   ```

2. Run development server:
   ```bash
   npm run dev
   ```

## Architecture
- Framework: React + Vite
- Grid Component: Luckysheet / FortuneSheet (Proposed)
- Backend API: Connects to Rust server at http://localhost:3000

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
