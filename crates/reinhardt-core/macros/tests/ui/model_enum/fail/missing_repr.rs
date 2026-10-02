use reinhardt_macros::ModelEnum;

#[derive(ModelEnum)]
enum MissingRepr {
	#[model_enum(value = "queued")]
	Queued,
}

fn main() {}
