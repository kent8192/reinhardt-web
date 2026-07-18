use reinhardt_macros::ModelEnum;

#[derive(ModelEnum)]
#[model_enum(repr = "i32")]
enum PayloadVariant {
	#[model_enum(value = 1)]
	Queued(String),
}

fn main() {}
