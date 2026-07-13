use reinhardt_macros::model;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

include!("../support.rs");

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
struct Uuid([u8; 16]);

fn deserialize_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
	D: Deserializer<'de>,
{
	let _ = <std::string::String as Deserialize>::deserialize(deserializer)?;
	Ok(Uuid([0; 16]))
}

mod uuid_serde {
	use super::*;

	pub fn serialize<S>(value: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_bytes(&value.0)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
	where
		D: Deserializer<'de>,
	{
		let _ = <std::string::String as Deserialize>::deserialize(deserializer)?;
		Ok(Uuid([0; 16]))
	}
}

#[model(table_name = "fixture_projection_models")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FixtureProjectionModel {
	#[field(primary_key = true)]
	id: i64,
	#[serde(deserialize_with = "deserialize_uuid")]
	payload: Uuid,
	#[serde(with = "uuid_serde")]
	persisted_payload: Uuid,
}

fn main() {}
