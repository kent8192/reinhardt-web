// Auto-generated module file for pagination integration tests
// Each test file in pagination/ subdirectory is explicitly included with #[path] attribute

#[path = "pagination/orm.rs"]
mod orm;

#[path = "pagination/orm_advanced.rs"]
mod orm_advanced;

#[path = "pagination/cursor_integration.rs"]
mod cursor_integration;

// TODO: Database-level pagination tests
//
// The following tests are not implemented because:
// - LimitOffsetPagination::paginate() operates on in-memory slices (&[T])
// - SelectQuery generates SQL with limit/offset but requires SqlAlchemyEngine for execution
// - Database-level pagination would require:
//   1. A method like SelectQuery::execute(engine) -> Result<Vec<T>>
//   2. Or integration with QueryExecution trait
//
// Current integration tests for limit/offset are in orm.rs and orm_advanced.rs
// which test the SQL generation and in-memory pagination separately.
