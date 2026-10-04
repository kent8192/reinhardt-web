//! Regression coverage for SQL parameter inlining.

use reinhardt_query::query::traits::inline_params;
use reinhardt_query::types::{TriggerEvent, TriggerScope, TriggerTiming};
use reinhardt_query::{
	Alias, ArrayType, ColumnDef, ColumnType, Expr, ExprTrait, MySqlQueryBuilder,
	PostgresQueryBuilder, Query, QueryBuilderTrait, QueryStatementBuilder, SelectStatement,
	SimpleExpr, SqliteQueryBuilder, Value, Values,
};
use rstest::{fixture, rstest};

#[rstest]
#[case::reported_bytes(Some(vec![0x00, 0x01, 0xff]), r"E'\\x0001ff'::bytea")]
#[case::empty(Some(vec![]), r"E'\\x'::bytea")]
#[case::sql_sensitive_bytes(Some(vec![0x00, 0x27, 0x5c, 0x80, 0xff]), r"E'\\x00275c80ff'::bytea")]
#[case::null(None, "NULL")]
fn postgres_to_string_renders_bytea_values(#[case] bytes: Option<Vec<u8>>, #[case] literal: &str) {
	// Arrange
	let value = Value::Bytes(bytes.map(Box::new));
	let query = Query::select()
		.expr(Expr::value(value.clone()))
		.expr(SimpleExpr::CustomWithExpr(
			"(?)".into(),
			vec![Expr::value(value.clone()).into()],
		))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);
	let after = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(inlined, format!("SELECT {literal}, ({literal})"));
	if value.is_null() {
		assert_eq!(sql, "SELECT NULL, (NULL)");
		assert_eq!(values, Values::new());
	} else {
		assert_eq!(sql, "SELECT $1, ($2)");
		assert_eq!(values, Values(vec![value.clone(), value]));
	}
	assert_eq!(after, (sql, values));
}

#[rstest]
#[case::mysql(MySqlQueryBuilder)]
#[case::sqlite(SqliteQueryBuilder)]
fn to_string_preserves_other_backend_byte_literals(#[case] builder: impl QueryBuilderTrait) {
	// Arrange
	let query = Query::select()
		.expr(Expr::value(vec![0x00_u8, 0x01, 0xff]))
		.to_owned();

	// Act
	let inlined = query.to_string(builder);

	// Assert
	assert_eq!(inlined, "SELECT X'0001FF'");
}

#[rstest]
fn inline_params_renders_postgres_bytea_values() {
	// Arrange
	let values = Values(vec![vec![0x00_u8, 0x01, 0xff].into(), Value::Bytes(None)]);

	// Act
	let inlined = inline_params("SELECT $1, $2, $1", &values);

	// Assert
	assert_eq!(
		inlined,
		r"SELECT E'\\x0001ff'::bytea, NULL, E'\\x0001ff'::bytea"
	);
}

#[rstest]
#[case::reported_bytes(Some(vec![0x00, 0x01, 0xff]), "X'0001FF'")]
#[case::empty(Some(vec![]), "X''")]
#[case::sql_sensitive_bytes(Some(vec![0x00, 0x27, 0x5c, 0x80, 0xff]), "X'00275C80FF'")]
#[case::null(None, "NULL")]
fn inline_params_preserves_positional_byte_literals(
	#[case] bytes: Option<Vec<u8>>,
	#[case] literal: &str,
) {
	// Arrange
	let values = Values(vec![Value::Bytes(bytes.map(Box::new)), Value::Bytes(None)]);

	// Act
	let inlined = inline_params("SELECT ?, ?, ?", &values);

	// Assert
	assert_eq!(inlined, format!("SELECT {literal}, NULL, ?"));
}

