//! `JsonField<T>` compiles WITHOUT `FromStr` impl on `T`.
//!
//! This test verifies that the `FromStr` bound has been removed from
//! `JsonField`'s where clause while keeping the runtime `PartialEq`
//! requirement. `Metadata` below intentionally does NOT implement `FromStr`.

use reinhardt_pages::form;

#[derive(Clone, Default, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
struct Metadata {
	key: String,
	value: String,
}

impl ::core::fmt::Display for Metadata {
	fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		write!(f, "{}", ::serde_json::to_string(self).unwrap_or_default())
	}
}

fn main() {
	let _ = form! {
		name: MetadataForm,
		action: "/api/metadata",
		fields: {
			data: JsonField<Metadata> {
				initial: Metadata::default(),
			}
		}
	};
}
