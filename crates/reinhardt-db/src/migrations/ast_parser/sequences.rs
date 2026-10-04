//! Strict reconstruction of framework-owned sequence builders; never evaluates Rust.
use super::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub(super) fn parse<T: DeserializeOwned>(expression: &Expr, context: &str) -> Result<T> {
	let value = expression_value(expression, context)?;
	serde_json::from_value(value)
		.map_err(|error| MigrationError::InvalidMigration(format!("{context}: {error}")))
}
fn expression_value(expression: &Expr, context: &str) -> Result<Value> {
	if let Some(value) = extract_string_expr(expression) {
		return Ok(Value::String(value));
	}
	if let Some(value) = parse_i64_expression(expression) {
		return Ok(json!(value));
	}
	if let Some(value) = parse_bool_expression(expression) {
		return Ok(json!(value));
	}
	match expression {
		Expr::Path(path) => {
			let names: Vec<_> = path
				.path
				.segments
				.iter()
				.map(|segment| segment.ident.to_string())
				.collect();
			if names.last().is_some_and(|name| name == "None") {
				return Ok(Value::Null);
			}
			if names.len() >= 2 {
				let owner = &names[names.len() - 2];
				let variant = &names[names.len() - 1];
				let permitted = match owner.as_str() {
					"SequenceDataType" => {
						["SmallInteger", "Integer", "BigInteger"].contains(&variant.as_str())
					}
					"SequenceBound" => variant == "Default",
					"IdentityGeneration" => ["Always", "ByDefault"].contains(&variant.as_str()),
					_ => false,
				};
				if permitted {
					return Ok(Value::String(variant.clone()));
				}
			}
		}
		Expr::Call(call) => {
			let Expr::Path(path) = &*call.func else {
				return Err(strict_payload_error(context, "constructor"));
			};
			let names: Vec<_> = path
				.path
				.segments
				.iter()
				.map(|segment| segment.ident.to_string())
				.collect();
			if names.last().is_some_and(|name| name == "Some") && call.args.len() == 1 {
				return expression_value(&call.args[0], context);
			}
			if names.len() < 2 {
				return Err(strict_payload_error(context, "constructor"));
			}
			let owner = &names[names.len() - 2];
			let method = &names[names.len() - 1];
			if owner == "SequenceBound" && method == "Value" && call.args.len() == 1 {
				return Ok(json!({"Value": expression_value(&call.args[0], context)?}));
			}
			if method != "new" {
				return Err(strict_payload_error(context, "constructor"));
			}
			let (fields, mut value): (&[&str], Value) = match owner.as_str() {
				"QualifiedName" => (&["name"], json!({"schema": null})),
				"SequenceKey" => (&["app_label", "logical_name"], json!({})),
				"SequenceOptions" => (
					&[],
					serde_json::to_value(super::super::SequenceOptions::new())
						.expect("serializable options"),
				),
				"SequenceOwner" => (&["table", "column"], json!({})),
				"SequenceDefinition" => (
					&["key", "name"],
					json!({"options": serde_json::to_value(super::super::SequenceOptions::new()).expect("serializable options"), "owned_by": null}),
				),
				"SequenceDefault" => (&["key", "name"], json!({})),
				"IdentityDefinition" => (
					&["generation"],
					json!({"sequence_name": null, "options": serde_json::to_value(super::super::SequenceOptions::new()).expect("serializable options")}),
				),
				"IdentityOperation" => (
					&["table", "column", "field_type", "old", "new"],
					json!({"old_field_type": null}),
				),
				_ => return Err(strict_payload_error(context, "constructor")),
			};
			if fields.len() != call.args.len() {
				return Err(strict_payload_error(context, "constructor arguments"));
			}
			for (field, argument) in fields.iter().zip(call.args.iter()) {
				value[*field] = if *field == "field_type" {
					serde_json::to_value(
						parse_field_type_strict(argument)
							.ok_or_else(|| strict_payload_error(context, field))?,
					)
					.expect("serializable field type")
				} else {
					expression_value(argument, context)?
				};
			}
			return Ok(value);
		}
		Expr::MethodCall(call) => {
			let mut value = expression_value(&call.receiver, context)?;
			if call.args.len() != 1 {
				return Err(strict_payload_error(context, "builder arguments"));
			}
			let method = call.method.to_string();
			let Some(field) = method.strip_prefix("with_") else {
				return Err(strict_payload_error(context, &method));
			};
			// Only methods on the concrete framework builder are accepted.
			let allowed: &[&str] = if value.get("field_type").is_some() {
				&["old_field_type"]
			} else if value.get("logical_name").is_some() || value.get("column").is_some() {
				&[]
			} else if value.get("generation").is_some() {
				&["sequence_name", "options"]
			} else if value.get("key").is_some() && value.get("options").is_some() {
				&["options", "owned_by"]
			} else if value.get("increment").is_some() {
				&[
					"data_type",
					"increment",
					"min_value",
					"max_value",
					"start",
					"cache",
					"cycle",
				]
			} else if value.get("schema").is_some() {
				&["schema"]
			} else {
				&[]
			};
			if !allowed.contains(&field) {
				return Err(strict_payload_error(context, &method));
			}
			if !value
				.as_object()
				.is_some_and(|object| object.contains_key(field))
			{
				return Err(strict_payload_error(context, &method));
			}
			value[field] = if field == "old_field_type" {
				serde_json::to_value(
					parse_field_type_strict(&call.args[0])
						.ok_or_else(|| strict_payload_error(context, field))?,
				)
				.expect("serializable field type")
			} else {
				expression_value(&call.args[0], context)?
			};
			return Ok(value);
		}
		Expr::Struct(operation) => {
			let names: Vec<_> = operation
				.path
				.segments
				.iter()
				.map(|segment| segment.ident.to_string())
				.collect();
			if names.len() < 2 || names[names.len() - 2] != "SequenceOperation" {
				return Err(strict_payload_error(context, "operation type"));
			}
			let kind = &names[names.len() - 1];
			let fields: &[&str] = match kind.as_str() {
				"Create" | "Drop" => &["definition"],
				"Alter" | "RenameDeclaration" => &["old", "new"],
				"Rename" => &["key", "old", "new"],
				"Ownership" => &["key", "name", "old", "new"],
				"Restart" => &["name", "value", "reverse_value"],
				_ => return Err(strict_payload_error(context, "sequence operation")),
			};
			validate_exact_named_fields(&operation.fields, fields, context)?;
			if operation.rest.is_some() {
				return Err(strict_payload_error(context, "struct update"));
			}
			let mut value = json!({"kind": kind});
			for field in fields {
				let expression = strict_field_expression(&operation.fields, field)
					.ok_or_else(|| strict_payload_error(context, field))?;
				value[*field] = expression_value(expression, context)?;
			}
			return Ok(value);
		}
		_ => {}
	}
	Err(strict_payload_error(context, "sequence expression"))
}