#[rstest]
#[case::quoted("SELECT '$1', ?", "SELECT '$1', X'0001FF'")]
#[case::comment("SELECT /* $1 */ ?", "SELECT /* $1 */ X'0001FF'")]
#[case::dollar_quote("SELECT $$ $1 $$, ?", "SELECT $$ $1 $$, X'0001FF'")]
#[case::identifier("SELECT price$1, ?", "SELECT price$1, X'0001FF'")]
#[case::numbered_precedence("SELECT ?, $1", r"SELECT ?, E'\\x0001ff'::bytea")]
fn inline_params_infers_byte_literal_syntax_from_placeholder_tokens(
	#[case] sql: &str,
	#[case] expected: &str,
) {
	// Arrange
	let values = Values(vec![vec![0x00_u8, 0x01, 0xff].into()]);

	// Act
	let inlined = inline_params(sql, &values);

	// Assert
	assert_eq!(inlined, expected);
}

#[rstest]
fn postgres_to_string_propagates_bytea_rendering_to_subqueries() {
	// Arrange
	let inner = Query::select()
		.expr(Expr::cust("$1"))
		.expr(Expr::value(vec![0x00_u8, 0x01, 0xff]))
		.to_owned();
	let query = Query::select().expr(Expr::subquery(inner)).to_owned();

	// Act
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(inlined, r"SELECT (SELECT $1, E'\\x0001ff'::bytea)");
}

#[rstest]
fn postgres_to_string_renders_bytea_defaults_and_checks() {
	// Arrange
	let bytes = vec![0x00_u8, 0x01, 0xff];
	let query = Query::create_table()
		.table("records")
		.col(
			ColumnDef::new("value")
				.blob()
				.default(Expr::value(bytes.clone()).into())
				.check(Expr::col("value").eq(Expr::value(bytes))),
		)
		.to_owned();

	// Act
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		inlined,
		r#"CREATE TABLE "records" ("value" BYTEA DEFAULT E'\\x0001ff'::bytea CHECK ("value" = E'\\x0001ff'::bytea))"#
	);
}

#[rstest]
#[case::mixed_bytes(
	ArrayType::Bytes,
	Some(vec![vec![0x00_u8, 0xff].into(), Vec::<u8>::new().into(), Value::Bytes(None)]),
	r"ARRAY[E'\\x00ff'::bytea,E'\\x'::bytea,NULL]::bytea[]"
)]
#[case::empty_bytes(ArrayType::Bytes, Some(vec![]), "ARRAY[]::bytea[]")]
#[case::null_bytes(ArrayType::Bytes, Some(vec![Value::Bytes(None)]), "ARRAY[NULL]::bytea[]")]
#[case::null_array(ArrayType::Bytes, None, "NULL")]
#[case::other_elements(ArrayType::Int, Some(vec![7_i32.into(), Value::Int(None)]), "ARRAY[7,NULL]")]
#[case::nested_bytes(
	ArrayType::Bytes,
	Some(vec![
		Value::Array(ArrayType::Bytes, Some(Box::new(vec![vec![0x00_u8].into()]))),
		Value::Array(ArrayType::Bytes, Some(Box::new(vec![vec![0xff_u8].into()]))),
	]),
	r"ARRAY[ARRAY[E'\\x00'::bytea]::bytea[],ARRAY[E'\\xff'::bytea]::bytea[]]::bytea[]"
)]
fn postgres_to_string_renders_array_elements_with_backend_literals(
	#[case] array_type: ArrayType,
	#[case] elements: Option<Vec<Value>>,
	#[case] literal: &str,
) {
	// Arrange
	let value = Value::Array(array_type, elements.map(Box::new));
	let inner = Query::select().expr(Expr::value(value.clone())).to_owned();
	let query = Query::select()
		.expr(Expr::value(value.clone()))
		.expr(SimpleExpr::CustomWithExpr(
			"(?)".into(),
			vec![Expr::value(value.clone()).into()],
		))
		.expr(Expr::subquery(inner))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);
	let helper_inlined = inline_params("SELECT $1", &Values(vec![value.clone()]));
	let after = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		inlined,
		format!("SELECT {literal}, ({literal}), (SELECT {literal})")
	);
	assert_eq!(helper_inlined, format!("SELECT {literal}"));
	if value.is_null() {
		assert_eq!(sql, "SELECT NULL, (NULL), (SELECT NULL)");
		assert_eq!(values, Values::new());
	} else {
		assert_eq!(sql, "SELECT $1, ($2), (SELECT $3)");
		assert_eq!(values, Values(vec![value.clone(), value.clone(), value]));
	}
	assert_eq!(after, (sql, values));
}

