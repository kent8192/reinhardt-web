use reinhardt_pages::ClientForm;

#[derive(Clone, PartialEq, ClientForm)]
struct CollectionRequest {
	names: Vec<String>,
}

fn main() {}
