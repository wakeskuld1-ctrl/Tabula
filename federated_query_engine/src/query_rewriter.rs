use datafusion::error::Result;
use datafusion::prelude::SessionContext;
use datafusion::sql::parser::{DFParser, Statement as DFStatement};
use datafusion::sql::sqlparser::ast::{
    Expr, Ident, ObjectName, ObjectNamePart, SetExpr, Statement, TableFactor, Value,
};
use datafusion::sql::sqlparser::tokenizer::Span;
use std::collections::{HashMap, HashSet};

use crate::metadata_manager::MetadataManager;
use metadata_store::TableMetadata;
use regex::Regex;
use std::sync::RwLock;

// Global Routing Configuration
lazy_static::lazy_static! {
    static ref ROUTING_TABLE: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
    static ref TABLE_ALIAS_AS_REGEX: Regex = Regex::new(r"(?i)\b(FROM|JOIN)\s+([a-zA-Z0-9_.\x22]+)\s+(AS)\s+([a-zA-Z0-9_]+)").unwrap();
}

/// Service for normalizing table identifiers to ensure consistent matching
/// regardless of quoting, casing, or schema prefixes.
pub struct IdentifierNormalizer;

impl IdentifierNormalizer {
    /// Normalizes an identifier by stripping quotes and schema prefixes.
    /// e.g. "schema"."table" -> table
    ///      "table" -> table
    ///      table -> table
    pub fn normalize(ident: &str) -> String {
        // 1. Handle compound identifiers (split by dot)
        // We only care about the final table name for matching in this federated context
        let parts: Vec<&str> = ident.split('.').collect();
        let last_part = parts.last().unwrap_or(&ident);

        // 2. Strip quotes
        last_part.trim_matches('"').trim_matches('\'').to_string()
    }

    /// Parses an identifier into (Option<Schema>, Table)
    /// Removes quotes from both parts.
    pub fn parse(ident: &str) -> (Option<String>, String) {
        let parts: Vec<&str> = ident.split('.').collect();
        match parts.len() {
            3 => {
                // catalog.schema.table -> (Some(schema), table)
                // We ignore the catalog (parts[0]) for matching purposes
                let schema = parts[1].trim_matches('"').trim_matches('\'').to_string();
                let table = parts[2].trim_matches('"').trim_matches('\'').to_string();
                (Some(schema), table)
            }
            2 => {
                // schema.table -> (Some(schema), table)
                let schema = parts[0].trim_matches('"').trim_matches('\'').to_string();
                let table = parts[1].trim_matches('"').trim_matches('\'').to_string();
                (Some(schema), table)
            }
            _ => {
                // table or other cases -> (None, table)
                let table = ident.trim_matches('"').trim_matches('\'').to_string();
                (None, table)
            }
        }
    }
}

const CHANGE_NOTES: &[&str] = &[
    "变更备注 2026-02-28: 移除未使用的路由规则删除函数，原因是清理clippy告警并避免误用",
    "变更备注 2026-02-28: 补充LIMIT重写行为测试，原因是回归确认仍保留LIMIT语法",
    "变更备注 2026-02-28: 修复CBO路由映射缺少Schema问题，原因是DataFusion无法在默认Schema中找到非public表，导致TPCC查询失败",
];

pub fn set_routing_rule(table: String, target: String) {
    let _ = CHANGE_NOTES;
    let mut table_map = ROUTING_TABLE.write().unwrap();
    table_map.insert(table.to_lowercase(), target);
}

pub fn get_routing_rule(table: &str) -> Option<String> {
    let table_map = ROUTING_TABLE.read().unwrap();
    table_map.get(&table.to_lowercase()).cloned()
}

// Cost-Based Source Selection
#[allow(dead_code)]
pub(crate) fn get_best_candidate(cands: &[TableMetadata]) -> Option<&TableMetadata> {
    if cands.is_empty() {
        return None;
    }

    // Sort by Cost (NUM_ROWS) ASC
    // We assume lower rows = better cost for now.
    let mut sorted: Vec<&TableMetadata> = cands.iter().collect();
    sorted.sort_by(|a, b| {
        let cost_a = get_cost(a);
        let cost_b = get_cost(b);
        cost_a.cmp(&cost_b)
    });

    Some(sorted[0])
}

fn get_cost(table: &TableMetadata) -> u64 {
    if let Some(stats) = &table.stats_json {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stats) {
            return json["num_rows"].as_u64().unwrap_or(1000000);
        }
    }
    1000000
}

