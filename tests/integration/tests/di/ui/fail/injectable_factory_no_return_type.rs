// Compile-fail test: injectable without explicit return type

use reinhardt_di::injectable;

#[injectable(scope = "singleton")]
async fn make_service() {
	// No return type
}

fn main() {}
