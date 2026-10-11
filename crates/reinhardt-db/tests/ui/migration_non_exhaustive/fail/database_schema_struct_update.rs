// Struct update syntax does not bypass `#[non_exhaustive]`: external crates
// must use `DatabaseSchema::new()` and its builders instead.
use reinhardt_db::migrations::introspection::DatabaseSchema;

fn main() {
	let _ = DatabaseSchema {
		default_schema: Some("public".to_string()),
		..DatabaseSchema::default()
	};
}
