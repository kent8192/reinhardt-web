use reinhardt_pages::FromRequest;
use reinhardt_pages::router::request::PathParam;

#[derive(FromRequest)]
struct BadRequest(PathParam<i64>);

fn main() {}
