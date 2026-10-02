use reinhardt_pages::loader;

#[loader]
async fn unsupported(value: i64) -> Result<i64, String> {
	Ok(value)
}

fn main() {}
