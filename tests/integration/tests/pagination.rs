// Auto-generated module file for pagination integration tests
// Each test file in pagination/ subdirectory is explicitly included with #[path] attribute

#[path = "pagination/orm.rs"]
mod orm;

#[path = "pagination/orm_advanced.rs"]
mod orm_advanced;

#[path = "pagination/cursor_integration.rs"]
mod cursor_integration;

// TODO: Future test implementations
// - limit_offset_integration.rs: Requires database-level pagination API (not yet implemented)
//   Current LimitOffsetPagination only supports in-memory collections (&[T])
// - performance_tests.rs: Requires database-level pagination API for large dataset testing
