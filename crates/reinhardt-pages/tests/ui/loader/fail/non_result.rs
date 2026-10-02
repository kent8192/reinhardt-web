use reinhardt_pages::loader;

#[loader]
async fn not_result() -> u8 {
	1
}

fn main() {}