/// 选择最佳数据源 (CBO 核心逻辑)
///
/// **实现方案**:
/// 1. **候选发现**: 遍历查询中涉及的所有逻辑表，在元数据中查找所有匹配的物理表（候选集）。
/// 2. **贪心覆盖算法**:
///    - 目标：最小化跨源 Join，最大化下推。
///    - 策略：优先选择能覆盖最多未覆盖表的源（Max Co-location）。
///    - 决策：在覆盖数量相同时，选择成本最低（行数最少）的源。
/// 3. **路由映射**: 生成逻辑表名到物理全名（Catalog.Schema.Table）的映射表。
///
/// **调用链路**:
/// - 被 `rewrite_query` 调用。
///
/// **关键问题点**:
/// - 逻辑表匹配：支持 Schema 限定名和简单名匹配。
/// - 跨源 Join：如果一个查询涉及多个源，算法会尝试将表分组到各自的源，DataFusion 将负责执行跨源 Join。
fn select_best_source(
    metadata: &MetadataManager,
    logical_tables: &[String],
) -> Option<HashMap<String, String>> {
    let all_tables = metadata.list_tables().ok()?;

    // 1. Map Logical Table -> List of Candidates (Physical Tables)
    let mut candidates: HashMap<String, Vec<&TableMetadata>> = HashMap::new();

    for logical in logical_tables {
        // Resolve logical name (e.g. "tpcc.warehouse" -> "warehouse")
        let (schema_opt, simple_name) = IdentifierNormalizer::parse(logical);

        // Universal Match (Ignore Schema Prefix)
        // User requested to remove strict schema matching to avoid issues with TPCC vs public schemas.
        // We now match purely on table name, regardless of the schema provided in the query.
        let matches: Vec<&TableMetadata> = all_tables
            .iter()
            .filter(|t| {
                t.sheet_name
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(&simple_name)
                    || t.table_name
                        .ends_with(&format!("_{}", simple_name.to_lowercase()))
                    || t.table_name.eq_ignore_ascii_case(&simple_name)
            })
            .collect();

        if !matches.is_empty() && schema_opt.is_some() {
            crate::logger::log(&format!(
                "CBO Info: Found {} candidates for '{}' (ignoring schema).",
                matches.len(),
                logical
            ));
        }

        if matches.is_empty() {
            crate::logger::log(&format!("CBO Warning: No physical table found for logical table '{}'. Simple name extracted: '{}'. Will attempt aggressive inference.", logical, simple_name));
        }
        candidates.insert(logical.clone(), matches);
    }

    // 2. Greedy Cover Algorithm (Improved for Conflict Resolution)
    // Goal: Resolve ambiguous tables (multi-source) by Cost & Co-location.
    // Unique tables are implicitly handled by their only source.
    let mut routing_map = HashMap::new();
    let mut uncovered_tables: HashSet<String> = logical_tables.iter().cloned().collect();

    // Identify unique vs ambiguous tables
    let mut unique_tables = HashMap::new();
    let mut ambiguous_tables = HashSet::new();

    for (logical, cands) in &candidates {
        if cands.is_empty() {
            continue;
        }
        if cands.len() == 1 {
            // Only one physical source available
            unique_tables.insert(logical.clone(), cands[0]);
        } else {
            ambiguous_tables.insert(logical.clone());
        }
    }

    // Pre-calculate source capabilities (Source Key -> Set of Logical Tables it contains)
    let mut source_inventory: HashMap<String, HashSet<String>> = HashMap::new();
    let mut source_table_cost: HashMap<String, HashMap<String, u64>> = HashMap::new();

    for (logical, table_candidates) in &candidates {
        for cand in table_candidates {
            let key = format!("{}|{}", cand.source_type, cand.file_path);
            source_inventory
                .entry(key.clone())
                .or_default()
                .insert(logical.clone());

            let rows = get_cost(cand);
            source_table_cost
                .entry(key)
                .or_default()
                .insert(logical.clone(), rows);
        }
    }

    // Step 1: Lock in Unique Tables immediately (They have no choice)
    for (logical, cand) in unique_tables {
        // Construct fully qualified name
        let full_name = if !cand.catalog_name.is_empty() && !cand.schema_name.is_empty() {
            format!("{}.{}.{}", cand.catalog_name, cand.schema_name, cand.table_name)
        } else if !cand.schema_name.is_empty() {
            format!("{}.{}", cand.schema_name, cand.table_name)
        } else {
            cand.table_name.clone()
        };
        routing_map.insert(logical.clone(), full_name.clone());
        let simple_name = IdentifierNormalizer::normalize(&logical);
        routing_map.insert(simple_name, full_name);
        uncovered_tables.remove(&logical);
    }

    // Step 2: Resolve Ambiguous Tables using Greedy Cover (Cost & Co-location)
    loop {
        if uncovered_tables.is_empty() {
            break;
        }

        // Score sources based on UNCOVERED tables
        let mut best_source_key: Option<String> = None;
        let mut best_count = 0;
        let mut best_cost = u64::MAX;

        let mut found_any = false;

        for (key, inventory) in &source_inventory {
            // Count how many uncovered tables this source has
            let matching_tables: Vec<&String> = inventory
                .iter()
                .filter(|t| uncovered_tables.contains(*t))
                .collect();

            let count = matching_tables.len();
            if count == 0 {
                continue;
            }
            found_any = true;

            // Calculate cost for these specific matching tables
            let cost: u64 = matching_tables
                .iter()
                .map(|t| {
                    source_table_cost
                        .get(key)
                        .unwrap()
                        .get(*t)
                        .unwrap_or(&1000000)
                })
                .sum();

            // Selection Logic:
            // 1. Maximize Coverage (Co-location)
            // 2. Minimize Cost (Row Count)
            if count > best_count {
                best_count = count;
                best_cost = cost;
                best_source_key = Some(key.clone());
            } else if count == best_count {
                if cost < best_cost {
                    best_cost = cost;
                    best_source_key = Some(key.clone());
                }
            }
        }

        if !found_any {
            // Remaining tables have no candidates in metadata?
            break;
        }

        // Apply selection
        if let Some(source_key) = best_source_key {
            let (source_type, file_path) = source_key.split_once('|').unwrap();
            let inventory = source_inventory.get(&source_key).unwrap();

            crate::logger::log(&format!(
                "CBO Selected Source Fragment: {} (Covers: {} ambiguous tables, Cost: {})",
                source_type, best_count, best_cost
            ));

            // Add to routing map
            let mut covered_now = Vec::new();
            for logical in &uncovered_tables {
                if inventory.contains(logical) {
                    // Find the specific metadata for this source
                    if let Some(cands) = candidates.get(logical) {
                        // Priority: Match source -> Match Cost -> First one
                        let target_opt = cands
                            .iter()
                            .find(|t| t.source_type == source_type && t.file_path == file_path);
                        
                        if let Some(target) = target_opt {
                             // Construct fully qualified name to ensure DataFusion finds it in the correct schema
                            let full_name = if !target.catalog_name.is_empty() && !target.schema_name.is_empty() {
                                format!("{}.{}.{}", target.catalog_name, target.schema_name, target.table_name)
                            } else if !target.schema_name.is_empty() {
                                format!("{}.{}", target.schema_name, target.table_name)
                            } else {
                                target.table_name.clone()
                            };

                            routing_map.insert(logical.clone(), full_name.clone());

                            // Map simple name too (for user convenience in SQL)
                            let simple_name = IdentifierNormalizer::normalize(logical);
                            routing_map.insert(simple_name, full_name);

                            covered_now.push(logical.clone());
                        }
                    }
                }
            }

            for t in covered_now {
                uncovered_tables.remove(&t);
            }
        } else {
            break;
        }
    }

    Some(routing_map)
}

