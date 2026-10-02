use reinhardt_macros::ModelEnum;

#[derive(ModelEnum)]
#[model_enum(repr = "string")]
enum WrongLiteralType {
	#[model_enum(value = 1)]
	Queued,
}

fn main() {}
