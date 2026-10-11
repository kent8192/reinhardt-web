// Supported external usage of the non-exhaustive sequence types: existing
// variants stay constructible, builders populate `SequenceDef`, and matches
// carry a wildcard arm that fails explicitly on unknown values.
use reinhardt_query::types::IntoIden;
use reinhardt_query::types::sequence::{OwnedBy, SequenceDef, SequenceOption, SequenceType};

fn describe_owner(owner: &OwnedBy) -> Result<&'static str, String> {
	match owner {
		OwnedBy::Column { .. } => Ok("column"),
		OwnedBy::SchemaColumn { .. } => Ok("schema column"),
		OwnedBy::None => Ok("none"),
		other => Err(format!("unsupported sequence ownership: {other:?}")),
	}
}

fn describe_option(option: &SequenceOption) -> Result<&'static str, String> {
	match option {
		SequenceOption::Restart(_) => Ok("restart"),
		SequenceOption::IncrementBy(_) => Ok("increment by"),
		SequenceOption::MinValue(_) => Ok("minvalue"),
		SequenceOption::NoMinValue => Ok("no minvalue"),
		SequenceOption::MaxValue(_) => Ok("maxvalue"),
		SequenceOption::NoMaxValue => Ok("no maxvalue"),
		SequenceOption::Cache(_) => Ok("cache"),
		SequenceOption::Cycle => Ok("cycle"),
		SequenceOption::NoCycle => Ok("no cycle"),
		SequenceOption::OwnedBy(owner) => describe_owner(owner),
		SequenceOption::AsType(_) => Ok("as type"),
		SequenceOption::StartWith(_) => Ok("start with"),
		other => Err(format!("unsupported sequence option: {other:?}")),
	}
}

fn main() {
	let owner = OwnedBy::SchemaColumn {
		schema: "public".into_iden(),
		table: "orders".into_iden(),
		column: "id".into_iden(),
	};
	assert_eq!(describe_owner(&owner), Ok("schema column"));

	let options = [
		SequenceOption::Restart(Some(1)),
		SequenceOption::IncrementBy(2),
		SequenceOption::MinValue(1),
		SequenceOption::NoMinValue,
		SequenceOption::MaxValue(100),
		SequenceOption::NoMaxValue,
		SequenceOption::Cache(10),
		SequenceOption::Cycle,
		SequenceOption::NoCycle,
		SequenceOption::OwnedBy(OwnedBy::None),
		SequenceOption::AsType(SequenceType::BigInteger),
		SequenceOption::StartWith(5),
	];
	for option in &options {
		assert!(describe_option(option).is_ok());
	}

	let definition = SequenceDef::new("order_seq")
		.schema("public")
		.as_type(SequenceType::Integer)
		.increment(1)
		.start(10)
		.cache(5)
		.cycle(false)
		.owned_by_schema_column("public", "orders", "id");
	assert!(format!("{definition:?}").contains("order_seq"));
}