/// 使用物理表名重写 SQL
///
/// **实现方案**:
/// 1. 使用 `DFParser` 解析 SQL 为 AST。
/// 2. 遍历 AST 中的所有查询语句。
/// 3. 在 `FROM` 和 `JOIN` 子句中，查找并替换表名。
///    - 优先精确匹配。
///    - 其次尝试归一化后匹配（忽略大小写和引号）。
/// 4. 将修改后的 AST 重新序列化为 SQL。
/// 5. 调用 `fix_dialect` 修复方言问题（如 `AS` 关键字）。
///
/// **调用链路**:
/// - 被 `test_quoted_table_rewrite_bug_reproduction` 测试用例调用。
/// - 实际上 `rewrite_query` 内部使用了类似的逻辑 (`rewrite_statement`)，但此函数提供了独立的重写能力。
pub fn rewrite_with_physical_tables(
    sql: &str,
    table_map: &HashMap<String, String>,
) -> Result<String> {
    let mut statements = DFParser::parse_sql(sql)?;

    for statement in &mut statements {
        if let DFStatement::Statement(stmt) = statement {
            if let Statement::Query(query) = &mut **stmt {
                rewrite_query_tables(&mut query.body, table_map);
            }
        }
    }

    let fixed_sql = fix_dialect(
        &statements
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("; "),
    );
    Ok(fixed_sql)
}

// Apply dialect fix to any SQL string
pub fn fix_dialect(sql: &str) -> String {
    TABLE_ALIAS_AS_REGEX
        .replace_all(sql, "$1 $2 $4")
        .to_string()
}

fn rewrite_query_tables(body: &mut SetExpr, table_map: &HashMap<String, String>) {
    if let SetExpr::Select(select) = body {
        for table in &mut select.from {
            replace_table_factor(&mut table.relation, table_map);
            for join in &mut table.joins {
                replace_table_factor(&mut join.relation, table_map);
            }
        }
    }
}

fn replace_table_factor(relation: &mut TableFactor, table_map: &HashMap<String, String>) {
    if let TableFactor::Table { name, .. } = relation {
        let table_name = name.to_string();

        // Priority 1: Exact Match (Handles quoted names correctly)
        // Since table_map keys come from logical_tables (which use name.to_string()),
        // exact match should be the primary lookup method.
        let mut replacement = table_map.get(&table_name);

        // Priority 2: Clean Name Match (Fallback for unquoted simple names or loose matching)
        if replacement.is_none() {
            let normalized = IdentifierNormalizer::normalize(&table_name);
            replacement = table_map.get(&normalized).or_else(|| {
                table_map
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&normalized))
                    .map(|(_, v)| v)
            });
        }

        if let Some(physical_name) = replacement {
            let parts: Vec<&str> = physical_name.split('.').collect();
            let idents: Vec<Ident> = parts
                .iter()
                .map(|p| {
                    let val = p.trim_matches('"');
                    Ident {
                        value: val.to_string(),
                        quote_style: Some('"'),
                        span: Span::empty(),
                    }
                })
                .collect();

            *name = ObjectName(idents.into_iter().map(ObjectNamePart::Identifier).collect());
        }
    }
}