#[rstest]
fn inline_params_preserves_generic_byte_array_literals_for_positional_placeholders() {
	// Arrange
	let value = Value::Array(
		ArrayType::Bytes,
		Some(Box::new(vec![
			vec![0x00_u8, 0xff].into(),
			Value::Bytes(None),
		])),
	);

	// Act
	let inlined = inline_params("SELECT ?", &Values(vec![value.clone()]));
	let generic = value.to_sql_literal();

	// Assert
	assert_eq!(inlined, "SELECT ARRAY[X'00FF',NULL]");
	assert_eq!(generic, "ARRAY[X'00FF',NULL]");
}

#[rstest]
fn postgres_to_string_renders_byte_array_defaults_and_checks() {
	// Arrange
	let value = Value::Array(
		ArrayType::Bytes,
		Some(Box::new(vec![
			vec![0x00_u8, 0xff].into(),
			Value::Bytes(None),
		])),
	);
	let query = Query::create_table()
		.table("records")
		.col(
			ColumnDef::new("value")
				.array(ColumnType::Blob)
				.default(Expr::value(value.clone()).into())
				.check(Expr::col("value").eq(Expr::value(value))),
		)
		.to_owned();

	// Act
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		inlined,
		r#"CREATE TABLE "records" ("value" BYTEA[] DEFAULT ARRAY[E'\\x00ff'::bytea,NULL]::bytea[] CHECK ("value" = ARRAY[E'\\x00ff'::bytea,NULL]::bytea[]))"#
	);
}

#[rstest]
#[case::mysql(
	MySqlQueryBuilder,
	"SELECT `title` LIKE ? ESCAPE 0x5C, `content` LIKE ? ESCAPE 0x5C, `suffix` LIKE ? ESCAPE 0x5C",
	"SELECT `title` LIKE '%web%' ESCAPE 0x5C, `content` LIKE 'guide%' ESCAPE 0x5C, `suffix` LIKE '%.md' ESCAPE 0x5C"
)]
#[case::postgres(
	PostgresQueryBuilder,
	r#"SELECT "title" LIKE $1 ESCAPE '\', "content" LIKE $2 ESCAPE '\', "suffix" LIKE $3 ESCAPE '\'"#,
	r#"SELECT "title" LIKE '%web%' ESCAPE '\', "content" LIKE 'guide%' ESCAPE '\', "suffix" LIKE '%.md' ESCAPE '\'"#
)]
#[case::sqlite(
	SqliteQueryBuilder,
	r#"SELECT "title" LIKE ? ESCAPE '\', "content" LIKE ? ESCAPE '\', "suffix" LIKE ? ESCAPE '\'"#,
	r#"SELECT "title" LIKE '%web%' ESCAPE '\', "content" LIKE 'guide%' ESCAPE '\', "suffix" LIKE '%.md' ESCAPE '\'"#
)]
fn to_string_inlines_values_after_like_escape_clauses(
	#[case] builder: impl QueryBuilderTrait + Clone,
	#[case] expected_build: &str,
	#[case] expected_inline: &str,
) {
	// Arrange
	let query = Query::select()
		.expr(Expr::col("title").contains("web"))
		.expr(Expr::col("content").starts_with("guide"))
		.expr(Expr::col("suffix").ends_with(".md"))
		.to_owned();

	// Act
	let (sql, values) = query.build(builder.clone());
	let inlined = query.to_string(builder);

	// Assert
	assert_eq!(inlined, expected_inline);
	assert_eq!(sql, expected_build);
	assert_eq!(
		values,
		Values(vec!["%web%".into(), "guide%".into(), "%.md".into()])
	);
}

