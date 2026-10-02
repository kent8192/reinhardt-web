extern crate self as reinhardt_db;

struct Option;
struct Result;
struct Some;
struct None;
struct Ok;
struct Err;
struct String;

// This macro must remain unused when the derive emits a hygienic std::vec! path.
#[allow(unused_macros)]
macro_rules! vec {
	($($token:tt)*) => {
		compile_error!("generated code used an unqualified vec! macro")
	};
}

pub mod orm {
	#[derive(Clone)]
	pub struct FieldCodecContext;

	pub struct FieldCodecError;

	impl FieldCodecError {
		pub fn invalid_enum(
			_context: FieldCodecContext,
			_repr: ModelEnumRepr,
			_value: ModelEnumValue,
		) -> Self {
			Self
		}
	}

	pub enum ModelEnumRepr {
		String,
		I32,
	}

	pub enum ModelEnumValue {
		String(::std::string::String),
		I32(i32),
	}

	pub enum ModelEnumValueRef {
		String(&'static str),
		I32(i32),
	}

	pub enum FieldDomain {
		Enum {
			repr: ModelEnumRepr,
			values: ::std::vec::Vec<ModelEnumValue>,
		},
	}

	pub trait DatabaseField: Sized {
		type Storage;
		const MAX_STRING_VALUE_CHARS: ::core::option::Option<usize>;

		fn encode_database(&self) -> ::core::result::Result<Self::Storage, FieldCodecError>;
		fn decode_database(
			value: Self::Storage,
			context: &FieldCodecContext,
		) -> ::core::result::Result<Self, FieldCodecError>;
		fn domain() -> ::core::option::Option<FieldDomain>;
	}

	pub trait ModelEnum: DatabaseField {
		const REPR: ModelEnumRepr;
		const VALUES: &'static [ModelEnumValueRef];
	}
}

#[derive(reinhardt_macros::ModelEnum)]
#[model_enum(repr = "string")]
enum Status {
	#[model_enum(value = "queued")]
	Queued,
}

fn main() {}
