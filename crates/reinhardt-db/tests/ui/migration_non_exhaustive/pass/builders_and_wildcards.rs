// Supported external usage of the non-exhaustive migration types: builders
// populate state, introspection, and registry metadata; known enum variants stay
// constructible; matches carry a wildcard arm that fails explicitly.
use reinhardt_db::migrations::introspection::{ColumnInfo, DatabaseSchema, TableInfo};
use reinhardt_db::migrations::operations::OperationStatement;
use reinhardt_db::migrations::{
	Constraint, FieldMetadata, FieldState, FieldType, IndexType, ModelMetadata, ModelState,
	Operation, ProjectState,
};

fn describe_operation(operation: &Operation) -> Result<String, String> {
	match operation {
		Operation::CreateTable { name, .. } => Ok(format!("create {name}")),
		Operation::DropTable { name } => Ok(format!("drop {name}")),
		other => Err(format!("unsupported migration operation: {other:?}")),
	}
}

fn describe_field_type(field_type: &FieldType) -> Result<&'static str, String> {
	match field_type {
		FieldType::BigInteger => Ok("bigint"),
		FieldType::Text => Ok("text"),
		other => Err(format!("unsupported field type: {other:?}")),
	}
}

fn describe_constraint(constraint: &Constraint) -> Result<&'static str, String> {
	match constraint {
		Constraint::Unique { .. } => Ok("unique"),
		Constraint::Check { .. } => Ok("check"),
		other => Err(format!("unsupported constraint: {other:?}")),
	}
}

fn describe_index_type(index_type: IndexType) -> Result<&'static str, String> {
	match index_type {
		IndexType::BTree => Ok("btree"),
		IndexType::Hash => Ok("hash"),
		other => Err(format!("unsupported index method: {other:?}")),
	}
}

fn describe_statement(statement: &OperationStatement) -> Result<&'static str, String> {
	match statement {
		OperationStatement::RawSql(_) => Ok("raw sql"),
		_ => Err("unsupported operation statement".to_string()),
	}
}

fn main() {
	// Introspection metadata
	let mut column = ColumnInfo::new("id", FieldType::BigInteger, false)
		.with_auto_increment(true)
		.with_default(None);
	column.nullable = false;
	let table = TableInfo::new("accounts")
		.with_column(column)
		.with_primary_key(vec!["id".to_string()]);
	let schema = DatabaseSchema::new()
		.with_table(table)
		.with_default_schema(Some("public".to_string()));
	let TableInfo {
		name, primary_key, ..
	} = &schema.tables["accounts"];
	assert_eq!(name, "accounts");
	assert_eq!(primary_key, &vec!["id".to_string()]);

	// Migration state
	let field = FieldState::new("id", FieldType::BigInteger, false)
		.with_param("primary_key", "true")
		.with_generated(None);
	let model = ModelState::new("accounts", "Account")
		.with_table_name("accounts")
		.with_field(field);
	let state = ProjectState::new().with_model(model);
	assert_eq!(
		state
			.get_model("accounts", "Account")
			.map(|model| model.table_name.as_str()),
		Some("accounts")
	);

	// Registry metadata
	let mut metadata = ModelMetadata::new("accounts", "Account", "accounts");
	metadata.add_field(
		"id".to_string(),
		FieldMetadata::new(FieldType::BigInteger).with_param("primary_key", "true"),
	);
	assert_eq!(metadata.to_model_state().table_name, "accounts");

	// Enums: known variants stay constructible; matches need a wildcard arm.
	let drop = Operation::DropTable {
		name: "legacy".to_string(),
	};
	assert_eq!(describe_operation(&drop).as_deref(), Ok("drop legacy"));
	assert_eq!(describe_field_type(&FieldType::Text), Ok("text"));
	assert!(describe_field_type(&FieldType::Uuid).is_err());
	let unique = Constraint::Unique {
		name: "accounts_email_key".to_string(),
		columns: vec!["email".to_string()],
	};
	assert_eq!(describe_constraint(&unique), Ok("unique"));
	assert_eq!(describe_index_type(IndexType::Hash), Ok("hash"));
	assert!(describe_index_type(IndexType::Gin).is_err());
	let statement = OperationStatement::RawSql("SELECT 1".to_string());
	assert_eq!(describe_statement(&statement), Ok("raw sql"));
}
