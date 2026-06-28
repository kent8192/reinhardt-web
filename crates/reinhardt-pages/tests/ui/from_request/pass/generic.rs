use std::error::Error;
use std::str::FromStr;

use reinhardt_pages::FromRequest;
use reinhardt_pages::router::request::{FromRequest as _, PathParam, RouteContext};

#[derive(FromRequest)]
struct GenericRequest<T>
where
	T: FromStr,
	T::Err: Error + Send + Sync + 'static,
{
	id: PathParam<T>,
}

fn main() {
	let _: fn(&RouteContext) -> Result<GenericRequest<i64>, _> =
		GenericRequest::<i64>::from_request;
}