#[rstest]
fn postgres_to_string_preserves_raw_bind_marker() {
	// Arrange
	let query = Query::select()
		.column(Alias::new("id"))
		.from(Alias::new("records"))
		.and_where(Expr::col(Alias::new("id")).eq(Expr::cust("$1")))
		.and_where(Expr::col(Alias::new("enabled")).eq(true))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "records" WHERE "id" = $1 AND "enabled" = $1"#
	);
	assert_eq!(values, Values(vec![true.into()]));
	assert_eq!(
		inlined,
		r#"SELECT "id" FROM "records" WHERE "id" = $1 AND "enabled" = TRUE"#
	);
}

#[rstest]
#[case::first_index("$1")]
#[case::later_index("$2")]
#[case::multi_digit("$10")]
#[case::repeated("$1 + $1")]
#[case::out_of_order("$2 + $1")]
#[case::cast("$1::uuid")]
#[case::unicode("COALESCE($1, '価格')")]
#[case::quoted_and_commented("COALESCE($1, '$1') /* $1 */")]
fn postgres_to_string_preserves_raw_fragments_around_values(#[case] raw: &str) {
	// Arrange
	let query = Query::select()
		.expr(Expr::cust(raw))
		.expr(Expr::val(7))
		.expr(Expr::val("it's $1, $2, ?"))
		.expr(Expr::cust(raw))
		.expr(Expr::val(None::<i32>))
		.limit(10)
		.offset(2)
		.to_owned();

	// Act
	let before = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);
	let after = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		inlined,
		format!("SELECT {raw}, 7, 'it''s $1, $2, ?', {raw}, NULL LIMIT 10 OFFSET 2")
	);
	assert_eq!(after, before);
}

#[rstest]
fn postgres_to_string_distinguishes_raw_markers_in_custom_templates() {
	// Arrange
	let query = Query::select()
		.expr(Expr::val(11))
		.expr(Expr::cust_with_values("$1 + ? + $2 + ?", [7, 13]))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, "SELECT $1, $1 + $2 + $2 + $3");
	assert_eq!(values, Values(vec![11.into(), 7.into(), 13.into()]));
	assert_eq!(inlined, "SELECT 11, $1 + 7 + $2 + 13");
}

#[fixture]
fn raw_bind_select() -> SelectStatement {
	Query::select()
		.expr(Expr::cust("$1"))
		.expr(Expr::val(7))
		.to_owned()
}

#[rstest]
#[case::scalar(
	|inner| Query::select().expr(Expr::val(11)).expr(Expr::subquery(inner)).to_owned(),
	"SELECT 11, (SELECT $1, 7)"
)]
#[case::from(
	|inner| Query::select().expr(Expr::val(11)).from_subquery(inner, Alias::new("source")).to_owned(),
	r#"SELECT 11 FROM (SELECT $1, 7) AS "source""#
)]
#[case::lateral(
	|inner| {
		use reinhardt_query::{IntoIden, TableRef};

		Query::select()
			.expr(Expr::val(11))
			.from(TableRef::LateralSubQuery(Box::new(inner), Alias::new("source").into_iden()))
			.to_owned()
	},
	r#"SELECT 11 FROM LATERAL (SELECT $1, 7) AS "source""#
)]
#[case::cte(
	|inner| Query::select().with_cte(Alias::new("source"), inner).expr(Expr::val(11)).from(Alias::new("source")).to_owned(),
	r#"WITH "source" AS (SELECT $1, 7) SELECT 11 FROM "source""#
)]
#[case::union(
	|inner| Query::select().expr(Expr::val(11)).union_all(inner).to_owned(),
	"SELECT 11 UNION ALL SELECT $1, 7"
)]
fn postgres_to_string_preserves_raw_markers_in_nested_queries(
	raw_bind_select: SelectStatement,
	#[case] wrap: fn(SelectStatement) -> SelectStatement,
	#[case] expected: &str,
) {
	// Arrange
	let query = wrap(raw_bind_select);

	// Act
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(inlined, expected);
}

#[rstest]
fn postgres_to_string_preserves_raw_select_without_values() {
	// Arrange
	let inner = SelectStatement::raw("SELECT $1");
	let query = Query::select()
		.expr(Expr::val(7))
		.expr(Expr::subquery(inner.clone()))
		.to_owned();

	// Act
	let raw = inner.to_string(PostgresQueryBuilder);
	let nested = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(raw, "SELECT $1");
	assert_eq!(nested, "SELECT 7, (SELECT $1)");
}

