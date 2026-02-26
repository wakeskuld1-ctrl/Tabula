use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::expr::InList;
use datafusion::prelude::Expr;
use datafusion::scalar::ScalarValue;
use std::fmt::Debug;

/// Defines the behavior for generating SQL for different database dialects.
pub trait SqlDialect: Send + Sync + Debug {
    /// Returns the name of the dialect (e.g., "sqlite", "oracle").
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Quotes an identifier (e.g., table name, column name).
    #[allow(dead_code)]
    fn quote_identifier(&self, ident: &str) -> String;

    /// Converts a DataFusion logical expression to a SQL string.
    #[allow(dead_code)]
    fn expr_to_sql(&self, expr: &Expr) -> Result<String>;

    /// Generates a paginated SQL query.
    ///
    /// # Arguments
    /// * `select_cols` - The columns to select (e.g., "id, name" or "*").
    /// * `table_name` - The name of the table.
    /// * `where_clause` - Optional WHERE clause (without "WHERE" keyword).
    /// * `limit` - The number of rows to return (batch size).
    /// * `offset` - The number of rows to skip.
    #[allow(dead_code)]
    fn generate_pagination_sql(
        &self,
        select_cols: &str,
        table_name: &str,
        where_clause: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> String;
}

/// CSV implementation of SqlDialect.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CsvDialect;

impl SqlDialect for CsvDialect {
    fn name(&self) -> &str {
        "csv"
    }

    fn quote_identifier(&self, ident: &str) -> String {
        format!("\"{}\"", ident)
    }

    fn expr_to_sql(&self, _expr: &Expr) -> Result<String> {
        Err(DataFusionError::Execution(
            "SQL generation not supported for CSV".to_string(),
        ))
    }

    fn generate_pagination_sql(
        &self,
        _select_cols: &str,
        _table_name: &str,
        _where_clause: Option<&str>,
        _limit: usize,
        _offset: usize,
    ) -> String {
        String::new() // Not supported
    }
}

/// Excel implementation of SqlDialect.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExcelDialect;

impl SqlDialect for ExcelDialect {
    fn name(&self) -> &str {
        "excel"
    }

    fn quote_identifier(&self, ident: &str) -> String {
        format!("\"{}\"", ident)
    }

    fn expr_to_sql(&self, _expr: &Expr) -> Result<String> {
        Err(DataFusionError::Execution(
            "SQL generation not supported for Excel".to_string(),
        ))
    }

    fn generate_pagination_sql(
        &self,
        _select_cols: &str,
        _table_name: &str,
        _where_clause: Option<&str>,
        _limit: usize,
        _offset: usize,
    ) -> String {
        String::new() // Not supported
    }
}

/// SQLite implementation of SqlDialect.
#[derive(Debug)]
#[allow(dead_code)]
pub struct SqliteDialect;

impl SqlDialect for SqliteDialect {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn quote_identifier(&self, ident: &str) -> String {
        // SQLite uses double quotes for identifiers usually, but strictly it depends.
        // We'll keep it simple or use brackets/quotes.
        format!("\"{}\"", ident)
    }

    fn expr_to_sql(&self, expr: &Expr) -> Result<String> {
        match expr {
            Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr { left, op, right }) => {
                let left_sql = self.expr_to_sql(left)?;
                let right_sql = self.expr_to_sql(right)?;
                Ok(format!("({} {} {})", left_sql, op, right_sql))
            }
            Expr::Column(col) => Ok(self.quote_identifier(&col.name)),
            Expr::Literal(scalar_value) => match scalar_value {
                ScalarValue::Utf8(Some(s)) => Ok(format!("'{}'", s.replace("'", "''"))),
                ScalarValue::Int64(Some(v)) => Ok(v.to_string()),
                ScalarValue::Float64(Some(v)) => Ok(v.to_string()),
                ScalarValue::Boolean(Some(v)) => {
                    Ok(if *v { "1".to_string() } else { "0".to_string() })
                }
                _ => Err(DataFusionError::Execution(format!(
                    "Unsupported literal: {:?}",
                    scalar_value
                ))),
            },
            Expr::Like(datafusion::logical_expr::Like {
                expr,
                pattern,
                negated,
                escape_char: _,
                case_insensitive,
            }) => {
                if *case_insensitive {
                    return Err(DataFusionError::Execution(
                        "Case-insensitive LIKE not supported for SQLite yet".to_string(),
                    ));
                }
                let expr_sql = self.expr_to_sql(expr)?;
                let pattern_sql = self.expr_to_sql(pattern)?;
                let op = if *negated { "NOT LIKE" } else { "LIKE" };
                Ok(format!("{} {} {}", expr_sql, op, pattern_sql))
            }
            Expr::IsNull(expr) => {
                let expr_sql = self.expr_to_sql(expr)?;
                Ok(format!("{} IS NULL", expr_sql))
            }
            Expr::IsNotNull(expr) => {
                let expr_sql = self.expr_to_sql(expr)?;
                Ok(format!("{} IS NOT NULL", expr_sql))
            }
            Expr::InList(InList {
                expr,
                list,
                negated,
            }) => {
                let expr_sql = self.expr_to_sql(expr)?;
                let list_sql = list
                    .iter()
                    .map(|e| self.expr_to_sql(e))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let op = if *negated { "NOT IN" } else { "IN" };
                Ok(format!("{} {} ({})", expr_sql, op, list_sql))
            }
            Expr::Not(expr) => {
                let expr_sql = self.expr_to_sql(expr)?;
                Ok(format!("NOT ({})", expr_sql))
            }
            _ => Err(DataFusionError::Execution(format!(
                "Unsupported expression for SQLite: {:?}",
                expr
            ))),
        }
    }

