//! Query abstraction layer
//!
//! Type definitions for passing SeaQuery objects instead of SQL strings

use sea_query::{
    DeleteStatement, IndexCreateStatement, IndexDropStatement, InsertStatement, MysqlQueryBuilder,
    PostgresQueryBuilder, SelectStatement, SqliteQueryBuilder, TableAlterStatement,
    TableCreateStatement, TableDropStatement, TableRenameStatement, UpdateStatement, Values,
};

/// Database backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbBackend {
    Postgres,
    Mysql,
    Sqlite,
}

impl DbBackend {
    /// Get query builder for this backend
    pub fn query_builder(&self) -> QueryBuilder {
        match self {
            DbBackend::Postgres => QueryBuilder::Postgres,
            DbBackend::Mysql => QueryBuilder::Mysql,
            DbBackend::Sqlite => QueryBuilder::Sqlite,
        }
    }
}

/// Query builder type
#[derive(Debug, Clone, Copy)]
pub enum QueryBuilder {
    Postgres,
    Mysql,
    Sqlite,
}

/// Unified query statement enum
///
/// Wraps SeaQuery objects for passing around instead of SQL strings
#[derive(Debug, Clone)]
pub enum QueryStatement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(TableCreateStatement),
    AlterTable(TableAlterStatement),
    DropTable(TableDropStatement),
    RenameTable(TableRenameStatement),
    CreateIndex(IndexCreateStatement),
    DropIndex(IndexDropStatement),
}

impl QueryStatement {
    /// Build SQL string and bind values according to database backend
    pub fn build(&self, backend: DbBackend) -> (String, Values) {
        match (self, backend) {
            (QueryStatement::Select(stmt), DbBackend::Postgres) => {
                stmt.clone().build(PostgresQueryBuilder)
            }
            (QueryStatement::Select(stmt), DbBackend::Mysql) => {
                stmt.clone().build(MysqlQueryBuilder)
            }
            (QueryStatement::Select(stmt), DbBackend::Sqlite) => {
                stmt.clone().build(SqliteQueryBuilder)
            }
            (QueryStatement::Insert(stmt), DbBackend::Postgres) => {
                stmt.clone().build(PostgresQueryBuilder)
            }
            (QueryStatement::Insert(stmt), DbBackend::Mysql) => {
                stmt.clone().build(MysqlQueryBuilder)
            }
            (QueryStatement::Insert(stmt), DbBackend::Sqlite) => {
                stmt.clone().build(SqliteQueryBuilder)
            }
            (QueryStatement::Update(stmt), DbBackend::Postgres) => {
                stmt.clone().build(PostgresQueryBuilder)
            }
            (QueryStatement::Update(stmt), DbBackend::Mysql) => {
                stmt.clone().build(MysqlQueryBuilder)
            }
            (QueryStatement::Update(stmt), DbBackend::Sqlite) => {
                stmt.clone().build(SqliteQueryBuilder)
            }
            (QueryStatement::Delete(stmt), DbBackend::Postgres) => {
                stmt.clone().build(PostgresQueryBuilder)
            }
            (QueryStatement::Delete(stmt), DbBackend::Mysql) => {
                stmt.clone().build(MysqlQueryBuilder)
            }
            (QueryStatement::Delete(stmt), DbBackend::Sqlite) => {
                stmt.clone().build(SqliteQueryBuilder)
            }
            (QueryStatement::CreateTable(stmt), DbBackend::Postgres) => {
                (stmt.clone().build(PostgresQueryBuilder), Values(vec![]))
            }
            (QueryStatement::CreateTable(stmt), DbBackend::Mysql) => {
                (stmt.clone().build(MysqlQueryBuilder), Values(vec![]))
            }
            (QueryStatement::CreateTable(stmt), DbBackend::Sqlite) => {
                (stmt.clone().build(SqliteQueryBuilder), Values(vec![]))
            }
            (QueryStatement::AlterTable(stmt), DbBackend::Postgres) => {
                (stmt.clone().build(PostgresQueryBuilder), Values(vec![]))
            }
            (QueryStatement::AlterTable(stmt), DbBackend::Mysql) => {
                (stmt.clone().build(MysqlQueryBuilder), Values(vec![]))
            }
            (QueryStatement::AlterTable(stmt), DbBackend::Sqlite) => {
                (stmt.clone().build(SqliteQueryBuilder), Values(vec![]))
            }
            (QueryStatement::DropTable(stmt), DbBackend::Postgres) => {
                (stmt.clone().build(PostgresQueryBuilder), Values(vec![]))
            }
            (QueryStatement::DropTable(stmt), DbBackend::Mysql) => {
                (stmt.clone().build(MysqlQueryBuilder), Values(vec![]))
            }
            (QueryStatement::DropTable(stmt), DbBackend::Sqlite) => {
                (stmt.clone().build(SqliteQueryBuilder), Values(vec![]))
            }
            (QueryStatement::RenameTable(stmt), DbBackend::Postgres) => {
                (stmt.clone().build(PostgresQueryBuilder), Values(vec![]))
            }
            (QueryStatement::RenameTable(stmt), DbBackend::Mysql) => {
                (stmt.clone().build(MysqlQueryBuilder), Values(vec![]))
            }
            (QueryStatement::RenameTable(stmt), DbBackend::Sqlite) => {
                (stmt.clone().build(SqliteQueryBuilder), Values(vec![]))
            }
            (QueryStatement::CreateIndex(stmt), DbBackend::Postgres) => {
                (stmt.clone().build(PostgresQueryBuilder), Values(vec![]))
            }
            (QueryStatement::CreateIndex(stmt), DbBackend::Mysql) => {
                (stmt.clone().build(MysqlQueryBuilder), Values(vec![]))
            }
            (QueryStatement::CreateIndex(stmt), DbBackend::Sqlite) => {
                (stmt.clone().build(SqliteQueryBuilder), Values(vec![]))
            }
            (QueryStatement::DropIndex(stmt), DbBackend::Postgres) => {
                (stmt.clone().build(PostgresQueryBuilder), Values(vec![]))
            }
            (QueryStatement::DropIndex(stmt), DbBackend::Mysql) => {
                (stmt.clone().build(MysqlQueryBuilder), Values(vec![]))
            }
            (QueryStatement::DropIndex(stmt), DbBackend::Sqlite) => {
                (stmt.clone().build(SqliteQueryBuilder), Values(vec![]))
            }
        }
    }

    /// Build SQL string only (without bind values)
    pub fn to_string(&self, backend: DbBackend) -> String {
        match (self, backend) {
            (QueryStatement::Select(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::Select(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::Select(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::Insert(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::Insert(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::Insert(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::Update(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::Update(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::Update(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::Delete(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::Delete(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::Delete(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::CreateTable(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::CreateTable(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::CreateTable(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::AlterTable(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::AlterTable(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::AlterTable(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::DropTable(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::DropTable(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::DropTable(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::RenameTable(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::RenameTable(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::RenameTable(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::CreateIndex(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::CreateIndex(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::CreateIndex(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
            (QueryStatement::DropIndex(stmt), DbBackend::Postgres) => {
                stmt.clone().to_string(PostgresQueryBuilder)
            }
            (QueryStatement::DropIndex(stmt), DbBackend::Mysql) => {
                stmt.clone().to_string(MysqlQueryBuilder)
            }
            (QueryStatement::DropIndex(stmt), DbBackend::Sqlite) => {
                stmt.clone().to_string(SqliteQueryBuilder)
            }
        }
    }
}