#[rstest]
fn postgres_to_string_preserves_raw_markers_in_mutations(raw_bind_select: SelectStatement) {
	// Arrange
	let insert = Query::insert()
		.into_table(Alias::new("records"))
		.columns([Alias::new("id"), Alias::new("score")])
		.from_subquery(raw_bind_select)
		.to_owned();
	let update = Query::update()
		.table(Alias::new("records"))
		.value(Alias::new("score"), 7)
		.and_where(Expr::col(Alias::new("id")).eq(Expr::cust("$1")))
		.to_owned();
	let delete = Query::delete()
		.from_table(Alias::new("records"))
		.and_where(Expr::col(Alias::new("id")).eq(Expr::cust("$1")))
		.and_where(Expr::col(Alias::new("score")).eq(7))
		.to_owned();

	// Act
	let insert_sql = insert.to_string(PostgresQueryBuilder);
	let update_sql = update.to_string(PostgresQueryBuilder);
	let delete_sql = delete.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		insert_sql,
		r#"INSERT INTO "records" ("id", "score") SELECT $1, 7"#
	);
	assert_eq!(
		update_sql,
		r#"UPDATE "records" SET "score" = 7 WHERE "id" = $1"#
	);
	assert_eq!(
		delete_sql,
		r#"DELETE FROM "records" WHERE "id" = $1 AND "score" = 7"#
	);
}

#[rstest]
fn postgres_to_string_preserves_raw_markers_in_views(raw_bind_select: SelectStatement) {
	// Arrange
	let view = Query::create_view()
		.name(Alias::new("records"))
		.as_select(raw_bind_select.clone())
		.to_owned();
	let materialized = Query::create_materialized_view()
		.name(Alias::new("records"))
		.as_select(raw_bind_select)
		.to_owned();

	// Act
	let view_sql = view.to_string(PostgresQueryBuilder);
	let materialized_sql = materialized.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(view_sql, r#"CREATE VIEW "records" AS SELECT $1, 7"#);
	assert_eq!(
		materialized_sql,
		r#"CREATE MATERIALIZED VIEW "records" AS SELECT $1, 7"#
	);
}

#[rstest]
fn postgres_to_string_preserves_raw_markers_in_table_defaults() {
	// Arrange
	let column = ColumnDef::new(Alias::new("score"))
		.integer()
		.default(Expr::cust_with_values("COALESCE($1, ?)", [7]).into());
	let create = Query::create_table()
		.table(Alias::new("records"))
		.col(column.clone())
		.to_owned();
	let alter = Query::alter_table()
		.table(Alias::new("records"))
		.add_column(column)
		.to_owned();

	// Act
	let create_sql = create.to_string(PostgresQueryBuilder);
	let alter_sql = alter.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		create_sql,
		r#"CREATE TABLE "records" ("score" INTEGER DEFAULT COALESCE($1, 7))"#
	);
	assert_eq!(
		alter_sql,
		r#"ALTER TABLE "records" ADD COLUMN "score" INTEGER DEFAULT COALESCE($1, 7)"#
	);
}

#[rstest]
fn postgres_to_string_preserves_raw_markers_in_schema_conditions() {
	// Arrange
	let condition = Expr::cust_with_values("$1 > ?", [7]).into_simple_expr();
	let index = Query::create_index()
		.name(Alias::new("records_score"))
		.table(Alias::new("records"))
		.col(Alias::new("score"))
		.r#where(condition.clone())
		.to_owned();
	let trigger = Query::create_trigger()
		.name(Alias::new("records_score"))
		.timing(TriggerTiming::Before)
		.event(TriggerEvent::Update { columns: None })
		.on_table(Alias::new("records"))
		.for_each(TriggerScope::Row)
		.when_condition(condition)
		.execute_function("check_score")
		.to_owned();

	// Act
	let index_sql = index.to_string(PostgresQueryBuilder);
	let trigger_sql = trigger.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		index_sql,
		r#"CREATE INDEX "records_score" ON "records" ("score") WHERE $1 > 7"#
	);
	assert_eq!(
		trigger_sql,
		r#"CREATE TRIGGER "records_score" BEFORE UPDATE ON "records" FOR EACH ROW WHEN ($1 > 7) EXECUTE FUNCTION "check_score"()"#
	);
}

