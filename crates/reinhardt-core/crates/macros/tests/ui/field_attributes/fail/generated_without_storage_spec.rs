use reinhardt_macros::model;

#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	// ERROR: generated_stored または generated_virtual が必須
	#[field(generated = "id * 2")]
	computed: i32,
}

fn main() {}
