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
            Expr::Literal(scalar_value, _) => match scalar_value {
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

    /// 生成带参数的 SQL
    ///
    /// **实现方案**:
    /// 遍历 Expr 树，将 Literal 替换为占位符 (`:1`, `:2` 等)，并将实际值收集到 `params` 向量中。
    ///
    /// **调用链路**:
    /// - 被 `OracleTable::supports_filters_pushdown` 和 `scan` 调用。
    ///
    /// **关键问题点**:
    /// - SQL 注入防护：通过参数化查询防止注入。
    // New method for parameterized query generation
    pub fn expr_to_sql_with_params(&self, expr: &Expr) -> Result<(String, Vec<ScalarValue>)> {
        let mut params = Vec::new();
        let sql = self.expr_to_sql_internal(expr, Some(&mut params))?;
        Ok((sql, params))
    }

    fn expr_to_sql_internal(
        &self,
        expr: &Expr,
        mut params: Option<&mut Vec<ScalarValue>>,
    ) -> Result<String> {
        match expr {
            Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr { left, op, right }) => {
                // Pass reborrowed mutable reference if present
                let left_sql = self.expr_to_sql_internal(left, params.as_deref_mut())?;
                let right_sql = self.expr_to_sql_internal(right, params.as_deref_mut())?;
                Ok(format!("({} {} {})", left_sql, op, right_sql))
            }
            Expr::Column(col) => Ok(self.quote_identifier(&col.name)),
            Expr::Literal(scalar_value, _) => {
                if let Some(p) = params {
                    p.push(scalar_value.clone());
                    Ok(format!(":{}", p.len()))
                } else {
                    match scalar_value {
                        ScalarValue::Utf8(Some(s)) => Ok(format!("'{}'", s.replace("'", "''"))),
                        ScalarValue::Int64(Some(v)) => Ok(v.to_string()),
                        ScalarValue::Int32(Some(v)) => Ok(v.to_string()), // Added Int32 support
                        ScalarValue::Float64(Some(v)) => Ok(v.to_string()),
                        ScalarValue::Boolean(Some(v)) => {
                            Ok(if *v { "1".to_string() } else { "0".to_string() })
                        }
                        _ => Err(DataFusionError::Execution(format!(
                            "Unsupported literal: {:?}",
                            scalar_value
                        ))),
                    }
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
                let expr_sql = self.expr_to_sql_internal(expr, params.as_deref_mut())?;
                let pattern_sql = self.expr_to_sql_internal(pattern, params.as_deref_mut())?;
                let op = if *negated { "NOT LIKE" } else { "LIKE" };
                Ok(format!("{} {} {}", expr_sql, op, pattern_sql))
            }
            Expr::IsNull(expr) => {
                let expr_sql = self.expr_to_sql_internal(expr, params.as_deref_mut())?;
                Ok(format!("{} IS NULL", expr_sql))
            }
            Expr::IsNotNull(expr) => {
                let expr_sql = self.expr_to_sql_internal(expr, params.as_deref_mut())?;
                Ok(format!("{} IS NOT NULL", expr_sql))
            }
            Expr::InList(InList {
                expr,
                list,
                negated,
            }) => {
                let expr_sql = self.expr_to_sql_internal(expr, params.as_deref_mut())?;
                let list_sql = list
                    .iter()
                    .map(|e| self.expr_to_sql_internal(e, params.as_deref_mut()))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let op = if *negated { "NOT IN" } else { "IN" };
                Ok(format!("{} {} ({})", expr_sql, op, list_sql))
            }
            Expr::Not(expr) => {
                let expr_sql = self.expr_to_sql_internal(expr, params.as_deref_mut())?;
                Ok(format!("NOT ({})", expr_sql))
            }
            Expr::Case(datafusion::logical_expr::Case {
                expr,
                when_then_expr,
                else_expr,
            }) => {
                let mut sql = "CASE".to_string();
                if let Some(e) = expr {
                    sql.push_str(&format!(" {}", self.expr_to_sql_internal(e, params.as_deref_mut())?));
                }
                for (when, then) in when_then_expr {
                    let when_sql = self.expr_to_sql_internal(when, params.as_deref_mut())?;
                    let then_sql = self.expr_to_sql_internal(then, params.as_deref_mut())?;
                    sql.push_str(&format!(" WHEN {} THEN {}", when_sql, then_sql));
                }
                if let Some(e) = else_expr {
                    sql.push_str(&format!(" ELSE {}", self.expr_to_sql_internal(e, params.as_deref_mut())?));
                }
                sql.push_str(" END");
                Ok(sql)
            }
            _ => Err(DataFusionError::Execution(format!(
                "Unsupported expression for Oracle: {:?}",
                expr
            ))),
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
        self.expr_to_sql_internal(expr, None)
    }

    /// 生成分页 SQL
    ///
    /// **实现方案**:
    /// 根据 `use_legacy_pagination` 字段决定生成策略：
    /// - **Legacy (Oracle 11g-)**: 使用 `ROWNUM` 嵌套查询 (`SELECT * FROM (SELECT t.*, ROWNUM rnum ...`)。
    /// - **Modern (Oracle 12c+)**: 使用 `OFFSET ... FETCH NEXT ...` 语法。
    ///
    /// **关键问题点**:
    /// - 版本兼容性：默认应检测数据库版本或由用户配置。
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

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::logical_expr::{col, lit, case};
    use datafusion::scalar::ScalarValue;

    #[test]
    fn test_oracle_expr_to_sql_parameterization() {
        let dialect = OracleDialect::new(false);

        // Test Case 1: Basic Comparison with Parameter
        // col("id") = 1
        let expr = col("id").eq(lit(1));
        
        // We verify that the method is implemented and returns correct parameterized SQL
        let result = dialect.expr_to_sql_with_params(&expr);
        
        assert!(result.is_ok(), "Expected success, got error: {:?}", result.err());
        let (sql, params) = result.unwrap();
        assert_eq!(sql, "(\"ID\" = :1)");
        assert_eq!(params.len(), 1);
        match &params[0] {
            ScalarValue::Int32(Some(v)) => assert_eq!(*v, 1),
            _ => panic!("Expected Int32 parameter, got {:?}", params[0]),
        }
    }

    #[test]
    fn test_oracle_expr_to_sql_m2_case_when() {
        let dialect = OracleDialect::new(false);

        // Test Case 2: CASE WHEN (M2 Requirement)
        // CASE WHEN status = 1 THEN 'active' ELSE 'inactive' END
        let expr = case(col("status").eq(lit(1)))
            .when(lit(true), lit("active")) // Simplified for test construction
            .otherwise(lit("inactive"))
            .unwrap();

        // Check if expr_to_sql handles it -> Expect success now
        let result = dialect.expr_to_sql(&expr);
        assert!(result.is_ok(), "Expected CASE WHEN support, got error: {:?}", result.err());
        let sql = result.unwrap();
        // CASE ("STATUS" = 1) WHEN true THEN 'active' ELSE 'inactive' END
        assert!(sql.starts_with("CASE"));
        assert!(sql.contains("WHEN"));
        assert!(sql.contains("THEN 'active'"));
        assert!(sql.contains("ELSE 'inactive'"));
        assert!(sql.ends_with("END"));
    }

    #[test]
    fn test_oracle_expr_to_sql_m3_window_function() {
        let dialect = OracleDialect::new(false);
        // Note: Constructing WindowFunction expr is complex in unit test without context, 
        // using a placeholder unsupported expr for now to verify fallback logic.
        // We use a Cast expr which might be M3 or M2 depending on complexity.
        let expr = datafusion::logical_expr::cast(col("price"), datafusion::arrow::datatypes::DataType::Utf8);
        
        // Expect failure
        let result = dialect.expr_to_sql(&expr);
        assert!(result.is_err());
    }
}
