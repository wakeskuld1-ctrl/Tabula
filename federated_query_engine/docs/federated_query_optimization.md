# 联邦查询引擎优化策略文档

## 1. 概述
本文档记录了联邦查询引擎的优化策略演进，包括当前的 "Aggressive Pushdown"（激进下推）策略以及未来的 Cost-Based Optimization (CBO) 规划。

## 2. 核心 Cost 计算公式 (CBO 模型)

我们需要比较两种执行路径的代价：**Cost_Local**（拉取数据到本地计算） vs **Cost_Pushdown**（下推 SQL 到远端计算）。

### 2.1 本地执行代价 (Cost_Local)
这种情况是把整张表（或仅做简单列裁剪）从数据库拉到 DataFusion 本地，然后由 DataFusion 进行 Join、Filter、Agg。

$$ Cost_{Local} = \underbrace{(N_{total} \times W_{row} \times K_{net})}_{\text{网络传输代价}} + \underbrace{(N_{total} \times K_{cpu\_local})}_{\text{本地反序列化与计算代价}} $$

*   $N_{total}$: **原始表行数** (Source Table Row Count)。
*   $W_{row}$: **平均行宽** (Average Row Width, bytes)。
*   $K_{net}$: **网络传输系数**（单位：Cost/Byte）。假设内网千兆/万兆，该值较小；公网则很大。
*   $K_{cpu\_local}$: **本地 CPU 处理系数**（单位：Cost/Row）。DataFusion 处理每行数据的开销（反序列化、内存分配、计算）。

### 2.2 下推执行代价 (Cost_Pushdown)
这种情况是将 SQL 发送给 Oracle/YashanDB 执行，我们只拉取最终结果集。

$$ Cost_{Pushdown} = \underbrace{(N_{result} \times W_{row} \times K_{net})}_{\text{结果集传输代价}} + \underbrace{C_{remote\_exec}}_{\text{远端执行代价}} $$

*   $N_{result}$: **预估结果集行数** (Estimated Result Rows)。这是 CBO 的核心难点（即 Selectivity 选择率估算）。
    *   通常 $N_{result} \ll N_{total}$（结果集远小于原表）。
*   $C_{remote\_exec}$: **远端数据库执行代价**。
    *   通常假设远端数据库（Oracle/Yashan）有索引且性能强劲，且不消耗我们本地资源，因此在我们的 Cost 模型中，这个值通常设得很低。

### 2.3 决策不等式

$$ \text{如果 } Cost_{Pushdown} < Cost_{Local} \text{，则选择 下推 (Pushdown)。} $$

## 3. 当前策略：Aggressive Pushdown (Rule-Based)

### 3.1 背景
由于目前系统尚未实现完整的统计信息收集（如 `NUM_ROWS`, `AVG_ROW_LEN`），无法精确计算 Cost。

### 3.2 策略逻辑
采用 **Rule-Based（基于规则）** 的激进下推策略：
1.  分析查询计划中的所有表扫描节点。
2.  **同源检测**：如果查询涉及的所有表都来自同一个数据源（例如都是 Oracle 连接 A 的表）。
3.  **强制下推**：直接将整个 SQL 查询重写并下推至该数据源执行。
4.  **例外处理**：如果涉及跨源 Join（如 Oracle Join YashanDB），则回退到 DataFusion 本地执行（Local Join）。

### 3.3 优势
*   **网络 I/O 最小化**：对于单源复杂查询（Join/Agg），避免了大量原始数据传输。
*   **利用远端算力**：利用 Oracle/YashanDB 成熟的优化器和索引。
*   **规避 Schema 问题**：绕过了 DataFusion 本地推断 Schema 可能导致的类型/大小写不匹配问题（如 TPCC `w_ytd` vs `w_name` 错误）。

## 4. 统计信息与 CBO 进展 (Phase 2 Status - 2026.01.30 Updated)

### 4.1 已实现功能
1.  **统计信息收集 (Implemented)**:
    *   **Oracle**: 已实现基于 `EXPLAIN PLAN` 的统计信息获取。不依赖 `ALL_TABLES`（可能不准），而是动态生成执行计划并查询 `PLAN_TABLE` 获取优化器估算的 `CARDINALITY` 和 `BYTES`。
    *   **YashanDB**: 同样采用 `EXPLAIN PLAN` 机制获取 `CARDINALITY`。
    *   这些统计信息已通过 `TableProvider::statistics()` 暴露给 DataFusion。

2.  **源选择优化 (Source Selection)**:
    *   实现了基于 `NUM_ROWS` 的源选择逻辑。在多源存在同名表（如 Oracle 和 YashanDB 都有 `TPCC` 表）时，优先选择行数较少或有统计信息的源进行下推，或者基于配置的路由规则。

### 4.2 下一步规划
1.  **混合 Join 优化**:
    *   目前仅实现了 "单源全下推"。对于 "Oracle Join YashanDB" 的场景，尚未完全利用统计信息进行 Join 顺序调整（Join Reordering）。
2.  **代价公式细化**:
    *   引入网络传输代价参数，区分内网/公网数据源。
