use reinhardt_pages::loader;

#[loader]
fn not_async() -> Result<u8, String> {
	Ok(1)
}

fn main() {}
