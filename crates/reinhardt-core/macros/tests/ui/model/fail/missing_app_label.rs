use reinhardt_macros::model;

include!("../support.rs");

#[model(table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: i64,
}

fn main() {}
