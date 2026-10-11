// External crates must not match `Operation` exhaustively. Every variant that
// exists today is listed, so the only reported gap is the `#[non_exhaustive]`
// wildcard. Adding an `Operation` variant requires listing it here as well.
use reinhardt_db::migrations::Operation;

fn describe(operation: &Operation) -> &'static str {
	match operation {
		Operation::CreateTable { .. } => "CreateTable",
		Operation::DropTable { .. } => "DropTable",
		Operation::AddColumn { .. } => "AddColumn",
		Operation::DropColumn { .. } => "DropColumn",
		Operation::AlterColumn { .. } => "AlterColumn",
		Operation::RenameTable { .. } => "RenameTable",
		Operation::RenameColumn { .. } => "RenameColumn",
		Operation::AddConstraint { .. } => "AddConstraint",
		Operation::AddConstraintDefinition { .. } => "AddConstraintDefinition",
		Operation::AddConstraintRepair { .. } => "AddConstraintRepair",
		Operation::RestoreConstraintOnRollback { .. } => "RestoreConstraintOnRollback",
		Operation::DropConstraint { .. } => "DropConstraint",
		Operation::DropConstraintDefinition { .. } => "DropConstraintDefinition",
		Operation::CreateIndex { .. } => "CreateIndex",
		Operation::CreateNamedIndex { .. } => "CreateNamedIndex",
		Operation::CreateIndexRepair { .. } => "CreateIndexRepair",
		Operation::RestoreIndexOnRollback { .. } => "RestoreIndexOnRollback",
		Operation::DropIndex { .. } => "DropIndex",
		Operation::DropNamedIndex { .. } => "DropNamedIndex",
		Operation::RunSQL { .. } => "RunSQL",
		Operation::RunRust { .. } => "RunRust",
		Operation::AlterTableComment { .. } => "AlterTableComment",
		Operation::AlterUniqueTogether { .. } => "AlterUniqueTogether",
		Operation::AlterModelOptions { .. } => "AlterModelOptions",
		Operation::CreateInheritedTable { .. } => "CreateInheritedTable",
		Operation::AddDiscriminatorColumn { .. } => "AddDiscriminatorColumn",
		Operation::MoveModel { .. } => "MoveModel",
		Operation::CreateSchema { .. } => "CreateSchema",
		Operation::DropSchema { .. } => "DropSchema",
		Operation::CreateExtension { .. } => "CreateExtension",
		Operation::BulkLoad { .. } => "BulkLoad",
		Operation::SetAutoIncrementValue { .. } => "SetAutoIncrementValue",
		Operation::CreateCompositePrimaryKey { .. } => "CreateCompositePrimaryKey",
		Operation::Sequence { .. } => "Sequence",
		Operation::Identity { .. } => "Identity",
		Operation::DropExtension { .. } => "DropExtension",
	}
}

fn main() {
	let operation = Operation::DropTable {
		name: "legacy".to_string(),
	};
	let _ = describe(&operation);
}
