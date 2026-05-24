//! A user type without `Default` must trip the struct-level where clause
//! emitted in Task 9 (`T: ... + ::core::default::Default + ...`).

use reinhardt_pages::form;

#[derive(Clone, ::serde::Serialize, ::serde::Deserialize)]
struct NoDefault {
	v: i64,
}

impl ::core::fmt::Display for NoDefault {
	fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		write!(f, "{}", self.v)
	}
}

impl ::core::str::FromStr for NoDefault {
	type Err = ::std::num::ParseIntError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(NoDefault { v: s.parse()? })
	}
}

fn main() {
	let _ = form! {
		name: BadForm,
		action: "/x",

		fields: {
			data: HiddenField<NoDefault> { },
		},
	};
}
