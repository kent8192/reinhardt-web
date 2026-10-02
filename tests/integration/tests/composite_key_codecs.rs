//! Composite primary keys retain field codecs across macro expansion and lookup.

use reinhardt::db::backends::DatabaseConnection as BackendsConnection;
use reinhardt::db::orm::composite_pk::PkValue;
use reinhardt::db::orm::{
	DatabaseConnectionLease, DatabaseField, DatabaseValue, FieldCodecContext, FieldCodecError,
	Model, QuerySet,
};
use reinhardt::{ModelEnum, model};
use reinhardt_query::prelude::{
	ColumnDef, Iden, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::rstest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};
use uuid::Uuid;

#[model(app_label = "composite_codecs", table_name = "uuid_memberships")]
#[derive(Serialize, Deserialize)]
struct UuidMembership {
	#[field(primary_key = true, db_column = "tenant_key")]
	tenant_id: Uuid,
	#[field(primary_key = true)]
	member_id: Uuid,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ModelEnum)]
#[model_enum(repr = "string")]
enum ResourceKind {
	#[default]
	#[model_enum(value = "task_record")]
	#[serde(rename = "public-task")]
	Task,
}

#[model(app_label = "composite_codecs", table_name = "resource_reads")]
#[derive(Serialize, Deserialize)]
struct ResourceRead {
	#[field(primary_key = true, max_length = 64)]
	owner: String,
	#[field(
		primary_key = true,
		field_type = "text",
		max_length = 64,
		db_column = "resource_kind"
	)]
	kind: ResourceKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ModelEnum)]
#[model_enum(repr = "i32")]
enum SequenceKind {
	#[model_enum(value = 17)]
	Task,
}

#[model(app_label = "composite_codecs", table_name = "numbered_resources")]
#[derive(Serialize, Deserialize)]
struct NumberedResource {
	#[field(primary_key = true)]
	owner: i32,
	#[field(primary_key = true)]
	kind: SequenceKind,
}

// Deliberately has no Display implementation and serializes differently from storage.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct TenantCode {
	value: Uuid,
}

impl DatabaseField for TenantCode {
	type Storage = Uuid;

	fn validate_database_context(
		&self,
		context: &FieldCodecContext,
	) -> Result<(), FieldCodecError> {
		if self.value == Uuid::from_u128(99) {
			return Err(FieldCodecError::FieldPolicyMismatch {
				context: Box::new(context.clone()),
				key: "tenant_access".to_owned(),
				expected: "allowed".to_owned(),
				actual: "denied".to_owned(),
			});
		}
		Ok(())
	}

	fn encode_database(&self) -> Result<Self::Storage, FieldCodecError> {
		if self.value.is_nil() {
			return Err(FieldCodecError::Serialization(
				"tenant code is nil".to_owned(),
			));
		}
		Ok(self.value)
	}

	fn decode_database(
		value: Self::Storage,
		_context: &FieldCodecContext,
	) -> Result<Self, FieldCodecError> {
		Ok(Self { value })
	}
}

#[model(app_label = "composite_codecs", table_name = "custom_memberships")]
#[derive(Serialize, Deserialize)]
struct CustomMembership {
	#[field(primary_key = true)]
	tenant: TenantCode,
	#[field(primary_key = true)]
	member_id: Uuid,
}

#[model(app_label = "composite_codecs", table_name = "optional_memberships")]
#[derive(Serialize, Deserialize)]
struct OptionalMembership {
	#[field(primary_key = true)]
	tenant_id: Option<Uuid>,
	#[field(primary_key = true)]
	member_id: Option<Uuid>,
}

#[rstest]
fn uuid_components_retain_native_database_values() {
	// Arrange
	let tenant_id = Uuid::from_u128(1);
	let member_id = Uuid::from_u128(2);
	let model = UuidMembership {
		tenant_id,
		member_id,
	};

	// Act
	let values = model.get_composite_pk_values().unwrap();

	// Assert
	assert_eq!(values.len(), 2);
	assert_eq!(
		values["tenant_id"],
		PkValue::Database {
			value: DatabaseValue::Uuid(tenant_id)
		}
	);
	assert_eq!(
		values["member_id"],
		PkValue::Database {
			value: DatabaseValue::Uuid(member_id)
		}
	);
	assert_eq!(
		model.primary_key().unwrap().to_string(),
		format!("(v2;tenant_id=36:{tenant_id}, member_id=36:{member_id})"),
	);
}