    fn generate_pagination_sql(
        &self,
        select_cols: &str,
        table_name: &str,
        where_clause: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> String {
        let mut sql = format!("SELECT {} FROM {}", select_cols, table_name);
        if let Some(w) = where_clause {
            sql.push_str(" WHERE ");
            sql.push_str(w);
        }
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        sql
    }
}

/// Oracle implementation of SqlDialect.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OracleDialect {
    /// If true, use ROWNUM based pagination (Oracle 11g and older).
    /// If false, use OFFSET ... FETCH NEXT (Oracle 12c+).
    pub use_legacy_pagination: bool,
}

impl OracleDialect {
    #[allow(dead_code)]
    pub fn new(use_legacy_pagination: bool) -> Self {
        Self {
            use_legacy_pagination,
        }
    }
}

impl SqlDialect for OracleDialect {
    fn name(&self) -> &str {
        "oracle"
    }

    fn quote_identifier(&self, ident: &str) -> String {
        format!("\"{}\"", ident.to_uppercase()) // Oracle identifiers are usually uppercase
    }

    fn expr_to_sql(&self, expr: &Expr) -> Result<String> {
        match expr {
            Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr { left, op, right }) => {
                let left_sql = self.expr_to_sql(left)?;
                let right_sql = self.expr_to_sql(right)?;
                Ok(format!("({} {} {})", left_sql, op, right_sql))
            }
            Expr::Column(col) => Ok(self.quote_identifier(&col.name)),
            Expr::Literal(scalar_value) => {
                match scalar_value {
                    ScalarValue::Utf8(Some(s)) => Ok(format!("'{}'", s.replace("'", "''"))),
                    ScalarValue::Int64(Some(v)) => Ok(v.to_string()),
                    ScalarValue::Float64(Some(v)) => Ok(v.to_string()),
                    // Oracle doesn't have BOOLEAN type in SQL (only PL/SQL), usually uses 1/0 or 'Y'/'N'
                    ScalarValue::Boolean(Some(v)) => {
                        Ok(if *v { "1".to_string() } else { "0".to_string() })
                    }
                    _ => Err(DataFusionError::Execution(format!(
                        "Unsupported literal: {:?}",
                        scalar_value
                    ))),
                }
            }
            Expr::Like(datafusion::logical_expr::Like {
                expr,
                pattern,
                negated,
                escape_char: _,
                case_insensitive,
            }) => {
                if *case_insensitive {
                    return Err(DataFusionError::Execution(
                        "Case-insensitive LIKE not supported for Oracle yet".to_string(),
                    ));
                }
                let expr_sql = self.expr_to_sql(expr)?;
                let pattern_sql = self.expr_to_sql(pattern)?;
                let op = if *negated { "NOT LIKE" } else { "LIKE" };
                Ok(format!("{} {} {}", expr_sql, op, pattern_sql))
            }
            Expr::IsNull(expr) => {
                let expr_sql = self.expr_to_sql(expr)?;
                Ok(format!("{} IS NULL", expr_sql))
            }
            Expr::IsNotNull(expr) => {
                let expr_sql = self.expr_to_sql(expr)?;
                Ok(format!("{} IS NOT NULL", expr_sql))
            }
            Expr::InList(InList {
                expr,
                list,
                negated,
            }) => {
                let expr_sql = self.expr_to_sql(expr)?;
                let list_sql = list
                    .iter()
                    .map(|e| self.expr_to_sql(e))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let op = if *negated { "NOT IN" } else { "IN" };
                Ok(format!("{} {} ({})", expr_sql, op, list_sql))
            }
            Expr::Not(expr) => {
                let expr_sql = self.expr_to_sql(expr)?;
                Ok(format!("NOT ({})", expr_sql))
            }
            _ => Err(DataFusionError::Execution(format!(
                "Unsupported expression for Oracle: {:?}",
                expr
            ))),
        }
    }

    fn generate_pagination_sql(
        &self,
        select_cols: &str,
        table_name: &str,
        where_clause: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> String {
        let where_part = where_clause
            .map(|w| format!("WHERE {}", w))
            .unwrap_or_default();

        if self.use_legacy_pagination {
            // Oracle 11g - ROWNUM strategy
            // SELECT * FROM (
            //   SELECT t.*, ROWNUM rnum FROM (SELECT cols FROM table WHERE ...) t
            //   WHERE ROWNUM <= end
            // ) WHERE rnum > start
            let start = offset;
            let end = offset + limit;

            format!(
                "SELECT {} FROM (
                    SELECT t.*, ROWNUM rnum FROM (SELECT {} FROM {} {}) t
                    WHERE ROWNUM <= {}
                ) WHERE rnum > {}",
                select_cols, select_cols, table_name, where_part, end, start
            )
        } else {
            // Oracle 12c+ - OFFSET FETCH strategy
            // SELECT cols FROM table WHERE ... OFFSET offset ROWS FETCH NEXT limit ROWS ONLY
            format!(
                "SELECT {} FROM {} {} OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
                select_cols, table_name, where_part, offset, limit
            )
        }
    }
}
