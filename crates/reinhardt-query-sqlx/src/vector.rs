//! SQLx-version-local dense vector argument encoding.

use sqlx::{
	Encode, Postgres, Type, TypeInfo,
	encode::IsNull,
	error::BoxDynError,
	postgres::{PgArgumentBuffer, PgTypeInfo},
};

// Workaround: https://github.com/kent8192/reinhardt-web/issues/6526
// pgvector 0.4.2 can resolve its SQLx codec to 0.9 while this crate uses 0.8.
// Remove this private codec once upstream version-specific features or a
// coordinated SQLx upgrade guarantee compatible traits in fresh consumers.
// The replacement is the upstream pgvector::Vector SQLx argument codec.
pub(crate) struct VectorArgument(Vec<f32>);

impl VectorArgument {
	pub(crate) fn new(values: Vec<f32>) -> Result<Self, &'static str> {
		if values.is_empty() || values.len() > 16_000 {
			return Err("vector dimension is outside the dense-vector range");
		}
		if values.iter().any(|value| !value.is_finite()) {
			return Err("vector contains a non-finite element");
		}
		Ok(Self(values))
	}
}

impl Type<Postgres> for VectorArgument {
	fn type_info() -> PgTypeInfo {
		PgTypeInfo::with_name("vector")
	}
	fn compatible(info: &PgTypeInfo) -> bool {
		info.name().eq_ignore_ascii_case("vector")
	}
}

impl<'query> Encode<'query, Postgres> for VectorArgument {
	fn encode_by_ref(&self, buffer: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
		let dimensions =
			i16::try_from(self.0.len()).expect("validated dense-vector dimensions fit in i16");
		buffer.extend_from_slice(&dimensions.to_be_bytes());
		buffer.extend_from_slice(&0_i16.to_be_bytes());
		for value in &self.0 {
			buffer.extend_from_slice(&value.to_be_bytes());
		}
		Ok(IsNull::No)
	}
	fn size_hint(&self) -> usize {
		4 + size_of_val(self.0.as_slice())
	}
}