#[rstest]
fn enum_components_use_storage_values_without_display_or_serde_labels() {
	// Arrange
	let key = ResourceReadCompositePk::new("alice".to_owned(), ResourceKind::Task);

	// Act
	let values = key.to_pk_values().unwrap();

	// Assert
	assert_eq!(values["kind"], PkValue::String("task_record".to_owned()));
	assert_eq!(key.to_string(), "(v2;owner=5:alice, kind=11:task_record)");
	assert_eq!(
		serde_json::to_value(ResourceKind::Task).unwrap(),
		"public-task"
	);
}

#[rstest]
fn integer_enum_and_scalar_components_keep_i32_storage() {
	// Arrange
	let key = NumberedResourceCompositePk::new(5, SequenceKind::Task);

	// Act
	let values = key.to_pk_values().unwrap();

	// Assert
	assert_eq!(
		values["owner"],
		PkValue::Database {
			value: DatabaseValue::I32(5)
		}
	);
	assert_eq!(
		values["kind"],
		PkValue::Database {
			value: DatabaseValue::I32(17)
		}
	);
	assert_eq!(key.to_string(), "(v2;owner=1:5, kind=2:17)");
}

#[rstest]
fn custom_database_fields_retain_storage_without_display() {
	// Arrange
	let value = Uuid::from_u128(3);
	let key = CustomMembershipCompositePk::new(TenantCode { value }, Uuid::from_u128(4));

	// Act
	let values = key.to_pk_values().unwrap();

	// Assert
	assert_eq!(
		values["tenant"],
		PkValue::Database {
			value: DatabaseValue::Uuid(value)
		}
	);
	assert_eq!(
		key.to_string(),
		format!(
			"(v2;tenant=36:{value}, member_id=36:{})",
			Uuid::from_u128(4)
		),
	);
}

#[rstest]
fn custom_codec_failures_propagate_without_panicking_or_querying() {
	// Arrange
	let model = CustomMembership {
		tenant: TenantCode { value: Uuid::nil() },
		member_id: Uuid::from_u128(4),
	};

	// Act
	let result = model.get_composite_pk_values();

	// Assert
	assert_eq!(
		result,
		Err(FieldCodecError::Serialization(
			"tenant code is nil".to_owned()
		))
	);
}

#[rstest]
#[case(Uuid::nil())]
#[case(Uuid::from_u128(99))]
fn invalid_composite_keys_display_without_panicking(#[case] value: Uuid) {
	// Arrange
	let key = CustomMembershipCompositePk::new(TenantCode { value }, Uuid::from_u128(4));

	// Act
	let text = key.to_string();

	// Assert
	assert_eq!(text, "<invalid composite primary key>");
}

#[rstest]
fn composite_key_encoding_honors_custom_field_context_policy() {
	// Arrange
	let key = CustomMembershipCompositePk::new(
		TenantCode {
			value: Uuid::from_u128(99),
		},
		Uuid::from_u128(4),
	);

	// Act
	let result = key.to_pk_values();

	// Assert
	assert_eq!(
		result,
		Err(FieldCodecError::FieldPolicyMismatch {
			context: Box::new(FieldCodecContext::new(
				"CustomMembership",
				"tenant",
				"tenant"
			)),
			key: "tenant_access".to_owned(),
			expected: "allowed".to_owned(),
			actual: "denied".to_owned(),
		}),
	);
}

#[rstest]
fn incomplete_optional_key_returns_an_empty_map() {
	// Arrange
	let model = OptionalMembership {
		tenant_id: None,
		member_id: Some(Uuid::from_u128(2)),
	};

	// Act
	let values = model.get_composite_pk_values().unwrap();

	// Assert
	assert_eq!(values, std::collections::HashMap::new());
	assert_eq!(model.primary_key(), None);
}

#[derive(Debug, Iden)]
enum UuidMemberships {
	Table,
	TenantKey,
	MemberId,
}

#[derive(Debug, Iden)]
enum ResourceReads {
	Table,
	Owner,
	ResourceKind,
}

#[derive(Debug, Iden)]
enum NumberedResources {
	Table,
	Owner,
	Kind,
}

#[derive(Debug, Iden)]
enum CustomMemberships {
	Table,
	Tenant,
	MemberId,
}

