use reinhardt_pages::ClientForm;

#[derive(Clone, Default, PartialEq, ClientForm)]
struct CollisionRequest {
	#[serde(rename = "second")]
	first: String,
	second: String,
}

fn main() {}