#[rstest]
#[case::exact(
	"$1",
	r#"SELECT "$1" FROM "probe" WHERE "id" = $1"#,
	r#"SELECT "$1" FROM "probe" WHERE "id" = 7"#
)]
#[case::embedded(
	"price$1",
	r#"SELECT "price$1" FROM "probe" WHERE "id" = $1"#,
	r#"SELECT "price$1" FROM "probe" WHERE "id" = 7"#
)]
#[case::longer_index(
	"$10",
	r#"SELECT "$10" FROM "probe" WHERE "id" = $1"#,
	r#"SELECT "$10" FROM "probe" WHERE "id" = 7"#
)]
#[case::escaped_quote(
	"a\"$1",
	r#"SELECT "a""$1" FROM "probe" WHERE "id" = $1"#,
	r#"SELECT "a""$1" FROM "probe" WHERE "id" = 7"#
)]
#[case::unicode(
	"価格$1",
	r#"SELECT "価格$1" FROM "probe" WHERE "id" = $1"#,
	r#"SELECT "価格$1" FROM "probe" WHERE "id" = 7"#
)]
fn postgres_to_string_preserves_quoted_identifiers(
	#[case] identifier: &str,
	#[case] expected_sql: &str,
	#[case] expected_inline: &str,
) {
	// Arrange
	let query = Query::select()
		.column(Alias::new(identifier))
		.from(Alias::new("probe"))
		.and_where(Expr::col(Alias::new("id")).eq(7_i32))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, Values(vec![Value::Int(Some(7))]));
	assert_eq!(inlined, expected_inline);
}

#[rstest]
fn postgres_to_string_handles_multi_digit_parameters() {
	// Arrange
	let mut query = Query::select();
	for value in 1_i32..=12 {
		query.expr(Expr::val(value));
	}

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		"SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12"
	);
	assert_eq!(
		values,
		Values((1..=12).map(|n| Value::Int(Some(n))).collect())
	);
	assert_eq!(inlined, "SELECT 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12");
}

#[rstest]
#[case::mysql(
	MySqlQueryBuilder,
	"SELECT `$1`, `?` FROM `$2?` WHERE `id` = ?",
	"SELECT `$1`, `?` FROM `$2?` WHERE `id` = 7"
)]
#[case::sqlite(
	SqliteQueryBuilder,
	r#"SELECT "$1", "?" FROM "$2?" WHERE "id" = ?"#,
	r#"SELECT "$1", "?" FROM "$2?" WHERE "id" = 7"#
)]
fn positional_to_string_ignores_quoted_placeholder_text(
	#[case] builder: impl QueryBuilderTrait + Clone,
	#[case] expected_sql: &str,
	#[case] expected_inline: &str,
) {
	// Arrange
	let query = Query::select()
		.columns([Alias::new("$1"), Alias::new("?")])
		.from(Alias::new("$2?"))
		.and_where(Expr::col(Alias::new("id")).eq(7_i32))
		.to_owned();

	// Act
	let (sql, values) = query.build(builder.clone());
	let inlined = query.to_string(builder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, Values(vec![Value::Int(Some(7))]));
	assert_eq!(inlined, expected_inline);
}

#[fixture]
fn numeric_values() -> Values {
	Values(vec![Value::Int(Some(7)), Value::Int(Some(11))])
}

