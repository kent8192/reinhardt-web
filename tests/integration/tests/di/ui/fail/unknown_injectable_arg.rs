use reinhardt_di::{FactoryOutput, injectable};

struct MyServiceKey;

impl reinhardt_di::InjectableKey for MyServiceKey {}

// This should fail: `unknown_arg` is not a valid argument for #[injectable]
#[injectable(unknown_arg = "value")]
async fn make_service() -> FactoryOutput<MyServiceKey, MyService> {
	FactoryOutput::new(MyService)
}

#[derive(Clone)]
struct MyService;

fn main() {}
