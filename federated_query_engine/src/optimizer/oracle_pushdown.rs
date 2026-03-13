use std::sync::Arc;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Result, TableReference};
use datafusion::datasource::DefaultTableSource;
use datafusion::logical_expr::{LogicalPlan, TableScan};
use datafusion::optimizer::optimizer::OptimizerRule;
use datafusion::optimizer::OptimizerConfig;
use datafusion::sql::unparser::plan_to_sql;
use crate::datasources::oracle::{OracleDataSource, OracleTable};

#[derive(Debug)]
pub struct OraclePushDown {}

impl OraclePushDown {
    pub fn new() -> Self {
        Self {}
    }

    fn get_oracle_source<'a>(&self, scan: &'a TableScan) -> Option<&'a OracleDataSource> {
        let provider = scan.source.as_any().downcast_ref::<OracleTable>()?;
        Some(provider.source())
    }

    /// 检查计划子树是否可以下推到 Oracle
    ///
    /// **实现方案**:
    /// 递归遍历计划节点，检查是否所有节点都支持下推。
    ///
    /// **支持的节点**:
    /// - `TableScan`: 必须是 Oracle 源，且所有 Scan 必须指向同一个 Oracle 实例 (host:port:service)。
    /// - `Join`, `Filter`, `Projection`, `Limit`, `SubqueryAlias`, `Sort`: 递归检查其子节点。
    ///
    /// **关键问题点**:
    /// - 同源约束：不支持跨源下推，所有表必须在同一个 Oracle 实例中。
    fn can_pushdown(&self, plan: &LogicalPlan, current_source: &mut Option<String>) -> bool {
        match plan {
            LogicalPlan::TableScan(scan) => {
                if let Some(source) = self.get_oracle_source(scan) {
                    let source_id = format!("{}:{}:{}", source.host, source.port, source.service);
                    if let Some(s) = current_source {
                        if s != &source_id {
                            return false;
                        }
                    } else {
                        *current_source = Some(source_id);
                    }
                    true
                } else {
                    false
                }
            }
            LogicalPlan::Join(join) => {
                self.can_pushdown(&join.left, current_source) && self.can_pushdown(&join.right, current_source)
            }
            LogicalPlan::Filter(filter) => {
                self.can_pushdown(&filter.input, current_source)
            }
            LogicalPlan::Projection(proj) => {
                self.can_pushdown(&proj.input, current_source)
            }
            LogicalPlan::Limit(limit) => {
                self.can_pushdown(&limit.input, current_source)
            }
            LogicalPlan::SubqueryAlias(alias) => {
                self.can_pushdown(&alias.input, current_source)
            }
            LogicalPlan::Sort(sort) => {
                self.can_pushdown(&sort.input, current_source)
            }
            _ => false, // Unsupported node type for pushdown
        }
    }

    /// 执行下推转换
    ///
    /// **实现方案**:
    /// 1. 检查计划是否满足 `can_pushdown` 条件。
    /// 2. 如果满足，首先将计划中的 `TableScan` 替换为使用物理表名（而非 DataFusion 中的逻辑表名）。
    /// 3. 使用 `plan_to_sql` 将整个子计划转换为 SQL 字符串。
    /// 4. 清理生成的 SQL（移除方言不兼容部分）。
    /// 5. 创建一个新的 `OracleDataSource`，其 `sql_table` 为生成的 SQL 子查询。
    /// 6. 返回一个新的 `TableScan` 节点，指向这个新创建的源，从而替代原始的复杂计划树。
    ///
    /// **关键问题点**:
    /// - Schema 一致性：新生成的 TableScan 必须输出与原计划相同的 Schema。
    /// - 性能：通过将计算（Join, Filter, Agg）下推到 Oracle，大幅减少网络传输数据量。
    fn pushdown(&self, plan: &LogicalPlan) -> Result<Option<LogicalPlan>> {
        let mut source_id = None;
        if !self.can_pushdown(plan, &mut source_id) {
            return Ok(None);
        }

        // If it's just a TableScan, no need to pushdown (already pushed)
        if let LogicalPlan::TableScan(_) = plan {
            return Ok(None);
        }

        // Transform the plan to use physical table names (sql_table) instead of logical names
        // We must clone the plan first because we need to modify it.
        // LogicalPlan is immutable, so we use transform to create a modified copy.
        let modified_plan = plan.clone().transform_down(&|node| {
            if let LogicalPlan::TableScan(scan) = &node {
                if let Some(source) = self.get_oracle_source(scan) {
                    let mut new_scan = scan.clone();
                    // Use the physical table name from OracleDataSource
                    new_scan.table_name = TableReference::from(source.sql_table.clone());
                    return Ok(Transformed::yes(LogicalPlan::TableScan(new_scan)));
                }
            }
            Ok(Transformed::no(node))
        })?.data;

        // Generate SQL from the modified plan
        let statement = plan_to_sql(&modified_plan)?;
        let sql = statement.to_string();
        
        // Use clean_oracle_sql to fix any dialect issues (e.g. removing AS)
        let clean_sql = OracleDataSource::clean_oracle_sql(&sql);

        crate::app_log!("Generated Pushdown SQL: {}", clean_sql);

        // Find the source to clone config from (using original plan to get source)
        let mut source_config = None;
        plan.apply(&mut |node: &LogicalPlan| {
            if let LogicalPlan::TableScan(scan) = node {
                if let Some(source) = self.get_oracle_source(scan) {
                    source_config = Some(source.clone());
                    return Ok(TreeNodeRecursion::Stop);
                }
            }
            Ok(TreeNodeRecursion::Continue)
        })?;

        if let Some(source) = source_config {
             // Create new OracleDataSource with the generated SQL as a subquery
            let wrapped_sql = format!("({}) pushed_down", clean_sql);
            
            // We need to create a new OracleDataSource
            let new_source = OracleDataSource::new(
                "pushed_down_query".to_string(),
                source.user.clone(),
                source.pass.clone(),
                source.host.clone(),
                source.port,
                source.service.clone(),
                wrapped_sql,
            )?;

            // Create new TableScan
            // We reuse the schema from the original plan because it represents the output of the pushdown query
            // However, OracleDataSource::new might infer a slightly different schema (e.g. types)
            // But DataFusion expects the schema to match what the plan produces.
            // If we replace the plan with a TableScan, the TableScan must have the SAME schema as the plan.
            
            let provider = OracleTable::new(
                new_source,
                plan.schema().inner().clone(), 
                None, // No stats
            );
            
            let table_source = Arc::new(DefaultTableSource::new(Arc::new(provider)));

            let new_scan = LogicalPlan::TableScan(TableScan {
                table_name: "pushed_down".into(),
                source: table_source,
                projection: None, // Select all from the subquery
                projected_schema: plan.schema().clone(),
                filters: vec![],
                fetch: None,
            });

            Ok(Some(new_scan))
        } else {
            Ok(None)
        }
    }
}

impl OptimizerRule for OraclePushDown {
    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        plan.transform_down(&|plan| {
            match self.pushdown(&plan)? {
                Some(new_plan) => Ok(Transformed::yes(new_plan)),
                None => Ok(Transformed::no(plan)),
            }
        }).map(|t| t)
    }

    fn name(&self) -> &str {
        "oracle_pushdown"
    }
}
