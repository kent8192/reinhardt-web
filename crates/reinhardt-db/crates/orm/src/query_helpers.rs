//! Query helper functions
//!
//! Common query patterns using SeaQuery for ORM operations.

use sea_query::{
    Alias, Asterisk, DeleteStatement, Expr, ExprTrait, Func, InsertStatement, Query,
    SelectStatement, UpdateStatement,
};

use crate::model::Model;

/// Build SELECT COUNT(*) query for a model
///
/// # Example
/// ```rust,ignore
/// let stmt = build_count_query::<User>();
/// // SELECT COUNT(*) FROM users
/// ```
pub fn build_count_query<M: Model>() -> SelectStatement {
    Query::select()
        .expr(Func::count(Expr::col(Asterisk)))
        .from(Alias::new(M::table_name()))
        .to_owned()
}

/// Build SELECT * WHERE pk = ? LIMIT 1 query for a model
///
/// # Example
/// ```rust,ignore
/// let stmt = build_get_query::<User>(1);
/// // SELECT * FROM users WHERE id = $1 LIMIT 1
/// ```
pub fn build_get_query<M: Model, V>(pk: V) -> SelectStatement
where
    V: Into<sea_query::Value>,
{
    Query::select()
        .from(Alias::new(M::table_name()))
        .columns([Asterisk])
        .and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk.into()))
        .limit(1)
        .to_owned()
}

/// Build DELETE WHERE pk = ? query for a model
///
/// # Example
/// ```rust,ignore
/// let stmt = build_delete_query::<User>(1);
/// // DELETE FROM users WHERE id = $1
/// ```
pub fn build_delete_query<M: Model, V>(pk: V) -> DeleteStatement
where
    V: Into<sea_query::Value>,
{
    Query::delete()
        .from_table(Alias::new(M::table_name()))
        .and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk.into()))
        .to_owned()
}

/// Build INSERT INTO table (columns...) VALUES (values...) query
///
/// # Example
/// ```rust,ignore
/// let stmt = build_insert_query::<User>(
///     vec!["name", "email"],
///     vec!["Alice".into(), "alice@example.com".into()]
/// );
/// // INSERT INTO users (name, email) VALUES ($1, $2)
/// ```
pub fn build_insert_query<M: Model>(
    columns: Vec<&str>,
    values: Vec<sea_query::Value>,
) -> InsertStatement {
    let mut stmt = Query::insert()
        .into_table(Alias::new(M::table_name()))
        .to_owned();

    // Set columns
    let col_refs: Vec<_> = columns.iter().map(|c| Alias::new(*c)).collect();
    stmt.columns(col_refs);

    // Set values - values_panic expects an array/slice, not a Vec
    if !values.is_empty() {
        // Convert Vec<Value> to expressions for values_panic
        let exprs: Vec<_> = values.into_iter().map(Expr::val).collect();
        stmt.values_panic(exprs);
    }

    stmt
}

/// Build UPDATE table SET columns = values WHERE pk = ? query
///
/// # Example
/// ```rust,ignore
/// let stmt = build_update_query::<User>(
///     vec![("name", "Bob".into()), ("email", "bob@example.com".into())],
///     1
/// );
/// // UPDATE users SET name = $1, email = $2 WHERE id = $3
/// ```
pub fn build_update_query<M: Model, V>(
    updates: Vec<(&str, sea_query::Value)>,
    pk: V,
) -> UpdateStatement
where
    V: Into<sea_query::Value>,
{
    let mut stmt = Query::update()
        .table(Alias::new(M::table_name()))
        .to_owned();

    for (col, val) in updates {
        stmt.value(Alias::new(col), val);
    }

    stmt.and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk.into()))
        .to_owned()
}

/// Build SELECT EXISTS(...) query
///
/// # Example
/// ```rust,ignore
/// let inner = Query::select()
///     .from(Alias::new("users"))
///     .and_where(Expr::col("id").eq(1))
///     .to_owned();
/// let stmt = build_exists_query(inner);
/// // SELECT EXISTS(SELECT * FROM users WHERE id = 1)
/// ```
pub fn build_exists_query(inner: SelectStatement) -> SelectStatement {
    Query::select().expr(Expr::exists(inner)).to_owned()
}

