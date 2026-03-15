use datafusion::error::Result;
use datafusion::prelude::SessionContext;
use datafusion::sql::parser::{DFParser, Statement as DFStatement};
use datafusion::sql::sqlparser::ast::{
    Expr, Ident, ObjectName, SetExpr, Statement, TableFactor, Value,
};
use std::collections::HashSet;

pub async fn rewrite_query(ctx: &SessionContext, sql: &str) -> Result<String> {
    // 1. Parse SQL using DataFusion's parser
    let mut statements = DFParser::parse_sql(sql)?;

    // 2. Process each statement
    for statement in &mut statements {
        rewrite_statement(ctx, statement).await?;
    }

    // 3. Reconstruct SQL
    let new_sql = statements
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("; ");
    Ok(new_sql)
}

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
                    });
                } else {
                    // Not a column, treat as string literal
                    *expr = Expr::Value(Value::SingleQuotedString(ident.value.clone()));
                }
            }
        }
        _ => {}
    }
}

async fn rewrite_statement(ctx: &SessionContext, statement: &mut DFStatement) -> Result<()> {
    if let DFStatement::Statement(stmt) = statement {
        rewrite_ast_statement(ctx, stmt).await?;
    }
    Ok(())
}

async fn rewrite_ast_statement(ctx: &SessionContext, statement: &mut Statement) -> Result<()> {
    if let Statement::Query(query) = statement {
        rewrite_query_body(ctx, &mut query.body).await?;
    }
    Ok(())
}

async fn rewrite_query_body(ctx: &SessionContext, body: &mut SetExpr) -> Result<()> {
    if let SetExpr::Select(select) = body {
        // 0. Fix Table Names in FROM
        for table in &mut select.from {
            fix_table_factor(ctx, &mut table.relation).await?;
            for join in &mut table.joins {
                fix_table_factor(ctx, &mut join.relation).await?;
            }
        }

        // 1. Identify tables and collect valid columns
        let mut valid_columns = HashSet::new();
        let mut table_found = false;

        for table in &select.from {
            if collect_columns(ctx, &table.relation, &mut valid_columns).await? {
                table_found = true;
            }
            for join in &table.joins {
                if collect_columns(ctx, &join.relation, &mut valid_columns).await? {
                    table_found = true;
                }
            }
        }

        if !table_found {
            return Ok(());
        }

        // 2. Rewrite Selection (WHERE clause)
        if let Some(selection) = &mut select.selection {
            rewrite_expr(selection, &valid_columns);
        }
    }
    Ok(())
}

async fn fix_table_factor(ctx: &SessionContext, relation: &mut TableFactor) -> Result<()> {
    if let TableFactor::Table { name, .. } = relation {
        let table_name = name.to_string();
        // Check if exists using DataFusion's standard lookup
        if ctx.table_provider(&table_name).await.is_err() {
            // Try to find a match in catalog (case-insensitive)
            // Assuming "datafusion" catalog and "public" schema for now
            if let Some(catalog) = ctx.catalog("datafusion") {
                if let Some(schema) = catalog.schema("public") {
                    let all_tables = schema.table_names();

                    if let Some(real_name) = all_tables
                        .iter()
                        .find(|t| t.eq_ignore_ascii_case(&table_name))
                    {
                        // Update name to be quoted real_name
                        *name = ObjectName(vec![Ident {
                            value: real_name.clone(),
                            quote_style: Some('"'),
                        }]);
                    }
                }
            }
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

fn rewrite_expr(expr: &mut Expr, valid_columns: &HashSet<String>) {
    match expr {
        Expr::BinaryOp { left, right, .. } => {
            rewrite_expr(left, valid_columns);
            rewrite_expr(right, valid_columns);

            maybe_convert_to_string(left, valid_columns);
            maybe_convert_to_string(right, valid_columns);
        }
        Expr::Nested(inner) => rewrite_expr(inner, valid_columns),
        Expr::UnaryOp { expr, .. } => rewrite_expr(expr, valid_columns),
        Expr::IsNull(expr) => rewrite_expr(expr, valid_columns),
        Expr::IsNotNull(expr) => rewrite_expr(expr, valid_columns),
        _ => {}
    }
}

fn maybe_convert_to_string(expr: &mut Box<Expr>, valid_columns: &HashSet<String>) {
    if let Expr::Identifier(ident) = &**expr {
        let name = ident.value.clone();

        if !valid_columns.contains(&name) {
            let upper = name.to_uppercase();
            if upper == "TRUE" || upper == "FALSE" || upper == "NULL" {
                return;
            }

            **expr = Expr::Value(Value::SingleQuotedString(name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::MemTable;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_rewrite_unquoted_string() {
        let ctx = SessionContext::new();

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let table = MemTable::try_new(schema, vec![vec![]]).unwrap();
        ctx.register_table("users", Arc::new(table)).unwrap();

        // Case 1: Unquoted string 'alice' -> 'alice'
        let sql = "SELECT * FROM users WHERE name = alice";
        let rewritten = rewrite_query(&ctx, sql).await.unwrap();
        assert!(rewritten.contains("name = 'alice'"));

        // Case 2: Quoted string 'alice' -> 'alice'
        let sql = "SELECT * FROM users WHERE name = 'alice'";
        let rewritten = rewrite_query(&ctx, sql).await.unwrap();
        assert!(rewritten.contains("name = 'alice'"));

        // Case 3: Column comparison
        let sql = "SELECT * FROM users WHERE name = id";
        let rewritten = rewrite_query(&ctx, sql).await.unwrap();
        assert!(rewritten.contains("name = id"));
    }
}
