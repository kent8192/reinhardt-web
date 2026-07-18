use reinhardt_macros::ModelEnum;

#[derive(ModelEnum)]
#[model_enum(repr = "string")]
enum EmptyEnum {}

fn main() {}
