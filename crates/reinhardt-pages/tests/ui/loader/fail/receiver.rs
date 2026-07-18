use reinhardt_pages::loader;

#[loader]
async fn receiver(self) -> Result<(), String> {
	Ok(())
}

fn main() {}
