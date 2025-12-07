use reinhardt_macros::model;

#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	// ERROR: generated + default は禁止
	#[field(generated = "id * 2", generated_stored = true, default = "0")]
	computed: i32,
}

fn main() {}
