extern crate self as reinhardt;
extern crate self as reinhardt_core;

pub mod macros {
	pub use reinhardt_macros::Model;
}

pub mod model_info {
	pub trait InfoModel {
		type PrimaryKey;
	}

	#[derive(Debug, Clone, PartialEq)]
	pub struct RelationInfo<T: InfoModel> {
		pub id: T::PrimaryKey,
	}

	impl<T: InfoModel> RelationInfo<T> {
		pub const fn new(id: T::PrimaryKey) -> Self {
			Self { id }
		}

		pub fn into_id(self) -> T::PrimaryKey {
			self.id
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	pub struct ManyToManyInfo<Source, Target: InfoModel> {
		pub target_ids: Vec<Target::PrimaryKey>,
		_source: core::marker::PhantomData<Source>,
	}

	impl<Source, Target: InfoModel> ManyToManyInfo<Source, Target> {
		pub fn new<I>(target_ids: I) -> Self
		where
			I: IntoIterator<Item = Target::PrimaryKey>,
		{
			Self {
				target_ids: target_ids.into_iter().collect(),
				_source: core::marker::PhantomData,
			}
		}

		pub const fn empty() -> Self {
			Self {
				target_ids: Vec::new(),
				_source: core::marker::PhantomData,
			}
		}
	}
}

pub mod db {
	pub mod orm {
		pub struct Manager<T>(core::marker::PhantomData<T>);

		pub trait FieldSelector: Sized {
			fn with_alias(self, _alias: &str) -> Self {
				self
			}
		}

		pub trait Model {
			type PrimaryKey;
			type Fields;
			type Objects;

			fn table_name() -> &'static str;
			fn new_fields() -> Self::Fields;
			fn app_label() -> &'static str;
			fn primary_key_field() -> &'static str;
			fn primary_key(&self) -> Option<Self::PrimaryKey>;
			fn set_primary_key(&mut self, value: Self::PrimaryKey);
			fn field_metadata() -> Vec<inspection::FieldInfo>;
			fn index_metadata() -> Vec<inspection::IndexInfo>;
			fn constraint_metadata() -> Vec<inspection::ConstraintInfo>;
			fn relationship_metadata() -> Vec<inspection::RelationInfo>;
		}

		pub mod expressions {
			#[derive(Debug, Clone)]
			pub struct FieldRef<Model, Type> {
				pub name: &'static str,
				_marker: core::marker::PhantomData<(Model, Type)>,
			}

			impl<Model, Type> FieldRef<Model, Type> {
				pub const fn new(name: &'static str) -> Self {
					Self {
						name,
						_marker: core::marker::PhantomData,
					}
				}
			}
		}

		pub mod query_fields {
			#[derive(Debug, Clone)]
			pub struct Field<Model, Type> {
				pub names: Vec<String>,
				pub alias: Option<String>,
				_marker: core::marker::PhantomData<(Model, Type)>,
			}

			impl<Model, Type> Field<Model, Type> {
				pub fn new<S: Into<String>>(names: Vec<S>) -> Self {
					Self {
						names: names.into_iter().map(Into::into).collect(),
						alias: None,
						_marker: core::marker::PhantomData,
					}
				}

				pub fn with_alias(mut self, alias: &str) -> Self {
					self.alias = Some(alias.to_string());
					self
				}
			}
		}

		pub mod fields {
			#[derive(Debug, Clone, PartialEq)]
			pub enum FieldKwarg {
				Bool(bool),
				Int(i64),
				Uint(u64),
				String(String),
			}
		}

		pub mod inspection {
			use super::fields::FieldKwarg;
			use std::collections::HashMap;

			#[derive(Debug, Clone, PartialEq)]
			pub struct FieldInfo {
				pub name: String,
				pub field_type: String,
				pub nullable: bool,
				pub primary_key: bool,
				pub unique: bool,
				pub blank: bool,
				pub editable: bool,
				pub default: Option<String>,
				pub db_default: Option<String>,
				pub db_column: Option<String>,
				pub choices: Option<Vec<String>>,
				pub attributes: HashMap<String, FieldKwarg>,
			}

