use reinhardt_di::injectable;

struct MyServiceKey;

impl reinhardt_di::InjectableKey for MyServiceKey {}

// This should fail: `unknown_arg` is not a valid argument for #[injectable]
#[injectable(unknown_arg = "value")]
async fn make_service() -> reinhardt_di::FactoryOutput<MyServiceKey, MyService> {
	reinhardt_di::FactoryOutput::new(MyService)
}

#[derive(Clone)]
struct MyService;

fn main() {}
