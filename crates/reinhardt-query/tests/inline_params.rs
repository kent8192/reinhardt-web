//! Regression coverage for SQL parameter inlining.

use reinhardt_query::query::traits::inline_params;
use reinhardt_query::{
	Alias, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuilderTrait,
	QueryStatementBuilder, SqliteQueryBuilder, Value, Values,
};
use rstest::{fixture, rstest};

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
