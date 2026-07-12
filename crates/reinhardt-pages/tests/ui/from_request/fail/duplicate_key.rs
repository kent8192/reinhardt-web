use reinhardt_pages::FromRequest;
use reinhardt_pages::router::request::{PathParam, QueryParam};

#[derive(FromRequest)]
struct BadRequest {
	id: PathParam<i64>,
	#[from_request(name = "id")]
	tab: QueryParam<String>,
}

fn main() {}
