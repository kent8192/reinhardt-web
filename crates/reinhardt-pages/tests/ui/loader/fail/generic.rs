use reinhardt_pages::loader;

#[loader]
async fn generic<T>() -> Result<T, String> {
	panic!()
}

fn main() {}