/// Build SELECT * FROM table WHERE column IN (values...) query
///
/// # Example
/// ```rust,ignore
/// let stmt = build_in_query::<User>("id", vec![1.into(), 2.into(), 3.into()]);
/// // SELECT * FROM users WHERE id IN ($1, $2, $3)
/// ```
pub fn build_in_query<M: Model>(column: &str, values: Vec<sea_query::Value>) -> SelectStatement {
    Query::select()
        .from(Alias::new(M::table_name()))
        .columns([Asterisk])
        .and_where(Expr::col(Alias::new(column)).is_in(values))
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};
    use serde::{Deserialize, Serialize};

    // Mock Model for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestModel {
        id: i64,
    }

    impl Model for TestModel {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "test_table"
        }

        fn primary_key_field() -> &'static str {
            "id"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            Some(&self.id)
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = value;
        }
    }

    #[test]
    fn test_build_count_query() {
        let stmt = build_count_query::<TestModel>();
        let sql = stmt.to_string(PostgresQueryBuilder);
        assert!(sql.contains("COUNT"));
        assert!(sql.contains("test_table"));
    }

    #[test]
    fn test_build_get_query() {
        let stmt = build_get_query::<TestModel, _>(123);
        let (sql, values) = stmt.build(PostgresQueryBuilder);
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("test_table"));
        assert!(sql.contains("id"));
        assert!(sql.contains("LIMIT"));
        // SeaQuery binds LIMIT value as a parameter, so we have 2 values: pk + limit
        assert_eq!(values.0.len(), 2);
    }

    #[test]
    fn test_build_delete_query() {
        let stmt = build_delete_query::<TestModel, _>(456);
        let (sql, values) = stmt.build(PostgresQueryBuilder);
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("test_table"));
        assert!(sql.contains("id"));
        assert_eq!(values.0.len(), 1);
    }

    #[test]
    fn test_build_insert_query() {
        let stmt = build_insert_query::<TestModel>(
            vec!["name", "email"],
            vec!["Alice".into(), "alice@example.com".into()],
        );
        let (sql, values) = stmt.build(PostgresQueryBuilder);
        assert!(sql.contains("INSERT"));
        assert!(sql.contains("test_table"));
        assert!(sql.contains("name"));
        assert!(sql.contains("email"));
        assert_eq!(values.0.len(), 2);
    }

    #[test]
    fn test_build_update_query() {
        let stmt = build_update_query::<TestModel, _>(
            vec![("name", "Bob".into()), ("age", 30.into())],
            789,
        );
        let (sql, values) = stmt.build(PostgresQueryBuilder);
        assert!(sql.contains("UPDATE"));
        assert!(sql.contains("test_table"));
        assert!(sql.contains("name"));
        assert!(sql.contains("age"));
        assert!(sql.contains("id"));
        assert_eq!(values.0.len(), 3); // 2 updates + 1 where condition
    }

    #[test]
    fn test_build_in_query() {
        let stmt = build_in_query::<TestModel>("status", vec!["active".into(), "pending".into()]);
        let (sql, values) = stmt.build(PostgresQueryBuilder);
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("test_table"));
        assert!(sql.contains("IN"));
        assert_eq!(values.0.len(), 2);
    }

    #[test]
    fn test_different_backends() {
        let stmt = build_count_query::<TestModel>();

        // PostgreSQL uses double quotes
        let pg_sql = stmt.to_string(PostgresQueryBuilder);
        assert!(pg_sql.contains("\"test_table\""));

        // MySQL uses backticks
        let mysql_sql = stmt.to_string(MysqlQueryBuilder);
        assert!(mysql_sql.contains("`test_table`"));

        // SQLite uses double quotes
        let sqlite_sql = stmt.to_string(SqliteQueryBuilder);
        assert!(sqlite_sql.contains("\"test_table\""));
    }
}