#[rstest]
#[case::single_quoted("SELECT '$1', 'it''s $2', $1", "SELECT '$1', 'it''s $2', 7")]
#[case::escape_string(r"SELECT E'it\'s $1', $1", r"SELECT E'it\'s $1', 7")]
#[case::standard_string(r"SELECT '\', $1", r"SELECT '\', 7")]
#[case::dollar_quoted("SELECT $$ $1 ' \" $$, $1", "SELECT $$ $1 ' \" $$, 7")]
#[case::tagged_dollar_quoted("SELECT $tag$ $1 $$ $tag$, $1", "SELECT $tag$ $1 $$ $tag$, 7")]
#[case::unicode_dollar_tag("SELECT $タグ$ $1 $タグ$, $1", "SELECT $タグ$ $1 $タグ$, 7")]
#[case::line_comment("SELECT -- $1\n$1", "SELECT -- $1\n7")]
#[case::block_comment("SELECT /* $1 /* $2 */ $1 */ $1", "SELECT /* $1 /* $2 */ $1 */ 7")]
#[case::unquoted_identifier("SELECT price$1, 価格$2, $1", "SELECT price$1, 価格$2, 7")]
#[case::repeated_out_of_order("SELECT $2, $1, $2", "SELECT 11, 7, 11")]
#[case::missing_index_one("SELECT $2", "SELECT 11")]
#[case::unbound(
	"SELECT $0, $3, $10, $184467440737095516160, $1",
	"SELECT $0, $3, $10, $184467440737095516160, 7"
)]
#[case::json_operator("SELECT payload ? 'key', $1", "SELECT payload ? 'key', 7")]
#[case::positional("SELECT '?', ?, /* ? $1 */ ?", "SELECT '?', 7, /* ? $1 */ 11")]
#[case::extra_positional("SELECT ?, ?, ?", "SELECT 7, 11, ?")]
#[case::unclosed_quote("SELECT $1, '$2", "SELECT 7, '$2")]
#[case::unclosed_dollar_quote("SELECT $1, $tag$ $2", "SELECT 7, $tag$ $2")]
#[case::unclosed_comment("SELECT $1 /* $2", "SELECT 7 /* $2")]
fn inline_params_preserves_sql_tokens(
	numeric_values: Values,
	#[case] sql: &str,
	#[case] expected: &str,
) {
	let inlined = inline_params(sql, &numeric_values);
	assert_eq!(inlined, expected);
}

#[rstest]
fn inline_params_does_not_rescan_inserted_values() {
	// Arrange
	let values = Values(vec![7_i32.into(), "it's $1, $2, ?".into()]);

	// Act
	let inlined = inline_params("SELECT $2, $1, $2", &values);

	// Assert
	assert_eq!(inlined, "SELECT 'it''s $1, $2, ?', 7, 'it''s $1, $2, ?'");
}

#[rstest]
fn inline_params_without_values_preserves_sql() {
	let sql = r#"SELECT "$1", '$2', ?, $1"#;
	assert_eq!(inline_params(sql, &Values::new()), sql);
}

#[rstest]
fn mysql_to_string_inlines_after_two_minus_operators() {
	let query = Query::select()
		.expr(Expr::cust_with_values("1--?", [7]))
		.to_owned();
	assert_eq!(query.to_string(MySqlQueryBuilder), "SELECT 1--7");
}

#[rstest]
#[case::mysql_space(MySqlQueryBuilder, "1-- ?\n2", "SELECT 1-- ?\n2, 7")]
#[case::mysql_tab(MySqlQueryBuilder, "1--\t?\n2", "SELECT 1--\t?\n2, 7")]
#[case::mysql_control(MySqlQueryBuilder, "1--\u{000b}?\n2", "SELECT 1--\u{000b}?\n2, 7")]
#[case::sqlite_comment(SqliteQueryBuilder, "1--?\n2", "SELECT 1--?\n2, 7")]
#[case::postgres_comment(PostgresQueryBuilder, "1--?\n2", "SELECT 1--?\n2, 7")]
fn to_string_respects_backend_line_comments(
	#[case] builder: impl QueryBuilderTrait,
	#[case] expression: &str,
	#[case] expected: &str,
) {
	// Arrange
	let query = Query::select()
		.expr(Expr::cust(expression))
		.expr(Expr::val(7))
		.to_owned();

	// Act
	let inlined = query.to_string(builder);

	// Assert
	assert_eq!(inlined, expected);
}

