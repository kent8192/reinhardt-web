use reinhardt_macros::ModelEnum;

#[derive(ModelEnum)]
#[model_enum(repr = "i32")]
enum ExplicitDiscriminant {
	#[model_enum(value = 1)]
	Queued = 1,
}

fn main() {}
