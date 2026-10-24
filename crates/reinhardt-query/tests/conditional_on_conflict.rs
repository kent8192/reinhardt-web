//! Checked conflict capabilities preserve predicates and bound argument order.
use reinhardt_query::{
	CockroachDBQueryBuilder, Expr, ExprTrait, MySqlQueryBuilder, OnConflict, PostgresQueryBuilder,
	Query, QueryBuildError, SqliteQueryBuilder,
};
use rstest::rstest;

#[rstest]
#[case::postgres(false)]
#[case::cockroach(true)]
fn named_constraint_and_condition_preserve_arguments(#[case] cockroach: bool) {
	// Arrange
	let name = "users\"unique";
	let payload = "bound' ? $77";
	let statement = Query::insert()
		.into_table("users")
		.columns(["id", "name"])
		.values_panic([1.into(), reinhardt_query::Value::from(payload)])
		.on_conflict(
			OnConflict::constraint(name)
				.update_columns(["name"])
				.action_and_where(Expr::col("id").gt(10))
				.action_and_where(Expr::col("name").ne(payload)),
		)
		.returning_all()
		.to_owned();
	// Act
	let (sql, values) = if cockroach {
		CockroachDBQueryBuilder::new()
			.build_insert_checked(&statement)
			.unwrap()
	} else {
		PostgresQueryBuilder
			.build_insert_checked(&statement)
			.unwrap()
	};
	// Assert
	assert_eq!(
		sql,
		"INSERT INTO \"users\" (\"id\", \"name\") VALUES ($1, $2) ON CONFLICT ON CONSTRAINT \"users\"\"unique\" DO UPDATE SET \"name\" = EXCLUDED.\"name\" WHERE \"id\" > $3 AND \"name\" <> $4 RETURNING *"
	);
	assert_eq!(
		values.0,
		vec![1.into(), payload.into(), 10.into(), payload.into()]
	);
	assert!(!sql.contains(payload));
}

#[rstest]
fn sqlite_condition_follows_subquery_arguments() {
	// Arrange
	let statement = Query::insert()
		.into_table("users")
		.columns(["id"])
		.from_subquery(Query::select().expr(Expr::val(1)).to_owned())
		.on_conflict(
			OnConflict::column("id")
				.update_columns(["id"])
				.action_and_where(Expr::col("id").lt(20)),
		)
		.returning_all()
		.to_owned();
	// Act
	let (sql, values) = SqliteQueryBuilder.build_insert_checked(&statement).unwrap();
	// Assert
	assert_eq!(
		sql,
		"INSERT INTO \"users\" (\"id\") SELECT ? ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" WHERE \"id\" < ? RETURNING *"
	);
	assert_eq!(values.0, vec![1.into(), 20.into()]);
}

#[rstest]
#[case::mysql(false)]
#[case::sqlite(true)]
fn checked_build_rejects_named_constraint_on_other_backends(#[case] sqlite: bool) {
	// Arrange
	let statement = Query::insert()
		.into_table("users")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(OnConflict::constraint("users_pkey"))
		.to_owned();
	// Act
	let result = if sqlite {
		SqliteQueryBuilder.build_insert_checked(&statement)
	} else {
		MySqlQueryBuilder.build_insert_checked(&statement)
	};
	// Assert
	assert_eq!(
		result.unwrap_err(),
		QueryBuildError::UnsupportedBackendFeature {
			feature: "ON CONFLICT ON CONSTRAINT",
			backend: if sqlite { "SQLite" } else { "MySQL" },
		}
	);
}

#[rstest]
fn checked_mysql_rejects_conditional_update() {
	// Arrange
	let statement = Query::insert()
		.into_table("users")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(
			OnConflict::column("id")
				.update_columns(["id"])
				.action_and_where(Expr::col("id").gt(0)),
		)
		.to_owned();
	// Act
	let error = MySqlQueryBuilder
		.build_insert_checked(&statement)
		.unwrap_err();
	// Assert
	assert_eq!(
		error,
		QueryBuildError::UnsupportedBackendFeature {
			feature: "ON CONFLICT DO UPDATE WHERE",
			backend: "MySQL",
		}
	);
}

#[rstest]
fn do_nothing_clears_condition() {
	// Arrange
	let statement = Query::insert()
		.into_table("users")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(
			OnConflict::column("id")
				.update_columns(["id"])
				.action_and_where(Expr::col("id").gt(0))
				.do_nothing(),
		)
		.to_owned();
	// Act
	let (sql, values) = MySqlQueryBuilder.build_insert_checked(&statement).unwrap();
	// Assert
	assert_eq!(
		sql,
		"INSERT INTO `users` (`id`) VALUES (?) ON DUPLICATE KEY UPDATE `id` = `id`"
	);
	assert_eq!(values.0, vec![1.into()]);
}

#[rstest]
#[case::missing_target(OnConflict::new().update_columns(["id"]), "DO UPDATE requires a conflict target")]
#[case::empty_update(OnConflict::column("id").update_columns(std::iter::empty::<&str>()), "DO UPDATE requires update columns")]
#[case::nothing_condition(OnConflict::column("id").action_and_where(Expr::col("id").gt(0)), "DO NOTHING cannot have an action condition")]
fn checked_invalid_conflict_returns_error(
	#[case] conflict: OnConflict,
	#[case] reason: &'static str,
) {
	// Arrange
	let statement = Query::insert()
		.into_table("users")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(conflict)
		.to_owned();
	// Act
	let error = PostgresQueryBuilder
		.build_insert_checked(&statement)
		.unwrap_err();
	// Assert
	assert_eq!(error, QueryBuildError::InvalidOnConflict { reason });
}

#[rstest]
fn sqlite_insert_select_guard_retains_value_order() {
	// Arrange
	let statement = Query::insert()
		.into_table("users")
		.columns(["name"])
		.from_subquery(
			Query::select()
				.expr(Expr::val("source' ? $9"))
				.from("source")
				.to_owned(),
		)
		.on_conflict(
			OnConflict::column("name")
				.update_columns(["name"])
				.action_and_where(Expr::col("name").ne("blocked' ? $10")),
		)
		.to_owned();
	// Act
	let (sql, values) = SqliteQueryBuilder.build_insert_checked(&statement).unwrap();
	// Assert
	assert_eq!(
		sql,
		"INSERT INTO \"users\" (\"name\") SELECT ? FROM \"source\" WHERE ? ON CONFLICT (\"name\") DO UPDATE SET \"name\" = EXCLUDED.\"name\" WHERE \"name\" <> ?"
	);
	assert_eq!(
		values.0,
		vec!["source' ? $9".into(), true.into(), "blocked' ? $10".into()]
	);
}