#[rstest]
fn to_string_respects_downstream_build_override() {
	// Arrange
	#[derive(Debug)]
	struct CustomStatement;
	impl QueryStatementBuilder for CustomStatement {
		fn build_any(&self, _: &dyn QueryBuilderTrait) -> (String, Values) {
			("SELECT ?".to_owned(), Values(vec![11_i32.into()]))
		}
		fn build<T: QueryBuilderTrait>(&self, _: T) -> (String, Values) {
			("SELECT ?".to_owned(), Values(vec![7_i32.into()]))
		}
	}

	// Act
	let inlined = CustomStatement.to_string(MySqlQueryBuilder);

	// Assert
	assert_eq!(inlined, "SELECT 7");
	assert_eq!(
		CustomStatement.build(MySqlQueryBuilder).1,
		Values(vec![7_i32.into()])
	);
}

#[rstest]
#[case::mysql_numbered_identifier(MySqlQueryBuilder, "$2", "SELECT $2, 7")]
#[case::sqlite_numbered_text(SqliteQueryBuilder, "$2", "SELECT $2, 7")]
#[case::mysql_dollar_identifier(MySqlQueryBuilder, "$tag$", "SELECT $tag$, 7")]
#[case::sqlite_dollar_text(SqliteQueryBuilder, "$tag$", "SELECT $tag$, 7")]
#[case::postgres_backtick_operator(PostgresQueryBuilder, "lhs ` rhs", "SELECT lhs ` rhs, 7")]
#[case::sqlite_backtick_identifier(SqliteQueryBuilder, "`$1?`", "SELECT `$1?`, 7")]
#[case::mysql_block_comment(MySqlQueryBuilder, "1 /* /* */", "SELECT 1 /* /* */, 7")]
#[case::sqlite_block_comment(SqliteQueryBuilder, "1 /* /* */", "SELECT 1 /* /* */, 7")]
#[case::postgres_nested_comment(
	PostgresQueryBuilder,
	"1 /* /* */ $9 */",
	"SELECT 1 /* /* */ $9 */, 7"
)]
#[case::postgres_dollar_quote(
	PostgresQueryBuilder,
	"$tag$ ? $1 $tag$",
	"SELECT $tag$ ? $1 $tag$, 7"
)]
#[case::mysql_double_quote(MySqlQueryBuilder, r#""a\"b""#, r#"SELECT "a\"b", 7"#)]
#[case::mysql_single_quote(MySqlQueryBuilder, r"'a\'b'", r"SELECT 'a\'b', 7")]
#[case::sqlite_standard_escape(SqliteQueryBuilder, r"'\'", r"SELECT '\', 7")]
#[case::mysql_hash_comment(MySqlQueryBuilder, "1 # ? $1\n", "SELECT 1 # ? $1\n, 7")]
#[case::mysql_short_version_comment(
	MySqlQueryBuilder,
	"1 /*!1234 + ? */",
	"SELECT 1 /*!1234 + ? */, 7"
)]
fn to_string_uses_backend_lexical_rules(
	#[case] builder: impl QueryBuilderTrait,
	#[case] expression: &str,
	#[case] expected: &str,
) {
	// Arrange
	let query = Query::select()
		.expr(Expr::cust(expression))
		.expr(Expr::val(7))
		.to_owned();

	// Act
	let inlined = query.to_string(builder);

	// Assert
	assert_eq!(inlined, expected);
}

#[rstest]
#[case::versioned("1 /*!80000 + ? */", "SELECT 1 /*!80000 + 7 */")]
#[case::unversioned("1 /*! + ? */", "SELECT 1 /*! + 7 */")]
#[case::following_value("1 /*!80000 + ? */ + ?", "SELECT 1 /*!80000 + 7 */ + 11")]
fn mysql_to_string_inlines_executable_comments(#[case] expression: &str, #[case] expected: &str) {
	// Arrange
	let query = Query::select()
		.expr(Expr::cust_with_values(expression, [7, 11]))
		.to_owned();

	// Act
	let inlined = query.to_string(MySqlQueryBuilder);

	// Assert
	assert_eq!(inlined, expected);
}
