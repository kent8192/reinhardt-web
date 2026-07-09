extern crate self as reinhardt;
extern crate self as reinhardt_core;

pub mod macros {
	pub use reinhardt_macros::Model;
}

pub mod apps {
	pub mod registry {
		#[derive(Debug, Clone, PartialEq, Eq)]
		pub struct RelationshipMetadata {
			pub from_model: &'static str,
			pub to_model: &'static str,
			pub relationship_type: RelationshipType,
			pub field_name: &'static str,
			pub related_name: Option<&'static str>,
			pub db_column: Option<&'static str>,
			pub through_table: Option<&'static str>,
		}

		#[derive(Debug, Clone, Copy, PartialEq, Eq)]
		pub enum RelationshipType {
			ForeignKey,
			ManyToMany,
			OneToOne,
		}

		#[linkme::distributed_slice]
		pub static RELATIONSHIPS: [RelationshipMetadata];
	}
}

pub mod exception {
	#[derive(Debug)]
	pub enum Error {
		Internal(String),
	}

	pub type Result<T> = core::result::Result<T, Error>;
}

pub mod model_info {
	pub trait InfoModel {
		type PrimaryKey;
	}

	#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
	#[serde(bound(
		serialize = "T::PrimaryKey: serde::Serialize",
		deserialize = "T::PrimaryKey: serde::Deserialize<'de>"
	))]
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

	impl<T> Default for RelationInfo<T>
	where
		T: InfoModel,
		T::PrimaryKey: Default,
	{
		fn default() -> Self {
			Self::new(T::PrimaryKey::default())
		}
	}

	#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
	#[serde(bound(
		serialize = "Target::PrimaryKey: serde::Serialize",
		deserialize = "Target::PrimaryKey: serde::Deserialize<'de>"
	))]
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
	pub mod m2m_naming {
		pub fn default_through_table(source_table: &str, field_name: &str) -> String {
			format!("{}_{}", source_table.to_lowercase(), field_name.to_lowercase())
		}

		pub fn default_m2m_columns(source_table: &str, target_table: &str) -> (String, String) {
			let source = source_table.to_lowercase();
			let target = target_table.to_lowercase();
			if source == target {
				(format!("from_{}_id", source), format!("to_{}_id", target))
			} else {
				(format!("{}_id", source), format!("{}_id", target))
			}
		}
	}

	pub mod associations {
		#[derive(Debug, Clone, Copy)]
		pub struct ForeignKeyField<T>(core::marker::PhantomData<T>);

		#[derive(Debug, Clone, Copy)]
		pub struct OneToOneField<T>(core::marker::PhantomData<T>);

		#[derive(Debug, Clone, Copy)]
		pub struct ManyToManyField<Source, Target>(core::marker::PhantomData<(Source, Target)>);

		impl<T> Default for ForeignKeyField<T> {
			fn default() -> Self {
				Self(core::marker::PhantomData)
			}
		}

		impl<T> Default for OneToOneField<T> {
			fn default() -> Self {
				Self(core::marker::PhantomData)
			}
		}

		impl<Source, Target> Default for ManyToManyField<Source, Target> {
			fn default() -> Self {
				Self(core::marker::PhantomData)
			}
		}

		impl<T> PartialEq for ForeignKeyField<T> {
			fn eq(&self, _other: &Self) -> bool {
				true
			}
		}

		impl<T> Eq for ForeignKeyField<T> {}

		impl<T> PartialEq for OneToOneField<T> {
			fn eq(&self, _other: &Self) -> bool {
				true
			}
		}

		impl<T> Eq for OneToOneField<T> {}

		impl<Source, Target> PartialEq for ManyToManyField<Source, Target> {
			fn eq(&self, _other: &Self) -> bool {
				true
			}
		}

		impl<Source, Target> Eq for ManyToManyField<Source, Target> {}
	}

	pub mod orm {
		pub struct Manager<T>(core::marker::PhantomData<T>);

		impl<T> Default for Manager<T> {
			fn default() -> Self {
				Self(core::marker::PhantomData)
			}
		}

		impl<T> Manager<T> {
			pub fn filter(self, _condition: impl Sized) -> Self {
				self
			}

			pub async fn first_with_db(
				self,
				_db: &connection::DatabaseConnection,
			) -> crate::exception::Result<Option<T>> {
				Ok(None)
			}
		}

		pub mod connection {
			#[derive(Debug, Clone)]
			pub struct DatabaseConnection;
		}

		pub trait FieldSelector: Sized {
			fn with_alias(self, _alias: &str) -> Self {
				self
			}
		}

		pub trait Model {
			type PrimaryKey;
			type Fields;
			type Objects: Default;

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

			fn objects() -> Self::Objects
			where
				Self: Sized,
			{
				Self::Objects::default()
			}
		}

		pub trait IntoPrimaryKey<T: Model> {
			fn into_primary_key(self) -> T::PrimaryKey;
		}

		impl<T: Model> IntoPrimaryKey<T> for &T {
			fn into_primary_key(self) -> T::PrimaryKey {
				self.primary_key().unwrap()
			}
		}

		impl<T: Model<PrimaryKey = i64>> IntoPrimaryKey<T> for i64 {
			fn into_primary_key(self) -> T::PrimaryKey {
				self
			}
		}

		pub struct ForeignKeyAccessor<Source, Target> {
			_marker: core::marker::PhantomData<(Source, Target)>,
		}

		impl<Source, Target> ForeignKeyAccessor<Source, Target> {
			pub const fn new(_db_column: &'static str) -> Self {
				Self {
					_marker: core::marker::PhantomData,
				}
			}
		}

		pub struct ManyToManyAccessor<Source, Target> {
			_marker: core::marker::PhantomData<(Source, Target)>,
		}

		impl<Source, Target> ManyToManyAccessor<Source, Target> {
			pub const fn new(
				_source: &Source,
				_field_name: &'static str,
				_db: connection::DatabaseConnection,
			) -> Self {
				Self {
					_marker: core::marker::PhantomData,
				}
			}
		}

		pub mod relationship {
			#[derive(Debug, Clone, Copy, PartialEq, Eq)]
			pub enum RelationshipType {
				OneToOne,
				OneToMany,
				ManyToOne,
				ManyToMany,
			}
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

				pub const fn name(&self) -> &'static str {
					self.name
				}

				pub fn eq(self, _value: impl Into<String>) -> bool {
					true
				}
			}
		}

		pub mod relations {
			use std::borrow::Cow;

			use super::Model;

			#[derive(Debug, Clone, Copy, PartialEq, Eq)]
			pub enum RelationJoinKind {
				Inner,
				Left,
			}

			#[derive(Debug, Clone, PartialEq, Eq)]
			pub struct RelationStep {
				pub name: Cow<'static, str>,
				pub source_table: Cow<'static, str>,
				pub target_table: Cow<'static, str>,
				pub source_column: Cow<'static, str>,
				pub target_column: Cow<'static, str>,
				pub default_join_kind: RelationJoinKind,
			}

			pub trait RelationDescriptor {
				type Source: Model;
				type Target: Model;

				fn steps() -> Vec<RelationStep>;
			}

			pub trait RelationPathLike {
				type Root: Model;
				type Target: Model;

				fn steps(&self) -> &[RelationStep];
				fn join_kind(&self) -> RelationJoinKind;
				fn join_kind_override(&self) -> Option<RelationJoinKind> {
					None
				}
				fn leaf_alias(&self) -> &str;
			}

			pub struct RelationPath<Root: Model, Target: Model> {
				steps: Vec<RelationStep>,
				step_aliases: Vec<String>,
				join_kind_override: Option<RelationJoinKind>,
				_marker: core::marker::PhantomData<(Root, Target)>,
			}

			impl<Root: Model, Target: Model> RelationPath<Root, Target> {
				pub fn from_descriptor<D>() -> Self
				where
					D: RelationDescriptor<Source = Root, Target = Target>,
				{
					let steps = D::steps();
					let step_aliases = step_aliases(&steps);
					Self {
						steps,
						step_aliases,
						join_kind_override: None,
						_marker: core::marker::PhantomData,
					}
				}

				pub fn optional(mut self) -> Self {
					self.join_kind_override = Some(RelationJoinKind::Left);
					self
				}

				pub fn then<D, Next>(self) -> RelationPath<Root, Next>
				where
					D: RelationDescriptor<Source = Target, Target = Next>,
					Next: Model,
				{
					let mut steps = self.steps;
					steps.extend(D::steps());
					let step_aliases = step_aliases(&steps);
					RelationPath {
						steps,
						step_aliases,
						join_kind_override: self.join_kind_override,
						_marker: core::marker::PhantomData,
					}
				}

				pub fn field<Value>(
					self,
					field: super::expressions::FieldRef<Target, Value>,
				) -> RelatedFieldRef<Root, Target, Value> {
					RelatedFieldRef {
						field: field.name(),
						_path: self,
						_marker: core::marker::PhantomData,
					}
				}
			}

			impl<Root: Model, Target: Model> RelationPathLike for RelationPath<Root, Target> {
				type Root = Root;
				type Target = Target;

				fn steps(&self) -> &[RelationStep] {
					&self.steps
				}

				fn join_kind(&self) -> RelationJoinKind {
					self.join_kind_override.unwrap_or(RelationJoinKind::Inner)
				}

				fn join_kind_override(&self) -> Option<RelationJoinKind> {
					self.join_kind_override
				}

				fn leaf_alias(&self) -> &str {
					self.step_aliases
						.last()
						.map(String::as_str)
						.unwrap_or(Target::table_name())
				}
			}

			fn step_aliases(steps: &[RelationStep]) -> Vec<String> {
				let mut aliases = Vec::new();
				let mut source_alias = String::new();
				for (index, step) in steps.iter().enumerate() {
					let alias = if index == 0 {
						step.name.to_string()
					} else {
						format!("{}__{}", source_alias, step.name)
					};
					source_alias = alias.clone();
					aliases.push(alias);
				}
				aliases
			}

			pub struct RelatedFieldRef<Root: Model, Target: Model, Value> {
				field: &'static str,
				_path: RelationPath<Root, Target>,
				_marker: core::marker::PhantomData<Value>,
			}

			impl<Root: Model, Target: Model, Value> RelatedFieldRef<Root, Target, Value> {
				pub fn name(&self) -> &'static str {
					self.field
				}

				pub fn eq(self, _value: impl Into<String>) -> bool {
					true
				}

				pub fn icontains(self, _value: impl Into<String>) -> bool {
					true
				}

				pub fn is_null(self) -> bool {
					true
				}
			}

			pub trait RelationTarget: Model {
				type Path<Root: Model>: RelationPathLike<Root = Root, Target = Self>;

				fn wrap_relation_path<Root: Model>(path: RelationPath<Root, Self>) -> Self::Path<Root>
				where
					Self: Sized;
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
			pub struct RelationInfo {
				pub name: String,
				pub relationship_type: super::relationship::RelationshipType,
				pub foreign_key: Option<String>,
				pub related_model: String,
				pub back_populates: Option<String>,
				pub through_table: Option<String>,
				pub source_field: Option<String>,
				pub target_field: Option<String>,
			}
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
