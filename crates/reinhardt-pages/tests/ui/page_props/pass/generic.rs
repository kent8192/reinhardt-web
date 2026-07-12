use std::error::Error;
use std::str::FromStr;

use reinhardt_pages::page_props;
use reinhardt_pages::router::request::{FromRequest, RouteContext};

#[page_props]
struct GenericPageProps<T>
where
	T: FromStr,
	T::Err: Error + Send + Sync + 'static,
{
	#[from_request(path)]
	id: T,
}

fn main() {
	let _: GenericPageProps<i64> = GenericPageProps::builder().id(7).build();
	let _: fn(&RouteContext) -> Result<GenericPageProps<i64>, _> =
		GenericPageProps::<i64>::from_request;
}
