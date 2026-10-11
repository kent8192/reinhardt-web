// External crates must not build `TableInfo` with a struct literal, even when
// every field that exists today is supplied.
use reinhardt_db::migrations::TableInfo;
use std::collections::HashMap;

fn main() {
	let _ = TableInfo {
		name: "accounts".to_string(),
		columns: HashMap::new(),
		indexes: HashMap::new(),
		primary_key: Vec::new(),
		foreign_keys: Vec::new(),
		unique_constraints: Vec::new(),
		check_constraints: Vec::new(),
	};
}
