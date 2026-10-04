//! Native generated execution preserves values without the legacy ORM converter.
#![cfg(all(
	feature = "backends",
	feature = "postgres",
	feature = "mysql",
	feature = "sqlite"
))]

use futures::StreamExt;
use reinhardt_db::backends::{DatabaseConnection, DatabaseErrorKind, DatabaseType};
use reinhardt_db::orm::execution::{
	ExecutionError, InsertExecution, QueryExecution, SelectExecution,
};
use reinhardt_db::orm::query::{FieldAssignment, Filter, UpdateValue};
use reinhardt_db::orm::{
	CustomManager, DatabaseConnectionLease, DatabaseValue, FilterOperator, FilterValue,
	ManyToManyAccessor, Model, OrmExecutor, QuerySet, ReverseAccessor,
};
use reinhardt_query::{
	Alias, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryStatementBuilder,
	SqliteQueryBuilder, Value, Values,
};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use testcontainers::runners::AsyncRunner;

fn build<T: QueryStatementBuilder>(database: DatabaseType, statement: T) -> (String, Values) {
	match database {
		DatabaseType::Postgres => statement.build(PostgresQueryBuilder),
		DatabaseType::Mysql => statement.build(MySqlQueryBuilder),
		DatabaseType::Sqlite => statement.build(SqliteQueryBuilder),
	}
}

