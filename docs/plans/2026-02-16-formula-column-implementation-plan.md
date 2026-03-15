# 公式列功能 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现可插入、只读、可持久化的公式列，并在后端 SQL 查询中计算公式结果。

**Architecture:** 公式列以 marker 形式持久化到元数据；后端查询拼接计算列；前端通过“插入公式列”弹窗输入列名+公式，设置只读并在公式栏展示 raw。

**Tech Stack:** Rust (Axum, DataFusion), React + Glide Data Grid

---

### Task 1: 后端接口与查询输出补齐

**Files:**
- Modify: `d:\Rust\metadata\federated_query_engine\src\api\grid_handler.rs`
- Modify: `d:\Rust\metadata\federated_query_engine\src\main.rs`
- Test: `d:\Rust\metadata\federated_query_engine\src\api\grid_handler.rs` (新增测试模块)

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_grid_data_includes_formula_columns() {
    // 断言 JSON 返回包含 formula_columns 字段
    // 预期字段不存在时测试失败
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p federated_query_engine grid_handler::tests::test_grid_data_includes_formula_columns`
Expected: FAIL（缺少 formula_columns 字段）

**Step 3: Write minimal implementation**

```rust
Ok(result) => Json(serde_json::json!({
    "status": "ok",
    "data": result.rows,
    "columns": result.columns,
    "column_types": result.column_types,
    "total_rows": result.total_rows,
    "metadata": result.metadata,
    "formula_columns": result.formula_columns
}))
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p federated_query_engine grid_handler::tests::test_grid_data_includes_formula_columns`
Expected: PASS

**Step 5: Commit**

```bash
git add federated_query_engine/src/api/grid_handler.rs
git commit -m "feat: include formula_columns in grid-data response"
```

仅在用户明确要求时执行提交。

---

### Task 2: 前端插入公式列与只读渲染

**Files:**
- Modify: `d:\Rust\metadata\frontend\src\components\GlideGrid.tsx`
- Modify: `d:\Rust\metadata\frontend\src\App.tsx`
- Test: `d:\Rust\metadata\frontend\src\scripts\verify_formula_bar_features.cjs` (可选补充)

**Step 1: Write the failing test**

```js
// 在验证脚本中断言插入公式列后该列不可编辑
// 在验证脚本中断言公式列被选中时公式栏显示 raw
// 在验证脚本中断言插入公式列时必须填写列名
```

**Step 2: Run test to verify it fails**

Run: `node frontend/src/scripts/verify_formula_bar_features.cjs`
Expected: FAIL（公式列仍可编辑）

**Step 3: Write minimal implementation**

```ts
// 读取 formula_columns 结果，建立只读列集合
// 插入公式列时弹窗输入列名与 raw 表达式
// buildFormulaColumnMarker(raw, columnIds) 生成 marker 并写入 default_formula
```

**Step 4: Run test to verify it passes**

Run: `node frontend/src/scripts/verify_formula_bar_features.cjs`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/components/GlideGrid.tsx frontend/src/App.tsx
git commit -m "feat: add insert formula column UI and readonly rendering"
```

仅在用户明确要求时执行提交。

---

### Task 3: 验证与全量校验

**Files:**
- Test: `d:\Rust\metadata\federated_query_engine`
- Test: `d:\Rust\metadata\frontend`

**Step 1: Run backend tests**

Run: `cargo test -p federated_query_engine`
Expected: PASS

**Step 2: Run lint / fmt / typecheck**

Run: `cargo fmt --all -- --check`
Expected: PASS

Run: `cargo clippy --all-targets --all-features -D warnings`
Expected: PASS

Run: `cargo check`
Expected: PASS

**Step 3: Frontend verification (manual or script)**

Run: `node frontend/src/scripts/verify_formula_bar_features.cjs`
Expected: PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "test: verify formula column feature"
```

仅在用户明确要求时执行提交。