#[rstest]
#[tokio::test]
async fn postgres_composite_lookups_preserve_uuid_enum_and_custom_codecs(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	// Arrange: caller-owned connections and container guards isolate all state.
	let (_container, pool, _port, url) = postgres_container.await;
	let owner = BackendsConnection::connect(&url).await.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let mut connection = lease.handle();
	let tenant_id = Uuid::from_u128(1);
	let member_id = Uuid::from_u128(2);
	let uuid_schema = Query::create_table()
		.table(UuidMemberships::Table.into_iden())
		.col(
			ColumnDef::new(UuidMemberships::TenantKey)
				.uuid()
				.not_null(true),
		)
		.col(
			ColumnDef::new(UuidMemberships::MemberId)
				.uuid()
				.not_null(true),
		)
		.primary_key([UuidMemberships::TenantKey, UuidMemberships::MemberId])
		.to_string(PostgresQueryBuilder);
	let enum_schema = Query::create_table()
		.table(ResourceReads::Table.into_iden())
		.col(ColumnDef::new(ResourceReads::Owner).string().not_null(true))
		.col(
			ColumnDef::new(ResourceReads::ResourceKind)
				.string()
				.not_null(true),
		)
		.primary_key([ResourceReads::Owner, ResourceReads::ResourceKind])
		.to_string(PostgresQueryBuilder);
	let integer_schema = Query::create_table()
		.table(NumberedResources::Table.into_iden())
		.col(
			ColumnDef::new(NumberedResources::Owner)
				.integer()
				.not_null(true),
		)
		.col(
			ColumnDef::new(NumberedResources::Kind)
				.integer()
				.not_null(true),
		)
		.primary_key([NumberedResources::Owner, NumberedResources::Kind])
		.to_string(PostgresQueryBuilder);
	let custom_schema = Query::create_table()
		.table(CustomMemberships::Table.into_iden())
		.col(
			ColumnDef::new(CustomMemberships::Tenant)
				.uuid()
				.not_null(true),
		)
		.col(
			ColumnDef::new(CustomMemberships::MemberId)
				.uuid()
				.not_null(true),
		)
		.primary_key([CustomMemberships::Tenant, CustomMemberships::MemberId])
		.to_string(PostgresQueryBuilder);
	for schema in [uuid_schema, enum_schema, integer_schema, custom_schema] {
		sqlx::query(&schema).execute(pool.as_ref()).await.unwrap();
	}
	// Literal inserts make stored values independent of the codec under test.
	let uuid_insert = Query::insert()
		.into_table(UuidMemberships::Table.into_iden())
		.columns([UuidMemberships::TenantKey, UuidMemberships::MemberId])
		.values_panic([tenant_id, member_id])
		.to_string(PostgresQueryBuilder);
	let enum_insert = Query::insert()
		.into_table(ResourceReads::Table.into_iden())
		.columns([ResourceReads::Owner, ResourceReads::ResourceKind])
		.values_panic(["alice", "task_record"])
		.to_string(PostgresQueryBuilder);
	let integer_insert = Query::insert()
		.into_table(NumberedResources::Table.into_iden())
		.columns([NumberedResources::Owner, NumberedResources::Kind])
		.values_panic([5, 17])
		.to_string(PostgresQueryBuilder);
	let custom_insert = Query::insert()
		.into_table(CustomMemberships::Table.into_iden())
		.columns([CustomMemberships::Tenant, CustomMemberships::MemberId])
		.values_panic([tenant_id, member_id])
		.to_string(PostgresQueryBuilder);
	for insert in [uuid_insert, enum_insert, integer_insert, custom_insert] {
		sqlx::query(&insert).execute(pool.as_ref()).await.unwrap();
	}

	// Act
	let membership = QuerySet::<UuidMembership>::new()
		.get_composite_with_db(
			&mut connection,
			&UuidMembershipCompositePk::new(tenant_id, member_id)
				.to_pk_values()
				.unwrap(),
		)
		.await
		.unwrap();
	let resource = QuerySet::<ResourceRead>::new()
		.get_composite_with_db(
			&mut connection,
			&ResourceReadCompositePk::new("alice".to_owned(), ResourceKind::Task)
				.to_pk_values()
				.unwrap(),
		)
		.await
		.unwrap();
	let numbered = QuerySet::<NumberedResource>::new()
		.get_composite_with_db(
			&mut connection,
			&NumberedResourceCompositePk::new(5, SequenceKind::Task)
				.to_pk_values()
				.unwrap(),
		)
		.await
		.unwrap();
	let custom = QuerySet::<CustomMembership>::new()
		.get_composite_with_db(
			&mut connection,
			&CustomMembershipCompositePk::new(TenantCode { value: tenant_id }, member_id)
				.to_pk_values()
				.unwrap(),
		)
		.await
		.unwrap();

	// Assert: aliases, typed binding and typed hydration all survive the lookup.
	assert_eq!(
		membership,
		UuidMembership {
			tenant_id,
			member_id
		}
	);
	assert_eq!(
		resource,
		ResourceRead {
			owner: "alice".to_owned(),
			kind: ResourceKind::Task
		}
	);
	assert_eq!(
		numbered,
		NumberedResource {
			owner: 5,
			kind: SequenceKind::Task
		}
	);
	assert_eq!(
		custom,
		CustomMembership {
			tenant: TenantCode { value: tenant_id },
			member_id
		}
	);
}
