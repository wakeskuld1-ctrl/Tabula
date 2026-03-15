# 单元格格式化自动化测试补全 Implementation Plan
 
> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
 
**Goal:** 增强单元格格式化的自动化测试覆盖（非法 format、范围更新性能、display 与原值分离）。
 
**Architecture:** 前端复用 verify_state_integration.cjs 增补 3 组测试；后端在 session_manager tests 增补 3 个 Rust 单测验证 format 写入与原值保持。
 
**Tech Stack:** Puppeteer (Node), Rust (tokio tests)
 
---
 
### Task 1: 前端脚本新增三组格式化验证
 
**Files:**
- Modify: `d:\Rust\metadata\frontend\src\scripts\verify_state_integration.cjs`
 
**Step 1: Write the failing test**
 
```js
// T4: 非法 format 不崩且回退显示原值
// T5: 大范围 format 更新完成并在阈值内
// T6: display 格式化不影响公式栏/原值
```
 
**Step 2: Run test to verify it fails**
 
Run: `node frontend/src/scripts/verify_state_integration.cjs`
Expected: FAIL（新增断言未满足或缺少辅助函数）
 
**Step 3: Write minimal implementation**
 
```js
// 新增 updateSelectionStyle 调用、耗时统计与断言
// 新增从公式栏读取原值的断言
```
 
**Step 4: Run test to verify it passes**
 
Run: `node frontend/src/scripts/verify_state_integration.cjs`
Expected: PASS
 
**Step 5: Commit**
 
```bash
git add frontend/src/scripts/verify_state_integration.cjs
git commit -m "test: add format validation scenarios"
```
 
仅在用户明确要求时执行提交。
 
---
 
### Task 2: 后端补齐格式化相关单测
 
**Files:**
- Modify: `d:\Rust\metadata\federated_query_engine\src\session_manager\mod.rs`
 
**Step 1: Write the failing test**
 
```rust
#[tokio::test]
async fn test_update_style_range_applies_format() {
    // 验证范围更新后多个 key 的 format 生效
}
 
#[tokio::test]
async fn test_invalid_format_does_not_change_cell_value() {
    // 写入原值 + 设置未知 format，验证原值不变
}
 
#[tokio::test]
async fn test_format_does_not_mutate_data_batches() {
    // 更新 format 后读取 data batch，断言原始值未变
}
```
 
**Step 2: Run test to verify it fails**
 
Run: `cargo test -p federated_query_engine test_update_style_range_applies_format`
Expected: FAIL（测试未实现或断言不成立）
 
**Step 3: Write minimal implementation**
 
```rust
// 在 tests 模块中补齐测试用例，复用现有 SessionManager 初始化逻辑
```
 
**Step 4: Run test to verify it passes**
 
Run: `cargo test -p federated_query_engine test_update_style_range_applies_format`
Expected: PASS
 
**Step 5: Commit**
 
```bash
git add federated_query_engine/src/session_manager/mod.rs
git commit -m "test: cover format range and display-only behaviors"
```
 
仅在用户明确要求时执行提交。
 
---
 
### Task 3: 全量检查与回归
 
**Files:**
- N/A
 
**Step 1: Run frontend script**
 
Run: `node frontend/src/scripts/verify_state_integration.cjs`
Expected: PASS
 
**Step 2: Run backend tests**
 
Run: `cargo test -p federated_query_engine test_update_style_range_applies_format`
Expected: PASS
 
**Step 3: Run lint/typecheck**
 
Run: `cargo fmt --all -- --check`
Expected: PASS（如失败，记录差异）
 
Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS（如失败，记录现有告警）
 
Run: `cargo check`
Expected: PASS
 
**Step 4: Commit**
 
```bash
git add .
git commit -m "test: complete format validation coverage"
```
 
仅在用户明确要求时执行提交。