			#[derive(Debug, Clone, PartialEq)]
			pub struct IndexInfo {
				pub name: String,
				pub fields: Vec<String>,
				pub unique: bool,
				pub condition: Option<String>,
			}

			#[derive(Debug, Clone, PartialEq)]
			pub enum ConstraintType {
				Check,
				Unique,
			}

			#[derive(Debug, Clone, PartialEq)]
			pub struct ConstraintInfo {
				pub name: String,
				pub constraint_type: ConstraintType,
				pub definition: String,
			}

			#[derive(Debug, Clone, PartialEq)]
			pub struct RelationInfo;
		}

		pub mod registry {
			#[derive(Debug, Clone, PartialEq)]
			pub struct ModelInfo {
				pub app_label: String,
				pub model_name: String,
				pub type_path: String,
				pub table_name: String,
			}

			pub struct Registry;

			impl Registry {
				pub fn register(&self, _info: ModelInfo) {}
			}

			pub fn global_model_registry() -> Registry {
				Registry
			}
		}
	}

	pub mod migrations {
		#[derive(Debug, Clone, PartialEq)]
		pub enum FieldType {
			Integer,
			BigInteger,
			VarChar(u32),
			Boolean,
			TimestampTz,
			Date,
			Time,
			Float,
			Double,
			Uuid,
		}

		#[derive(Debug, Clone, PartialEq)]
		pub struct ConstraintDefinition {
			pub name: String,
			pub constraint_type: String,
			pub fields: Vec<String>,
			pub expression: Option<String>,
			pub foreign_key_info: Option<ForeignKeyInfo>,
		}

		#[derive(Debug, Clone, PartialEq)]
		pub struct ForeignKeyInfo {
			pub referenced_table: String,
			pub referenced_column: String,
			pub on_delete: ForeignKeyAction,
			pub on_update: ForeignKeyAction,
		}

		#[derive(Debug, Clone, PartialEq)]
		pub enum ForeignKeyAction {
			Cascade,
		}

		pub fn to_snake_case(value: &str) -> String {
			value.to_ascii_lowercase()
		}

		pub mod model_registry {
			use super::{ConstraintDefinition, FieldType, ForeignKeyInfo};

			#[derive(Debug, Clone, PartialEq)]
			pub struct FieldMetadata {
				pub field_type: FieldType,
			}

			impl FieldMetadata {
				pub const fn new(field_type: FieldType) -> Self {
					Self { field_type }
				}

				pub fn with_param(self, _key: &str, _value: &str) -> Self {
					self
				}

				pub fn with_nullable(self, _nullable: bool) -> Self {
					self
				}

				pub fn with_foreign_key(self, _foreign_key: ForeignKeyInfo) -> Self {
					self
				}
			}

			#[derive(Debug, Clone, PartialEq)]
			pub struct ManyToManyMetadata {
				pub field_name: String,
				pub to_model: String,
				pub related_name: Option<String>,
				pub through: Option<String>,
				pub source_field: Option<String>,
				pub target_field: Option<String>,
				pub db_constraint_prefix: Option<String>,
			}

			pub struct ModelMetadata;

			impl ModelMetadata {
				pub const fn new(_app_label: &str, _model_name: &str, _table_name: &str) -> Self {
					Self
				}

				pub fn add_field(&mut self, _name: String, _metadata: FieldMetadata) {}

				pub fn add_many_to_many(&mut self, _metadata: ManyToManyMetadata) {}

				pub fn add_constraint(&mut self, _constraint: ConstraintDefinition) {}
			}

			pub struct Registry;

			impl Registry {
				pub fn register_model(&self, _metadata: ModelMetadata) {}
			}

			pub fn global_registry() -> Registry {
				Registry
			}
		}
	}
}
