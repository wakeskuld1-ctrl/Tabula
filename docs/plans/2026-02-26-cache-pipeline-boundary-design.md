# 缓存管线与模块边界重构 Design

**目标**
在不改变功能行为的前提下，明确 DataSource / Cache / Metadata 的边界，收敛重复的缓存与单飞行逻辑，并为后续抽象收敛（方案B）铺路。

**背景与问题**
- DataSource（sqlite/oracle）内部包含 L1/L2/sidecar/singleflight 的大量重复逻辑，边界混杂。
- 缓存与数据源耦合导致后续改造成本高，测试难度大。
- Metadata 写入与 DataSource 注册流程分散在多处，职责不清晰。

**范围**
- 引入 CachePipeline 作为缓存与单飞行的统一入口。
- DataSource 只保留“如何读取数据”的最小职责。
- Metadata 写入统一由门面或统一入口完成（保持现有 metadata_manager 结构）。

**非目标**
- 不引入复杂泛型或宏级抽象。
- 不改变已有缓存策略语义或行为（L1/L2/singleflight/sidecar）。
- 不在本次设计中启用 Oracle feature gate（仅为后续留口）。

**分层与依赖**
- DataSource 层：负责构建 TableProvider / Exec，提供数据读取策略。
- CachePipeline 层：统一实现缓存命中/回退、singleflight、sidecar 写入与 L2 填充。
- Metadata 层：统一写入入口（沿用 MetadataManager 或薄门面）。

**核心抽象**
- CachePipelineInput：缓存键、schema、where_clause、batch_size、volatility_policy、flight_guard 等必要输入。
- CachePipeline::run：接收 DataReader（读取数据的闭包/trait）并返回统一的 SendableRecordBatchStream。
- DataReader：DataSource 提供的最小读取能力接口（例如 `read_fn`）。

**数据流**
1. DataSource::register 构建 TableProvider。
2. Exec::execute 组装 CachePipelineInput。
3. CachePipeline::run：
   - L2 命中 → 直接返回
   - L1 命中 → 读取 parquet 返回
   - 未命中 → singleflight（leader/follower）
   - leader 触发 sidecar 写入 L1/L2
   - follower 等待完成后从 L2/L1 重试

**风险与缓解**
- 风险：抽取逻辑后接口参数过长  
  缓解：使用 CachePipelineInput 聚合参数。
- 风险：不同数据源读取差异难以统一  
  缓解：仅抽取缓存与单飞行控制，读取逻辑仍由数据源实现。

**测试策略**
- 为 CachePipeline 的 L1/L2/singleflight 行为增加单元测试。
- sqlite/oracle 执行路径保持现有测试；必要时添加最小回归用例。
- 继续使用 cargo fmt / cargo check / cargo clippy -D warnings 作为门禁。