/// 重写查询以使用最佳物理表
///
/// **实现方案**:
/// 1. **解析**: 解析 SQL 提取所有逻辑表名。
/// 2. **CBO**: 调用 `select_best_source` 计算最佳路由映射。
///    - 如果 CBO 失败或未启用，回退到静态路由表。
/// 3. **重写**: 根据路由映射，将 AST 中的逻辑表名替换为物理全名。
/// 4. **生成**: 输出重写后的 SQL。
///
/// **调用链路**:
/// - API 层 (`/api/execute`) 调用，作为查询执行的第一步。
pub async fn rewrite_query(
    ctx: &SessionContext,
    metadata_manager: Option<&MetadataManager>,
    sql: &str,
) -> Result<String> {
    // 1. Parse SQL using DataFusion's parser
    let statements = DFParser::parse_sql(sql)?;

    // 2. Identify all tables in the query first (for CBO)
    let mut logical_tables = HashSet::new();
    for statement in &statements {
        if let DFStatement::Statement(stmt) = statement {
            if let Statement::Query(query) = &**stmt {
                if let SetExpr::Select(select) = &*query.body {
                    for table in &select.from {
                        if let TableFactor::Table { name, .. } = &table.relation {
                            logical_tables.insert(name.to_string());
                        }
                        for join in &table.joins {
                            if let TableFactor::Table { name, .. } = &join.relation {
                                logical_tables.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // 1. Static Base (Guaranteed Availability)
    let mut final_map = HashMap::new();
    let mut static_map = HashMap::new();

    for table in &logical_tables {
        let simple_name = IdentifierNormalizer::normalize(table);
        
        // Try exact match first
        if let Some(target) = get_routing_rule(table) {
            final_map.insert(table.clone(), target.clone());
            static_map.insert(table.clone(), target.clone());
        } 
        // Try normalized match
        else if let Some(target) = get_routing_rule(&simple_name) {
            final_map.insert(table.clone(), target.clone());
            // Also map the simple name to target for convenience
            final_map.entry(simple_name.clone()).or_insert(target.clone());
            static_map.insert(table.clone(), target);
        }
    }

    if !static_map.is_empty() {
        crate::logger::log(&format!("Applying Routing Map (Static Base): {:?}", static_map));
    }

    // 2. CBO Overlay (Optimization & Conflict Resolution)
    if let Some(meta) = metadata_manager {
        let tables_vec: Vec<String> = logical_tables.iter().cloned().collect();
        // CBO now acts as an improver/selector for ambiguous cases
        if let Some(cbo_map) = select_best_source(meta, &tables_vec) {
            if !cbo_map.is_empty() {
                crate::logger::log(&format!("Applying Routing Map (CBO Overlay): {:?}", cbo_map));
                // CBO results overwrite static results (assuming CBO made a better choice based on cost)
                final_map.extend(cbo_map);
            }
        }
    }

    let missing_tables: Vec<String> = logical_tables
        .iter()
        .filter(|t| !final_map.contains_key(*t))
        .cloned()
        .collect();
    if !missing_tables.is_empty() {
        crate::logger::log(&format!(
            "Routing Warning: unresolved logical tables after merge: {:?}",
            missing_tables
        ));
    }
    let final_map = if final_map.is_empty() { None } else { Some(final_map) };

    // Always run AST rewriting (Handles Map replacement AND Fallback logic like schema stripping)
    let mut new_statements = statements;
    for stmt in &mut new_statements {
        rewrite_statement(ctx, stmt, &final_map)?;
    }
    // Return the rewritten SQL (taking the first statement)
    Ok(new_statements[0].to_string())
}

/// 修复查询中的字段引用问题 (Case Sensitivity)
///
/// **实现方案**:
/// 1. 解析 SQL。
/// 2. 遍历 `SELECT` 列表。
/// 3. 如果发现 `unknown_field`，则尝试在当前上下文的所有表中查找该字段（忽略大小写）。
/// 4. 如果找到匹配的字段，将 AST 中的标识符替换为正确的、带引号的字段名。
///
/// **调用链路**:
/// - API 层在捕获到 DataFusion 的 "Schema error: No field named..." 错误时调用，尝试自动修复。
///
/// **关键问题点**:
/// - 字段发现：需要访问 DataFusion `SessionContext` 来获取表的 Schema。
pub async fn fix_query(ctx: &SessionContext, sql: &str, unknown_field: &str) -> Result<String> {
    let mut statements = DFParser::parse_sql(sql)?;
    for statement in &mut statements {
        if let DFStatement::Statement(stmt) = statement {
            if let Statement::Query(query) = &mut **stmt {
                fix_query_body(ctx, &mut query.body, unknown_field).await?;
            }
        }
    }
    Ok(statements
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("; "))
}

fn rewrite_statement(
    _ctx: &SessionContext,
    statement: &mut DFStatement,
    table_map: &Option<HashMap<String, String>>,
) -> Result<()> {
    if let DFStatement::Statement(stmt) = statement {
        if let Statement::Query(query) = &mut **stmt {
            if let Some(map) = table_map {
                rewrite_query_tables(&mut query.body, map);
            }
        }
    }
    Ok(())
}

async fn fix_query_body(
    ctx: &SessionContext,
    body: &mut SetExpr,
    unknown_field: &str,
) -> Result<()> {
    if let SetExpr::Select(select) = body {
        // Collect valid columns to see if unknown_field is actually a column
        let mut valid_columns = HashSet::new();
        for table in &select.from {
            collect_columns(ctx, &table.relation, &mut valid_columns).await?;
            for join in &table.joins {
                collect_columns(ctx, &join.relation, &mut valid_columns).await?;
            }
        }

        // Check if unknown_field matches any valid column case-insensitively
        let matched_column = valid_columns
            .iter()
            .find(|c| c.eq_ignore_ascii_case(unknown_field))
            .cloned();

        if let Some(selection) = &mut select.selection {
            fix_expr(selection, unknown_field, matched_column.as_deref());
        }
    }
    Ok(())
}

async fn collect_columns(
    ctx: &SessionContext,
    relation: &TableFactor,
    columns: &mut HashSet<String>,
) -> Result<bool> {
    if let TableFactor::Table { name, alias: _, .. } = relation {
        // Try original name first
        let table_name = name.to_string();
        let mut provider = ctx.table_provider(&table_name).await.ok();

        // If not found, try unquoted name (if different)
        if provider.is_none() {
            let unquoted = table_name.trim_matches('"').to_string();
            if unquoted != table_name {
                provider = ctx.table_provider(&unquoted).await.ok();
            }
        }

        if let Some(p) = provider {
            let schema = p.schema();
            for field in schema.fields() {
                columns.insert(field.name().clone());
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn fix_expr(expr: &mut Expr, unknown_field: &str, matched_column: Option<&str>) {
    match expr {
        Expr::BinaryOp { left, right, .. } => {
            fix_expr(left, unknown_field, matched_column);
            fix_expr(right, unknown_field, matched_column);
        }
        Expr::Nested(inner) => fix_expr(inner, unknown_field, matched_column),
        Expr::UnaryOp { expr, .. } => fix_expr(expr, unknown_field, matched_column),
        Expr::IsNull(expr) => fix_expr(expr, unknown_field, matched_column),
        Expr::IsNotNull(expr) => fix_expr(expr, unknown_field, matched_column),
        Expr::InList {
            expr,
            list,
            negated: _,
        } => {
            fix_expr(expr, unknown_field, matched_column);
            for item in list {
                fix_expr(item, unknown_field, matched_column);
            }
        }
        Expr::Identifier(ident) => {
            if ident.value == unknown_field {
                if let Some(real_col) = matched_column {
                    // It is a column, but likely wrong case or unquoted.
                    // Replace with correct, quoted identifier.
                    *expr = Expr::Identifier(Ident {
                        value: real_col.to_string(),
                        quote_style: Some('"'),
                        span: Span::empty(),
                    });
                } else {
                    // Not a column, treat as string literal
                    *expr = Expr::Value(Value::SingleQuotedString(ident.value.clone()).into());
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_manager::MetadataManager;
    use datafusion::prelude::SessionContext;
    use metadata_store::MetadataStore;
    use std::fs;

    #[test]
    fn test_identifier_normalizer_parse() {
        // 1. One part
        let (schema, table) = IdentifierNormalizer::parse("users");
        assert_eq!(schema, None);
        assert_eq!(table, "users");

        // 2. Two parts
        let (schema, table) = IdentifierNormalizer::parse("public.users");
        assert_eq!(schema, Some("public".to_string()));
        assert_eq!(table, "users");

        // 3. Three parts (Catalog should be ignored)
        let (schema, table) = IdentifierNormalizer::parse("datafusion.public.users");
        assert_eq!(schema, Some("public".to_string()));
        assert_eq!(table, "users");

        // 4. Quoted 3-part
        let (schema, table) = IdentifierNormalizer::parse("\"datafusion\".\"tpcc\".\"warehouse\"");
        assert_eq!(schema, Some("tpcc".to_string()));
        assert_eq!(table, "warehouse");
    }

    #[tokio::test]
    async fn test_cbo_no_guessing_for_missing_table() {
        let db_path = "test_cbo_guessing_data";
        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }

        // 1. Setup Metadata
        {
            let store = MetadataStore::new(db_path).expect("Failed to create store");
            // Add t1 (exists in YashanDB)
            store
                .add_table(
                    "default",
                    "public",
                    "t1",
                    "yashandb_conn_str",
                    "yashandb",
                    None,
                    None,
                )
                .expect("Failed to add table");
        }

        // 2. Setup Manager & Context
        let mgr = MetadataManager::new(db_path).expect("Failed to create manager");
        let ctx = SessionContext::new();

        // 3. Query with t1 (exists) and t2 (missing)
        // t2 is NOT in metadata.
        let sql = "SELECT * FROM t1, t2";

        // 4. Run Rewrite
        let result = rewrite_query(&ctx, Some(&mgr), sql)
            .await
            .expect("Rewrite failed");

        // Clean up
        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }

        println!("Rewritten SQL: {}", result);

        // 5. Assertions
        // Current behavior (Bug): t2 is guessed as "yashandb_t2" because source is yashandb.
        // Desired behavior: t2 should remain "t2" (no replacement) because it's missing from metadata.

        assert!(
            !result.contains("yashandb_t2"),
            "CBO should NOT guess table name 'yashandb_t2' for missing table 't2'"
        );
    }

    #[tokio::test]
    async fn test_cbo_cost_based_selection() {
        let db_path = "test_cbo_cost_data";
        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }

        // 1. Setup Metadata with explicit costs
        // Same table 't1' exists in both Oracle and YashanDB
        {
            let store = MetadataStore::new(db_path).expect("Failed to create store");

            // Oracle: High Cost (e.g. 1000 rows)
            store
                .add_table(
                    "default",
                    "public",
                    "t1",
                    "oracle_conn_str",
                    "oracle",
                    None,
                    Some(1000),
                )
                .expect("Failed to add oracle table");

            // YashanDB: Low Cost (e.g. 100 rows)
            // Note: In real world, we fetch stats via EXPLAIN PLAN.
            // Here we simulate the metadata store already having these stats.
            store
                .add_table(
                    "default",
                    "public",
                    "t1",
                    "yashandb_conn_str",
                    "yashandb",
                    None,
                    Some(100),
                )
                .expect("Failed to add yashandb table");
        }

        // 2. Setup Manager & Context
        let mgr = MetadataManager::new(db_path).expect("Failed to create manager");
        let ctx = SessionContext::new();

        // 3. Query t1
        let sql = "SELECT * FROM t1";

        // 4. Run Rewrite
        let result = rewrite_query(&ctx, Some(&mgr), sql)
            .await
            .expect("Rewrite failed");

        // Clean up
        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }

        println!("Rewritten SQL: {}", result);

        // 5. Assertions
        // CBO should pick YashanDB source because cost 100 < 1000
        // The mock logic in rewrite_query currently might just return the table name,
        // but let's verify if the decision log or result reflects the choice.
        // Since we don't have a full physical plan execution here, we rely on the rewriter
        // effectively mapping 't1' to the one with lower cost if it does source selection.
        //
        // However, current rewrite_query logic mainly maps logical -> physical name.
        // If both have same logical name 't1', how does it distinguish?
        // Let's check `get_best_candidate`.

        // If the rewriter replaces 't1' with a specific physical name or connection info, we check that.
        // But here both are 't1'.
        // To make this test observable, we can check the LOGS or if we had different physical names.
        // Let's assume physical names are different for test observability.
    }

    #[tokio::test]
    async fn test_cbo_cost_selection_observable() {
        let db_path = "test_cbo_cost_obs_data";
        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }

        {
            let store = MetadataStore::new(db_path).expect("Failed to create store");

            // Oracle: High Cost, Physical Name: ORACLE_T1
            store
                .add_table(
                    "default",
                    "public",
                    "t1",
                    "oracle_conn",
                    "oracle",
                    None,
                    Some(1000),
                )
                .expect("Failed to add oracle table");

            // Yashan: Low Cost, Physical Name: YASHAN_T1
            // In a real scenario, the physical table name would be different or the connection string would differ.
            // Here we use the same logical name 't1' but different source types and costs.
            // To make the selection observable in the rewritten SQL, we rely on the fact that
            // rewrite_query resolves to a specific physical table access path.
            //
            // HOWEVER, since both have logical name "t1", the rewriter needs to know WHICH one to pick.
            // The current implementation of `rewrite_query_tables` iterates over all tables in the query.
            // For 't1', it calls `get_best_candidate`.
            // `get_best_candidate` should return the one with lower cost.

            store
                .add_table(
                    "default",
                    "public",
                    "t1",
                    "yashan_conn",
                    "yashandb",
                    None,
                    Some(100),
                )
                .expect("Failed to add yashandb table");
        }

        let mgr = MetadataManager::new(db_path).expect("Failed to create manager");
        let _ctx = SessionContext::new();

        // We need to verify that the CBO actually selects the lower cost source.
        // We modified MetadataStore to support multiple sources (by source_type) for the same table.
        // So now we have 2 candidates for "t1".

        let _sql = "SELECT * FROM t1";

        // rewrite_query should internally select the best candidate.
        // The output SQL string might not change if table name is same.
        // But we can check which candidate was selected by instrumenting `rewrite_query` or
        // by verifying `get_best_candidate` behavior if we expose it.
        //
        // Since `rewrite_query` returns the rewritten SQL, if we want to observe the decision,
        // we can check if the rewriter logic does something different based on source type.
        //
        // Currently `rewrite_query` is mostly about finding the table.
        // If we want to verify CBO, we should probably check if `get_best_candidate` works.
        //
        // Let's create a unit test for `get_best_candidate` inside `query_rewriter` module
        // instead of integration test via `rewrite_query`, OR
        // trust that if we can query the `MetadataManager` for "t1", we get 2 candidates.

        let cands = mgr.find_tables("default", "public", "t1");
        assert_eq!(cands.len(), 2, "Should find 2 candidates for t1");

        // Now call the internal logic of CBO
        // We need to access `get_best_candidate` which is private.
        // We can expose it for tests or copy logic here.
        // Since we are in `mod tests` which is a child of `query_rewriter`, we can access private items?
        // `get_best_candidate` is defined in `query_rewriter.rs`.
        // `mod tests` is inside `query_rewriter.rs`. Yes, we can access it!

        let best = get_best_candidate(&cands).expect("Should find best candidate");

        // Verify the best candidate is YashanDB (cost 100)
        assert_eq!(
            best.source_type, "yashandb",
            "CBO should select yashandb due to lower cost"
        );
        // file_path stores the connection string/id in our test setup
        assert_eq!(best.file_path, "yashan_conn");

        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }
    }

    #[tokio::test]
    async fn test_cbo_greedy_cover_mixed_source() {
        let db_path = "test_cbo_mixed_data";
        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }

        {
            let store = MetadataStore::new(db_path).expect("Failed to create store");

            // Source A: Covers T1, T2, T3 (High Cost for T3)
            // Using "source_a" as source_type, "conn_a" as file_path (connection key)
            // T1
            store
                .add_table(
                    "default",
                    "public",
                    "t1_a",
                    "conn_a",
                    "source_a",
                    Some("t1".to_string()),
                    Some(100),
                )
                .unwrap();

            // T2
            store
                .add_table(
                    "default",
                    "public",
                    "t2_a",
                    "conn_a",
                    "source_a",
                    Some("t2".to_string()),
                    Some(200),
                )
                .unwrap();

            // T3 (High Cost in Source A)
            store
                .add_table(
                    "default",
                    "public",
                    "t3_a",
                    "conn_a",
                    "source_a",
                    Some("t3".to_string()),
                    Some(300),
                )
                .unwrap();

            // Source B: Covers T3 (Low Cost)
            store
                .add_table(
                    "default",
                    "public",
                    "t3_b",
                    "conn_b",
                    "source_b",
                    Some("t3".to_string()),
                    Some(50),
                )
                .unwrap();
        }

        let mgr = MetadataManager::new(db_path).expect("Failed to create manager");

        // Query: T1, T2, T3
        // We pass "t1", "t2", "t3" as logical tables.
        // The matcher will find t1_a (via sheet_name "t1"), t2_a (via sheet_name "t2"),
        // and both t3_a/t3_b (via sheet_name "t3").
        let tables = vec!["t1".to_string(), "t2".to_string(), "t3".to_string()];

        let routing_map = select_best_source(&mgr, &tables).expect("Should return routing map");

        println!("Routing Map: {:?}", routing_map);

        // Assertions
        // T1 and T2 must come from Source A
        assert_eq!(routing_map.get("t1"), Some(&"t1_a".to_string()));
        assert_eq!(routing_map.get("t2"), Some(&"t2_a".to_string()));

        // T3 should ALSO come from Source A (Max Co-location), even though Source B is cheaper (50 vs 300)
        // Because Source A covers 3 tables, Source B covers 1.
        assert_eq!(
            routing_map.get("t3"),
            Some(&"t3_a".to_string()),
            "T3 should be routed to Source A for Max Co-location"
        );

        if std::path::Path::new(db_path).exists() {
            let _ = fs::remove_dir_all(db_path);
        }
    }
}

#[tokio::test]
async fn test_quoted_table_rewrite_bug_reproduction() {
    use crate::metadata_manager::MetadataManager;
    use metadata_store::MetadataStore;
    use std::fs;
    // Setup
    let db_path = "test_quoted_bug_data";
    if std::path::Path::new(db_path).exists() {
        let _ = fs::remove_dir_all(db_path);
    }

    // We don't strictly need the store for this unit test if we mock the map,
    // but let's keep it consistent with other tests.
    let store = MetadataStore::new(db_path).expect("Failed to create store");
    store
        .add_table("default", "public", "t1", "conn_str", "oracle", None, None)
        .expect("Failed to add table");

    let mgr = MetadataManager::new(db_path).expect("Failed to create manager");
    let ctx = SessionContext::new();

    // Register routing rule for QUOTED name (simulating what select_best_source does)
    set_routing_rule("\"t1\"".to_string(), "oracle_public_t1".to_string());

    // Query using QUOTES
    let sql = "SELECT * FROM \"t1\"";

    // Manual Map for rewrite_with_physical_tables
    let mut map = HashMap::new();
    map.insert("\"t1\"".to_string(), "oracle_public_t1".to_string());

    let rewritten = rewrite_with_physical_tables(sql, &map).expect("Rewrite failed");

    println!("Rewritten SQL: {}", rewritten);

    if std::path::Path::new(db_path).exists() {
        let _ = fs::remove_dir_all(db_path);
    }

    // Assert failure until fixed
    assert!(
        rewritten.contains("oracle_public_t1"),
        "Failed to replace quoted table name. Got: {}",
        rewritten
    );
}

#[tokio::test]
async fn test_schema_qualified_table_rewrite() {
    use crate::query_rewriter::{rewrite_with_physical_tables, IdentifierNormalizer};
    use std::collections::HashMap;

    // Scenario: User queries "public"."t1", but we only have "t1" in our routing map (normalized).
    // This simulates the CBO stripping schema prefixes.

    let sql = "SELECT * FROM \"public\".\"t1\"";

    // Map contains only the simple name (simulating select_best_source output)
    let mut map = HashMap::new();
    map.insert("t1".to_string(), "oracle_public_t1".to_string());

    // Verify Normalizer Logic first
    assert_eq!(IdentifierNormalizer::normalize("\"public\".\"t1\""), "t1");

    // Run Rewrite
    let rewritten = rewrite_with_physical_tables(sql, &map).expect("Rewrite failed");

    println!("Rewritten SQL: {}", rewritten);

    // Should find "t1" in map via normalization and replace "public"."t1"
    assert!(
        rewritten.contains("oracle_public_t1"),
        "Failed to replace schema-qualified table name. Got: {}",
        rewritten
    );
}

#[test]
fn test_rewrite_with_physical_tables_keeps_limit() {
    use crate::query_rewriter::rewrite_with_physical_tables;
    use std::collections::HashMap;

    let sql = "SELECT * FROM t1 LIMIT 10";
    let mut map = HashMap::new();
    map.insert("t1".to_string(), "oracle_public_t1".to_string());

    let rewritten = rewrite_with_physical_tables(sql, &map).expect("Rewrite failed");

    assert!(rewritten.contains("oracle_public_t1"));
    assert!(rewritten.contains("LIMIT 10"));
}

#[tokio::test]
async fn test_cbo_schema_ambiguity() {
    use crate::metadata_manager::MetadataManager;
    use crate::query_rewriter::rewrite_query;
    use datafusion::prelude::SessionContext;
    use metadata_store::MetadataStore;
    use std::fs;

    let db_path = "test_cbo_schema_data";
    if std::path::Path::new(db_path).exists() {
        let _ = fs::remove_dir_all(db_path);
    }

    // Setup Metadata: Two tables with same name "users", but different schemas
    {
        let store = MetadataStore::new(db_path).expect("Failed to create store");

        // 1. Schema: public, Table: users -> Oracle
        store
            .add_table(
                "default",
                "public",
                "oracle_users",
                "conn_oracle",
                "oracle",
                Some("users".to_string()),
                Some(100),
            )
            .expect("Failed to add oracle table");

        // 2. Schema: crm, Table: users -> MySQL
        store
            .add_table(
                "default",
                "crm",
                "mysql_users",
                "conn_mysql",
                "mysql",
                Some("users".to_string()),
                Some(100),
            )
            .expect("Failed to add mysql table");
    }

    let mgr = MetadataManager::new(db_path).expect("Failed to create manager");
    let ctx = SessionContext::new();

    // Case 1: Query "public"."users" -> Should resolve to Oracle
    let sql_1 = "SELECT * FROM \"public\".\"users\"";
    let res_1 = rewrite_query(&ctx, Some(&mgr), sql_1)
        .await
        .expect("Rewrite 1 failed");

    // Case 2: Query "crm"."users" -> Should resolve to MySQL
    let sql_2 = "SELECT * FROM \"crm\".\"users\"";
    let res_2 = rewrite_query(&ctx, Some(&mgr), sql_2)
        .await
        .expect("Rewrite 2 failed");

    if std::path::Path::new(db_path).exists() {
        let _ = fs::remove_dir_all(db_path);
    }

    println!("Res 1 (public): {}", res_1);
    println!("Res 2 (crm): {}", res_2);

    // Verification
    assert!(
        res_1.contains("oracle_users"),
        "public.users should route to oracle table. Got: {}",
        res_1
    );
    assert!(
        res_2.contains("mysql_users"),
        "crm.users should route to mysql table. Got: {}",
        res_2
    );
}
