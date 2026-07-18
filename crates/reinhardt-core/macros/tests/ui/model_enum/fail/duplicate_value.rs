use reinhardt_macros::ModelEnum;

#[derive(ModelEnum)]
#[model_enum(repr = "string")]
enum DuplicateStatus {
	#[model_enum(value = "queued")]
	First,
	#[model_enum(value = "queued")]
	Second,
}

fn main() {}
