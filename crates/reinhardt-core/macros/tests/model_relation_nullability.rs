//! Relation companion types must follow nullability independently of model forms.

// Model derives emit the framework's cfg(native) in this standalone test crate.
#![allow(unexpected_cfgs)]

use reinhardt_macros::model;
use rstest::rstest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

include!("ui/model/support.rs");

macro_rules! relation_nullability_tests {
	($module:ident, $app_label:literal $(, $($form:tt)+)?) => {
		mod $module {
			use super::*;

			#[model(app_label = $app_label)]
			#[derive(Serialize, Deserialize)]
			struct Parent {
				#[field(primary_key = true)]
				id: Uuid,
			}

			#[model(app_label = $app_label $(, $($form)+)?)]
			#[derive(Serialize, Deserialize)]
			struct Child {
				#[field(primary_key = true)]
				id: Uuid,
				#[field(max_length = 64)]
				label: String,
				#[rel(foreign_key, null = true)]
				optional_parent: db::associations::ForeignKeyField<Parent>,
				#[rel(foreign_key)]
				required_parent: db::associations::ForeignKeyField<Parent>,
				#[rel(foreign_key, null = false, db_constraint = "fk_not_null = true")]
				explicit_required_parent: db::associations::ForeignKeyField<Parent>,
				#[rel(one_to_one, null = true)]
				optional_profile: db::associations::OneToOneField<Parent>,
				#[rel(one_to_one)]
				required_profile: db::associations::OneToOneField<Parent>,
				#[rel(one_to_one, null = false)]
				explicit_required_profile: db::associations::OneToOneField<Parent>,
			}

			#[rstest::fixture]
			fn required_fields() -> serde_json::Value {
				serde_json::json!({
					"id": Uuid::from_u128(1),
					"label": "child",
					"required_parent_id": Uuid::from_u128(2),
					"explicit_required_parent_id": Uuid::from_u128(2),
					"required_profile_id": Uuid::from_u128(2),
					"explicit_required_profile_id": Uuid::from_u128(2),
				})
			}

			#[rstest]
			#[case::null(None)]
			#[case::present(Some(Uuid::from_u128(3)))]
			fn relation_ids_preserve_types_and_round_trip(
				mut required_fields: serde_json::Value,
				#[case] expected: Option<Uuid>,
			) {
				// Arrange
				required_fields["optional_parent_id"] = serde_json::json!(expected);
				required_fields["optional_profile_id"] = serde_json::json!(expected);

				// Act
				let child: Child = serde_json::from_value(required_fields.clone()).unwrap();
				let optional_ids: (Option<Uuid>, Option<Uuid>) =
					(child.optional_parent_id, child.optional_profile_id);
				let required_ids: (Uuid, Uuid, Uuid, Uuid) = (
					child.required_parent_id,
					child.explicit_required_parent_id,
					child.required_profile_id,
					child.explicit_required_profile_id,
				);

				// Assert
				assert_eq!(optional_ids, (expected, expected));
				let parent_id = Uuid::from_u128(2);
				assert_eq!(required_ids, (parent_id, parent_id, parent_id, parent_id));
				assert_eq!(serde_json::to_value(child).unwrap(), required_fields);
			}

			#[rstest]
			fn omitted_nullable_relation_ids_default_to_none(required_fields: serde_json::Value) {
				// Act
				let child: Child = serde_json::from_value(required_fields).unwrap();

				// Assert
				assert_eq!(child.optional_parent_id, None);
				assert_eq!(child.optional_profile_id, None);
			}

			#[rstest]
			#[case("required_parent_id")]
			#[case("explicit_required_parent_id")]
			#[case("required_profile_id")]
			#[case("explicit_required_profile_id")]
			fn required_relation_ids_reject_null(
				mut required_fields: serde_json::Value,
				#[case] field: &str,
			) {
				// Arrange
				required_fields[field] = serde_json::Value::Null;

				// Act
				let error = serde_json::from_value::<Child>(required_fields).unwrap_err();

				// Assert
				assert_eq!(error.classify(), serde_json::error::Category::Data);
			}
		}
	};
}

relation_nullability_tests!(without_forms, "nullability_plain");
relation_nullability_tests!(forms_disabled, "nullability_disabled", form = false);
relation_nullability_tests!(with_forms, "nullability_forms", form = true);
relation_nullability_tests!(
	with_named_form,
	"nullability_named",
	form(name = CreateChild, fields(label))
);
