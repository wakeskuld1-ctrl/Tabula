# Backend Stress Test Report
**Date:** 2026-02-03
**Version:** Fix-Race-And-Perf-v3 (10MB Stress Re-test)

## 1. Summary
对 10MB 压力测试进行复测，确认性能有明显好转。

## 2. Test Results (reproduce_issue.cjs)

| Test Case | Result | Status | Notes |
| :--- | :--- | :--- | :--- |
| **1. Concurrency Storm** | **20/20 Success** | ✅ PASS | All 20 parallel requests succeeded. Previously caused session explosion or failures. |
| **2. Race Condition** | **Serialized** | ✅ PASS | Concurrent requests on the same cell/session were correctly serialized. Both requests modified the *same* session ID (`eb75...`). |
| **3. Formula Chain** | **Consistent** | ✅ PASS | Sequential updates preserved the session lineage correctly. |
| **4. Payload Stress** | **Success** | ✅ PASS | 10MB payload update succeeded. **Duration: 116,393ms (~1.94 min)**. |

## 3. 复测结论
1. **性能好转**：10MB 写入从 235,076ms 降至 116,393ms（约 50.5% 改善）。
2. **竞态行为稳定**：同一基准会话并发更新被序列化到同一会话 ID，最终值为后写入值。
3. **读回验证正常**：10MB 读回 252ms，内容长度正确。

## 4. 备注
本次仅复测性能与稳定性，未引入新的功能变更。
