use reinhardt_macros::injectable;

struct MyService;

#[injectable]
fn bad_factory(regular_param: String) -> MyService {
	MyService
}

fn main() {}