async fn exercise_crud(connection: &DatabaseConnection) {
	let database = connection.database_type();
	// Arrange: fixture SQL is test-owned; generated runtime DML uses the public path.
	let schema = match database {
		DatabaseType::Postgres => {
			"CREATE TABLE generated_rows (id BIGINT PRIMARY KEY, name TEXT, absent TEXT, uid UUID, amount NUMERIC(30,12))"
		}
		DatabaseType::Mysql => {
			"CREATE TABLE generated_rows (id BIGINT PRIMARY KEY, name TEXT, absent TEXT, uid CHAR(36), amount DECIMAL(30,12))"
		}
		DatabaseType::Sqlite => {
			"CREATE TABLE generated_rows (id BIGINT PRIMARY KEY, name TEXT, absent TEXT, uid TEXT, amount REAL)"
		}
	};
	connection.execute(schema, vec![]).await.unwrap();
	let text = "quoted' ? $1 payload";
	let uuid = uuid::Uuid::parse_str("12345678-1234-5678-9abc-def012345678").unwrap();
	let precise = rust_decimal::Decimal::from_str_exact("123456789012.123456789012").unwrap();
	let amount = if database == DatabaseType::Sqlite {
		Value::Double(Some(1.25))
	} else {
		Value::Decimal(Some(Box::new(precise)))
	};
	let built = build(
		database,
		Query::insert()
			.into_table(Alias::new("generated_rows"))
			.columns([
				Alias::new("id"),
				Alias::new("name"),
				Alias::new("absent"),
				Alias::new("uid"),
				Alias::new("amount"),
			])
			.values_panic([
				Value::Int(Some(7)),
				text.into(),
				Value::String(None),
				Value::Uuid(Some(Box::new(uuid))),
				amount,
			])
			.take(),
	);
	assert_eq!(built.1.0.len(), 4);
	assert!(built.0.contains("NULL"));
	// Act
	assert_eq!(
		connection
			.execute_generated(built, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let select = || {
		build(
			database,
			Query::select()
				.columns([Alias::new("id"), Alias::new("name"), Alias::new("absent")])
				.from(Alias::new("generated_rows"))
				.and_where(Expr::col(Alias::new("name")).eq(text))
				.and_where(Expr::col(Alias::new("uid")).eq(Value::Uuid(Some(Box::new(uuid)))))
				.take(),
		)
	};
	let rows = connection
		.fetch_all_generated(select(), None)
		.await
		.unwrap();
	// Assert: UUID equality still targets legacy text columns on MySQL/SQLite.
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("id").unwrap(), 7);
	assert_eq!(rows[0].get::<String>("name").unwrap(), text);
	assert!(matches!(
		rows[0].data.get("absent"),
		Some(reinhardt_db::backends::QueryValue::Null)
	));
	assert_eq!(
		connection
			.fetch_one_generated(select(), None)
			.await
			.unwrap()
			.get::<String>("name")
			.unwrap(),
		text
	);
	assert!(
		connection
			.fetch_optional_generated(select(), None)
			.await
			.unwrap()
			.is_some()
	);
	// Independently read exact decimal precision through the native SQLx codec.
	match database {
		DatabaseType::Postgres => {
			let stored: rust_decimal::Decimal =
				sqlx::query_scalar("SELECT amount FROM generated_rows")
					.fetch_one(&connection.into_postgres().unwrap())
					.await
					.unwrap();
			assert_eq!(stored, precise);
		}
		DatabaseType::Mysql => {
			let stored: rust_decimal::Decimal =
				sqlx::query_scalar("SELECT amount FROM generated_rows")
					.fetch_one(&connection.into_mysql().unwrap())
					.await
					.unwrap();
			assert_eq!(stored, precise);
		}
		DatabaseType::Sqlite => {}
	}
	let update = build(
		database,
		Query::update()
			.table(Alias::new("generated_rows"))
			.value(Alias::new("name"), "updated' ? $2")
			.and_where(Expr::col(Alias::new("id")).eq(7_i32))
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(update, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	assert!(
		connection
			.fetch_optional_generated(select(), None)
			.await
			.unwrap()
			.is_none()
	);
	let delete = build(
		database,
		Query::delete()
			.from_table(Alias::new("generated_rows"))
			.and_where(Expr::col(Alias::new("id")).eq(7_i32))
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(delete, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let absent = build(
		database,
		Query::select()
			.column(Alias::new("id"))
			.from(Alias::new("generated_rows"))
			.take(),
	);
	assert!(
		connection
			.fetch_all_generated(absent, None)
			.await
			.unwrap()
			.is_empty()
	);
}

fn transaction_insert(database: DatabaseType) -> (String, Values) {
	let amount = if database == DatabaseType::Sqlite {
		Value::Double(Some(1.25))
	} else {
		Value::Decimal(Some(Box::new(
			rust_decimal::Decimal::from_str_exact("123456789012.123456789012").unwrap(),
		)))
	};
	build(
		database,
		Query::insert()
			.into_table(Alias::new("generated_rows"))
			.columns([Alias::new("id"), Alias::new("name"), Alias::new("amount")])
			.values_panic([Value::Int(Some(19)), "transaction' ? $3".into(), amount])
			.take(),
	)
}

fn transaction_select(database: DatabaseType) -> (String, Values) {
	build(
		database,
		Query::select()
			.columns([Alias::new("id"), Alias::new("name")])
			.from(Alias::new("generated_rows"))
			.and_where(Expr::col(Alias::new("id")).eq(19_i32))
			.take(),
	)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct GeneratedModel {
	id: Option<i64>,
	name: String,
}
#[derive(Clone)]
struct GeneratedFields;
impl reinhardt_db::orm::model::FieldSelector for GeneratedFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}
impl Model for GeneratedModel {
	type PrimaryKey = i64;
	type Fields = GeneratedFields;
	type Objects = reinhardt_db::orm::Manager<Self>;
	fn table_name() -> &'static str {
		"generated_rows"
	}
	fn new_fields() -> Self::Fields {
		GeneratedFields
	}
	fn primary_key(&self) -> Option<i64> {
		self.id
	}
	fn set_primary_key(&mut self, value: i64) {
		self.id = Some(value);
	}
	fn field_metadata() -> Vec<reinhardt_db::orm::inspection::FieldInfo> {
		use reinhardt_db::orm::field_codec::DatabaseStorageKind;
		[
			("id", DatabaseStorageKind::I64),
			("name", DatabaseStorageKind::String),
		]
		.into_iter()
		.map(|(name, storage)| reinhardt_db::orm::inspection::FieldInfo {
			name: name.into(),
			field_type: name.into(),
			storage_kind: Some(storage),
			domain: None,
			nullable: name == "id",
			primary_key: name == "id",
			unique: false,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: None,
			choices: None,
			attributes: std::collections::HashMap::new(),
		})
		.collect()
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct GeneratedChild {
	id: Option<i64>,
	parent_id: i64,
	name: String,
}
impl Model for GeneratedChild {
	type PrimaryKey = i64;
	type Fields = GeneratedFields;
	type Objects = reinhardt_db::orm::Manager<Self>;
	fn table_name() -> &'static str {
		"generated_children"
	}
	fn new_fields() -> Self::Fields {
		GeneratedFields
	}
	fn primary_key(&self) -> Option<i64> {
		self.id
	}
	fn set_primary_key(&mut self, value: i64) {
		self.id = Some(value);
	}
}

async fn exercise_relationship_generated_capabilities(connection: &DatabaseConnection) {
	// Arrange: isolated test-owned tables and exact integer/quote-bearing records.
	connection
		.execute(
			"CREATE TABLE generated_children (id BIGINT PRIMARY KEY, parent_id BIGINT, name TEXT)",
			vec![],
		)
		.await
		.unwrap();
	connection.execute(
		"CREATE TABLE generated_rows_members (from_generated_rows_id BIGINT, to_generated_rows_id BIGINT)",
		vec![],
	).await.unwrap();
	let source = GeneratedModel {
		id: Some(41),
		name: "source' ? $9".into(),
	};
	let first = GeneratedModel {
		id: Some(42),
		name: "first' ? $10".into(),
	};
	let second = GeneratedModel {
		id: Some(43),
		name: "second' ? $11".into(),
	};
	for model in [&source, &first, &second] {
		let built = build(
			connection.database_type(),
			Query::insert()
				.into_table("generated_rows")
				.columns(["id", "name"])
				.values_panic([Value::BigInt(model.id), model.name.as_str().into()])
				.take(),
		);
		connection.execute_generated(built, None).await.unwrap();
	}
	let children = [
		GeneratedChild {
			id: Some(51),
			parent_id: 41,
			name: "child' ? $12".into(),
		},
		GeneratedChild {
			id: Some(52),
			parent_id: 41,
			name: "child' ? $13".into(),
		},
	];
	for child in &children {
		let built = build(
			connection.database_type(),
			Query::insert()
				.into_table("generated_children")
				.columns(["id", "parent_id", "name"])
				.values_panic([
					Value::BigInt(child.id),
					child.parent_id.into(),
					child.name.as_str().into(),
				])
				.take(),
		);
		connection.execute_generated(built, None).await.unwrap();
	}
	let lease = DatabaseConnectionLease::register(connection.clone()).unwrap();
	let mut handle = lease.handle();
	// Act / Assert: reverse FK decoding, count and bound pagination retain behavior.
	let reverse = || ReverseAccessor::<GeneratedModel, GeneratedChild>::new(&source, "parent_id");
	let mut loaded = reverse().all_with_conn(&mut handle).await.unwrap();
	loaded.sort_by_key(|child| child.id);
	assert_eq!(loaded, children);
	assert_eq!(reverse().count_with_conn(&mut handle).await.unwrap(), 2);
	let page = reverse()
		.limit(1)
		.offset(1)
		.all_with_conn(&mut handle)
		.await
		.unwrap();
	assert_eq!(page.len(), 1);
	assert!(children.contains(&page[0]));
	assert!(
		reverse()
			.limit(1)
			.offset(2)
			.all_with_conn(&mut handle)
			.await
			.unwrap()
			.is_empty()
	);
	let members = ManyToManyAccessor::<GeneratedModel, GeneratedModel>::new(&source, "members");
	members.add_with_conn(&mut handle, &first).await.unwrap();
	assert!(
		members
			.contains_with_conn(&mut handle, &first)
			.await
			.unwrap()
	);
	assert!(
		!members
			.contains_with_conn(&mut handle, &second)
			.await
			.unwrap()
	);
	assert_eq!(members.count_with_conn(&mut handle).await.unwrap(), 1);
	assert_eq!(
		members.all_with_conn(&mut handle).await.unwrap(),
		std::slice::from_ref(&first)
	);
	let sources = ManyToManyAccessor::<GeneratedModel, GeneratedModel>::filter_by_target_with_conn(
		&GeneratedModel::objects(),
		"members",
		&first,
		&mut handle,
	)
	.await
	.unwrap();
	assert_eq!(sources, std::slice::from_ref(&source));
	let rollback: reinhardt_core::exception::Result<()> = handle
		.atomic_write(async |transaction| {
			members
				.set_with_conn(transaction, std::slice::from_ref(&second))
				.await?;
			assert_eq!(
				members.all_with_conn(transaction).await?,
				std::slice::from_ref(&second)
			);
			Err(reinhardt_core::exception::Error::Conflict(
				"rollback relationship replacement".into(),
			))
		})
		.await;
	assert!(matches!(
		rollback,
		Err(reinhardt_core::exception::Error::Conflict(_))
	));
	assert_eq!(
		members.all_with_conn(&mut handle).await.unwrap(),
		std::slice::from_ref(&first)
	);
	handle
		.atomic_write(async |transaction| {
			members
				.set_with_conn(transaction, &[first.clone(), second.clone()])
				.await
		})
		.await
		.unwrap();
	assert_eq!(members.count_with_conn(&mut handle).await.unwrap(), 2);
	let page = ManyToManyAccessor::<GeneratedModel, GeneratedModel>::new(&source, "members")
		.limit(1)
		.offset(1)
		.all_with_conn(&mut handle)
		.await
		.unwrap();
	assert_eq!(page.len(), 1);
	assert!([first.clone(), second.clone()].contains(&page[0]));
	members.remove_with_conn(&mut handle, &first).await.unwrap();
	assert_eq!(members.all_with_conn(&mut handle).await.unwrap(), [second]);
	members.clear_with_conn(&mut handle).await.unwrap();
	assert_eq!(members.count_with_conn(&mut handle).await.unwrap(), 0);
}

fn generated_id_field() -> reinhardt_db::orm::expressions::FieldRef<
	GeneratedModel,
	Option<i64>,
	reinhardt_db::orm::expressions::GeneratedModelField,
> {
	// SAFETY: id is the declared optional i64 primary key with physical column id.
	unsafe {
		reinhardt_db::orm::expressions::FieldRef::from_generated_model_field_with_names("id", "id")
	}
}

fn generated_name_field() -> reinhardt_db::orm::expressions::FieldRef<
	GeneratedModel,
	String,
	reinhardt_db::orm::expressions::GeneratedModelField,
> {
	// SAFETY: name is the declared String field with physical column name.
	unsafe {
		reinhardt_db::orm::expressions::FieldRef::from_generated_model_field_with_names(
			"name", "name",
		)
	}
}

async fn exercise_orm_generated_capabilities(connection: &DatabaseConnection) {
	let database = connection.database_type();
	// Arrange: the lease owns the registry slot; typed execution receives its handle.
	let lease = DatabaseConnectionLease::register(connection.clone()).unwrap();
	let mut handle = lease.handle();
	let amount = if database == DatabaseType::Sqlite {
		Value::Double(Some(1.25))
	} else {
		Value::Decimal(Some(Box::new(
			rust_decimal::Decimal::from_str_exact("123456789012.123456789012").unwrap(),
		)))
	};
	let insert = InsertExecution::new(
		Query::insert()
			.into_table(Alias::new("generated_rows"))
			.columns([Alias::new("id"), Alias::new("name"), Alias::new("amount")])
			.values_panic([Value::Int(Some(19)), "transaction' ? $3".into(), amount])
			.take(),
	);
	let query = SelectExecution::<GeneratedModel>::new(
		Query::select()
			.columns([Alias::new("id"), Alias::new("name")])
			.from(Alias::new("generated_rows"))
			.and_where(Expr::col(Alias::new("id")).eq(19_i32))
			.take(),
	);
	let expected = GeneratedModel {
		id: Some(19),
		name: "transaction' ? $3".into(),
	};
	// Act / Assert: generic execution uses the native pair and keeps cardinality contracts.
	assert_eq!(
		insert
			.execute_async(&mut handle)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	assert_eq!(
		query.all_async(&mut handle).await.unwrap(),
		vec![expected.clone()]
	);
	assert_eq!(
		query.first_async(&mut handle).await.unwrap(),
		Some(expected.clone())
	);
	assert_eq!(query.one_async(&mut handle).await.unwrap(), expected);
	assert_eq!(
		query.one_or_none_async(&mut handle).await.unwrap(),
		Some(expected.clone())
	);
	assert_eq!(query.get_async(&mut handle, &19).await.unwrap(), expected);
	assert_eq!(query.count_async(&mut handle).await.unwrap(), 1);
	match database {
		DatabaseType::Postgres => assert!(query.exists_async(&mut handle).await.unwrap()),
		DatabaseType::Mysql | DatabaseType::Sqlite => {
			// Workaround: https://github.com/kent8192/reinhardt-web/issues/6530
			// Native integer EXISTS results need backend-aware decoding. Assert the
			// existing error until that repair permits the same boolean assertion.
			let error = query.exists_async(&mut handle).await.unwrap_err();
			assert!(matches!(&error, ExecutionError::Deserialization(_)));
			assert!(error.to_string().contains("expected a boolean"));
		}
	}
	let mut stream =
		OrmExecutor::fetch_stream_generated(&mut handle, transaction_select(database), 1, None)
			.unwrap();
	assert_eq!(
		stream
			.next()
			.await
			.unwrap()
			.unwrap()
			.get::<i64>("id")
			.unwrap(),
		19
	);
	drop(stream);
	let delete = || {
		build(
			database,
			Query::delete()
				.from_table(Alias::new("generated_rows"))
				.and_where(Expr::col(Alias::new("id")).eq(19_i32))
				.take(),
		)
	};
	assert_eq!(
		OrmExecutor::execute_generated(&mut handle, delete(), None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	handle
		.atomic(async |transaction| {
			assert_eq!(
				insert
					.execute_async(transaction)
					.await
					.unwrap()
					.rows_affected,
				1
			);
			let duplicate = OrmExecutor::execute_generated_in_savepoint(
				transaction,
				transaction_insert(database),
				None,
			)
			.await;
			assert!(duplicate.is_err());
			let rows = OrmExecutor::fetch_all_generated_in_savepoint(
				transaction,
				transaction_select(database),
				None,
			)
			.await?;
			assert_eq!(rows.len(), 1);
			assert_eq!(query.one_async(transaction).await.unwrap(), expected);
			Ok::<_, reinhardt_core::exception::Error>(())
		})
		.await
		.unwrap();
	assert_eq!(
		OrmExecutor::execute_generated(&mut handle, delete(), None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	// A resolved stream retains its owner across lease expiry; new operations reject expiry.
	let built = build(
		database,
		Query::select()
			.expr_as(Expr::val(19_i32), Alias::new("id"))
			.take(),
	);
	let mut stream = OrmExecutor::fetch_stream_generated(&mut handle, built, 1, None).unwrap();
	drop(lease);
	assert_eq!(
		stream
			.next()
			.await
			.unwrap()
			.unwrap()
			.get::<i64>("id")
			.unwrap(),
		19
	);
	drop(stream);
	assert_eq!(
		OrmExecutor::fetch_all_generated(&mut handle, transaction_select(database), None)
			.await
			.unwrap_err()
			.database_kind(),
		Some(DatabaseErrorKind::ConnectionHandleExpired)
	);
}

async fn exercise_queryset_generated_capabilities(connection: &DatabaseConnection) {
	let database = connection.database_type();
	// Arrange: punctuation and precise decimal operands must reach the driver unchanged.
	let precise = rust_decimal::Decimal::from_str_exact("123456789012.123456789012").unwrap();
	let amount = if database == DatabaseType::Sqlite {
		Value::Double(Some(1.25))
	} else {
		Value::Decimal(Some(Box::new(precise)))
	};
	connection
		.execute_generated(
			build(
				database,
				Query::insert()
					.into_table(Alias::new("generated_rows"))
					.columns([Alias::new("id"), Alias::new("name"), Alias::new("amount")])
					.values_panic([Value::Int(Some(29)), "queryset' ? $9".into(), amount])
					.take(),
			),
			None,
		)
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(connection.clone()).unwrap();
	let mut handle = lease.handle();
	let by_id = QuerySet::<GeneratedModel>::new().filter(Filter::new(
		"id",
		FilterOperator::Eq,
		FilterValue::Typed(Ok(DatabaseValue::I32(29))),
	));
	let mut query = by_id.clone().filter(Filter::new(
		"name",
		FilterOperator::Eq,
		FilterValue::String("queryset' ? $9".into()),
	));
	if database != DatabaseType::Sqlite {
		query = query.filter(Filter::new(
			"amount",
			FilterOperator::Eq,
			FilterValue::Typed(Ok(DatabaseValue::Decimal(precise))),
		));
	}
	let query = query.order_by(&["id"]).limit(2).offset(0);
	let expected = GeneratedModel {
		id: Some(29),
		name: "queryset' ? $9".into(),
	};
	// Act / Assert: pool and transaction paths preserve filtering and bound pagination.
	assert_eq!(
		query.all_with_db(&mut handle).await.unwrap(),
		vec![expected.clone()]
	);
	assert_eq!(query.rows_with_db(&mut handle).await.unwrap().len(), 1);
	assert_eq!(by_id.count_with_db(&mut handle).await.unwrap(), 1);
	let mut rows = query.iterator_with_db(&mut handle, 1).unwrap();
	assert_eq!(rows.next().await.unwrap().unwrap(), expected);
	assert!(rows.next().await.is_none());
	drop(rows);
	let mut transaction = connection.begin().await.unwrap();
	assert_eq!(
		query.all_with_executor(transaction.as_mut()).await.unwrap(),
		vec![expected.clone()]
	);
	assert_eq!(
		query
			.rows_with_executor(transaction.as_mut())
			.await
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		by_id
			.count_with_executor(transaction.as_mut())
			.await
			.unwrap(),
		1
	);
	let mut rows = query
		.iterator_with_executor(transaction.as_mut(), 1)
		.unwrap();
	assert_eq!(rows.next().await.unwrap().unwrap(), expected);
	drop(rows);
	transaction.rollback().await.unwrap();
	let updated_amount = if database == DatabaseType::Sqlite {
		UpdateValue::Float(2.5)
	} else {
		UpdateValue::Typed(Ok(DatabaseValue::Decimal(
			precise + rust_decimal::Decimal::ONE,
		)))
	};
	assert_eq!(
		by_id
			.update_fields_with_conn(
				&mut handle,
				[
					FieldAssignment::new("name", UpdateValue::String("updated' ? $4".into())),
					FieldAssignment::new("amount", updated_amount),
				]
			)
			.await
			.unwrap(),
		1,
	);
	let mut matching = by_id.clone();
	if database != DatabaseType::Sqlite {
		matching = matching.filter(Filter::new(
			"amount",
			FilterOperator::Eq,
			FilterValue::Typed(Ok(DatabaseValue::Decimal(
				precise + rust_decimal::Decimal::ONE,
			))),
		));
	}
	assert_eq!(
		matching.all_with_db(&mut handle).await.unwrap(),
		vec![GeneratedModel {
			id: Some(29),
			name: "updated' ? $4".into(),
		}]
	);
	assert_eq!(by_id.delete_with_conn(&mut handle).await.unwrap(), 1);
	assert_eq!(by_id.count_with_db(&mut handle).await.unwrap(), 0);
	// Generated upserts retain their cardinality and write-intent lock contracts.
	let (created, inserted) = GeneratedModel::objects()
		.get_or_create()
		.lookup(generated_id_field(), Some(33_i64))
		.default(generated_name_field(), "upsert' ? $5")
		.execute_with(&mut handle)
		.await
		.unwrap();
	assert!(inserted);
	assert_eq!(
		created,
		GeneratedModel {
			id: Some(33),
			name: "upsert' ? $5".into()
		}
	);
	let (existing, inserted) = GeneratedModel::objects()
		.get_or_create()
		.lookup(generated_id_field(), Some(33_i64))
		.default(generated_name_field(), "ignored")
		.execute_with(&mut handle)
		.await
		.unwrap();
	assert!(!inserted);
	assert_eq!(existing, created);
	let (updated, inserted) = handle
		.atomic_write(async |transaction| {
			GeneratedModel::objects()
				.update_or_create()
				.lookup(generated_id_field(), Some(33_i64))
				.set(generated_name_field(), "upsert update' ? $7")
				.execute_with(transaction)
				.await
		})
		.await
		.unwrap();
	assert!(!inserted);
	assert_eq!(
		updated,
		GeneratedModel {
			id: Some(33),
			name: "upsert update' ? $7".into()
		}
	);
	let (created, inserted) = handle
		.atomic_write(async |transaction| {
			GeneratedModel::objects()
				.update_or_create()
				.lookup(generated_id_field(), Some(34_i64))
				.set(generated_name_field(), "upsert create' ? $8")
				.execute_with(transaction)
				.await
		})
		.await
		.unwrap();
	assert!(inserted);
	assert_eq!(
		created,
		GeneratedModel {
			id: Some(34),
			name: "upsert create' ? $8".into()
		}
	);
	let upserts = QuerySet::<GeneratedModel>::new().filter(Filter::new(
		"id",
		FilterOperator::In,
		FilterValue::List(vec![FilterValue::Integer(33), FilterValue::Integer(34)]),
	));
	assert_eq!(upserts.delete_with_conn(&mut handle).await.unwrap(), 2);
}

async fn exercise_transactions_and_streams(connection: &DatabaseConnection) {
	let database = connection.database_type();
	match database {
		DatabaseType::Postgres => {
			connection
				.execute("CREATE TABLE generated_tx_arrays (items INTEGER[])", vec![])
				.await
				.unwrap();
		}
		DatabaseType::Mysql => {
			connection
				.execute(
					"CREATE TABLE generated_tx_unsigned (amount BIGINT UNSIGNED)",
					vec![],
				)
				.await
				.unwrap();
		}
		DatabaseType::Sqlite => {}
	}
	for write_intent in [false, true] {
		// Arrange: cover ordinary SQLx and owned raw write-intent executors.
		let mut transaction = if write_intent {
			connection.begin_write().await.unwrap()
		} else {
			connection.begin().await.unwrap()
		};
		// Act
		match database {
			DatabaseType::Postgres => {
				let built = build(
					database,
					Query::insert()
						.into_table(Alias::new("generated_tx_arrays"))
						.columns([Alias::new("items")])
						.values_panic([Value::Array(
							reinhardt_query::ArrayType::Int,
							Some(Box::new(vec![Value::Int(Some(3)), Value::Int(None)])),
						)])
						.take(),
				);
				assert_eq!(
					transaction
						.execute_generated(built, None)
						.await
						.unwrap()
						.rows_affected,
					1
				);
			}
			DatabaseType::Mysql => {
				let built = build(
					database,
					Query::insert()
						.into_table(Alias::new("generated_tx_unsigned"))
						.columns([Alias::new("amount")])
						.values_panic([u64::MAX])
						.take(),
				);
				assert_eq!(
					transaction
						.execute_generated(built, None)
						.await
						.unwrap()
						.rows_affected,
					1
				);
			}
			DatabaseType::Sqlite => {}
		}
		assert_eq!(
			transaction
				.execute_generated(transaction_insert(database), None)
				.await
				.unwrap()
				.rows_affected,
			1
		);
		let row = transaction
			.fetch_one_generated(transaction_select(database), None)
			.await
			.unwrap();
		assert_eq!(row.get::<String>("name").unwrap(), "transaction' ? $3");
		assert_eq!(
			transaction
				.fetch_all_generated(transaction_select(database), None)
				.await
				.unwrap(),
			vec![row.clone()]
		);
		assert_eq!(
			transaction
				.fetch_optional_generated(transaction_select(database), None)
				.await
				.unwrap(),
			Some(row.clone())
		);
		{
			let mut stream = transaction
				.fetch_stream_generated(transaction_select(database), 2, None)
				.unwrap();
			assert_eq!(stream.next().await.unwrap().unwrap(), row);
			// Drop without exhausting: the same transaction must remain usable.
		}
		assert!(
			transaction
				.fetch_one_generated(transaction_select(database), None)
				.await
				.is_ok()
		);
		transaction.commit().await.unwrap();
		match database {
			DatabaseType::Postgres => {
				let stored: Vec<Option<i32>> =
					sqlx::query_scalar("SELECT items FROM generated_tx_arrays")
						.fetch_one(&connection.into_postgres().unwrap())
						.await
						.unwrap();
				assert_eq!(stored, [Some(3), None]);
				let built = build(
					database,
					Query::delete()
						.from_table(Alias::new("generated_tx_arrays"))
						.take(),
				);
				assert_eq!(
					connection
						.execute_generated(built, None)
						.await
						.unwrap()
						.rows_affected,
					1
				);
				let stored: rust_decimal::Decimal =
					sqlx::query_scalar("SELECT amount FROM generated_rows WHERE id = 19")
						.fetch_one(&connection.into_postgres().unwrap())
						.await
						.unwrap();
				assert_eq!(stored.to_string(), "123456789012.123456789012");
			}
			DatabaseType::Mysql => {
				let stored: u64 = sqlx::query_scalar("SELECT amount FROM generated_tx_unsigned")
					.fetch_one(&connection.into_mysql().unwrap())
					.await
					.unwrap();
				assert_eq!(stored, u64::MAX);
				let built = build(
					database,
					Query::delete()
						.from_table(Alias::new("generated_tx_unsigned"))
						.take(),
				);
				assert_eq!(
					connection
						.execute_generated(built, None)
						.await
						.unwrap()
						.rows_affected,
					1
				);
				let stored: rust_decimal::Decimal =
					sqlx::query_scalar("SELECT amount FROM generated_rows WHERE id = 19")
						.fetch_one(&connection.into_mysql().unwrap())
						.await
						.unwrap();
				assert_eq!(stored.to_string(), "123456789012.123456789012");
			}
			DatabaseType::Sqlite => {}
		}
		// Assert: committed row can stream from the pool; dropping releases its cursor.
		{
			let mut stream = connection
				.fetch_stream_generated(transaction_select(database), 1, None)
				.unwrap();
			assert_eq!(
				stream
					.next()
					.await
					.unwrap()
					.unwrap()
					.get::<i64>("id")
					.unwrap(),
				19
			);
		}
		let delete = || {
			build(
				database,
				Query::delete()
					.from_table(Alias::new("generated_rows"))
					.and_where(Expr::col(Alias::new("id")).eq(19_i32))
					.take(),
			)
		};
		assert_eq!(
			connection
				.execute_generated(delete(), None)
				.await
				.unwrap()
				.rows_affected,
			1
		);
		let mut transaction = if write_intent {
			connection.begin_write().await.unwrap()
		} else {
			connection.begin().await.unwrap()
		};
		transaction
			.execute_generated(transaction_insert(database), None)
			.await
			.unwrap();
		transaction.rollback().await.unwrap();
		assert!(
			connection
				.fetch_optional_generated(transaction_select(database), None)
				.await
				.unwrap()
				.is_none()
		);
		let mut transaction = if write_intent {
			connection.begin_write().await.unwrap()
		} else {
			connection.begin().await.unwrap()
		};
		transaction
			.execute_generated(transaction_insert(database), None)
			.await
			.unwrap();
		drop(transaction);
		assert!(
			connection
				.fetch_optional_generated(transaction_select(database), None)
				.await
				.unwrap()
				.is_none()
		);
	}
	// Invalid streaming inputs fail before a cursor or connection is acquired.
	assert_eq!(
		connection
			.fetch_stream_generated(transaction_select(database), 0, None)
			.err()
			.unwrap()
			.database_kind(),
		Some(DatabaseErrorKind::Configuration)
	);
	if database == DatabaseType::Sqlite {
		let invalid = (
			"private SQL".into(),
			Values(vec![Value::BigUnsigned(Some(u64::MAX))]),
		);
		assert_eq!(
			connection
				.fetch_stream_generated(invalid.clone(), 1, None)
				.err()
				.unwrap()
				.database_kind(),
			Some(DatabaseErrorKind::Type)
		);
		let mut transaction = connection.begin_write().await.unwrap();
		assert_eq!(
			transaction
				.fetch_stream_generated(invalid, 1, None)
				.err()
				.unwrap()
				.database_kind(),
			Some(DatabaseErrorKind::Type)
		);
		transaction.rollback().await.unwrap();
	}
}

#[rstest]
#[tokio::test]
async fn postgres_generated_pool_execution_keeps_decimal_and_nullable_arrays() {
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	exercise_crud(&connection).await;
	exercise_transactions_and_streams(&connection).await;
	exercise_orm_generated_capabilities(&connection).await;
	exercise_queryset_generated_capabilities(&connection).await;
	exercise_relationship_generated_capabilities(&connection).await;
	// Arrange / Act: nullable array elements bypass the legacy Debug conversion.
	connection
		.execute("CREATE TABLE generated_arrays (items INTEGER[])", vec![])
		.await
		.unwrap();
	let built = build(
		DatabaseType::Postgres,
		Query::insert()
			.into_table(Alias::new("generated_arrays"))
			.columns([Alias::new("items")])
			.values_panic([Value::Array(
				reinhardt_query::ArrayType::Int,
				Some(Box::new(vec![Value::Int(Some(3)), Value::Int(None)])),
			)])
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(built, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let row = sqlx::query("SELECT items FROM generated_arrays")
		.fetch_one(&connection.into_postgres().unwrap())
		.await
		.unwrap();
	// Assert
	assert_eq!(
		row.try_get::<Vec<Option<i32>>, _>("items").unwrap(),
		[Some(3), None]
	);
	// Workaround: https://github.com/kent8192/reinhardt-web/issues/6533
	// Independently reproduce SQLx's cache key omitting parameter type identity.
	// Retain this compatibility assertion until a validated driver/framework
	// fix isolates argument signatures; persistent(false) alone cannot do so.
	let pool = connection.into_postgres().unwrap();
	let mut dedicated = pool.acquire().await.unwrap();
	let sql = "SELECT $1::BIGINT AS cached_width";
	let first: i64 = sqlx::query_scalar(sql)
		.bind(7_i32)
		.fetch_one(&mut *dedicated)
		.await
		.unwrap();
	assert_eq!(first, 7);
	let error = sqlx::query_scalar::<_, i64>(sql)
		.bind(8_i64)
		.persistent(false)
		.fetch_one(&mut *dedicated)
		.await
		.unwrap_err();
	assert_eq!(
		error.as_database_error().unwrap().code().as_deref(),
		Some("22P03")
	);
	sqlx::Connection::clear_cached_statements(&mut *dedicated)
		.await
		.unwrap();
	let second: i64 = sqlx::query_scalar(sql)
		.bind(8_i64)
		.persistent(false)
		.fetch_one(&mut *dedicated)
		.await
		.unwrap();
	assert_eq!(second, 8);
}

#[rstest]
#[tokio::test]
async fn mysql_generated_pool_execution_keeps_decimal_and_full_unsigned_range() {
	let container = testcontainers_modules::mysql::Mysql::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"mysql://root@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	let connection = DatabaseConnection::connect_mysql(&url).await.unwrap();
	exercise_crud(&connection).await;
	exercise_transactions_and_streams(&connection).await;
	exercise_orm_generated_capabilities(&connection).await;
	exercise_queryset_generated_capabilities(&connection).await;
	exercise_relationship_generated_capabilities(&connection).await;
	// Arrange / Act: the native backend must bypass the signed compatibility bridge.
	connection
		.execute(
			"CREATE TABLE generated_unsigned (amount BIGINT UNSIGNED)",
			vec![],
		)
		.await
		.unwrap();
	let built = build(
		DatabaseType::Mysql,
		Query::insert()
			.into_table(Alias::new("generated_unsigned"))
			.columns([Alias::new("amount")])
			.values_panic([u64::MAX])
			.take(),
	);
	assert_eq!(
		connection
			.execute_generated(built, None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let stored: u64 = sqlx::query_scalar("SELECT amount FROM generated_unsigned")
		.fetch_one(&connection.into_mysql().unwrap())
		.await
		.unwrap();
	// Assert
	assert_eq!(stored, u64::MAX);
}

#[rstest]
#[tokio::test]
async fn sqlite_generated_pool_execution_keeps_text_uuid_and_rejects_lossy_values() {
	let directory = tempfile::tempdir().unwrap();
	let options = sqlx::sqlite::SqliteConnectOptions::new()
		.filename(directory.path().join("generated.sqlite"))
		.create_if_missing(true);
	let pool = sqlx::sqlite::SqlitePoolOptions::new()
		.max_connections(1)
		.connect_with(options)
		.await
		.unwrap();
	let connection = DatabaseConnection::from_sqlite_pool(pool);
	exercise_crud(&connection).await;
	exercise_transactions_and_streams(&connection).await;
	exercise_orm_generated_capabilities(&connection).await;
	exercise_queryset_generated_capabilities(&connection).await;
	exercise_relationship_generated_capabilities(&connection).await;
	// Act: failed encoding must occur before an invalid SQL statement is executed.
	let built = build(
		DatabaseType::Sqlite,
		Query::select()
			.expr(Expr::val("private payload"))
			.expr(Expr::val(u64::MAX))
			.take(),
	);
	let error = connection
		.fetch_all_generated(built, None)
		.await
		.expect_err("overflow must fail");
	// Assert
	assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
	let message = error.to_string();
	assert!(message.contains("BigUnsigned argument 2 for sqlite"));
	assert!(!message.contains("private payload"));
	assert!(!message.contains(&u64::MAX.to_string()));
}
