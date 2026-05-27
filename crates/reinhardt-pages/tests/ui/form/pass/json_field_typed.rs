//! `JsonField<T>` with a user-supplied struct as the inner type.
//!
//! The where clause emitted in Task 9 requires `T: Serialize +
//! DeserializeOwned + Clone + Display + FromStr + Default + 'static`.
//! `UserPrefs` below satisfies all of those.

use reinhardt_pages::form;

#[derive(Clone, Default, ::serde::Serialize, ::serde::Deserialize)]
struct UserPrefs {
	theme: String,
}

impl ::core::fmt::Display for UserPrefs {
	fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		write!(f, "{}", self.theme)
	}
}

impl ::core::str::FromStr for UserPrefs {
	type Err = ::std::convert::Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(UserPrefs {
			theme: s.to_string(),
		})
	}
}

fn main() {
	let _ = form! {
		name: PrefsForm,
		action: "/api/prefs",
		fields: {
			prefs: JsonField {
				initial: UserPrefs::default(),
			}
		}
	};
}
