use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "dm".to_string(),
		name: "0001_initial".to_string(),
		operations: vec![
			Operation::CreateTable {
				name: "dm_message".to_string(),
				columns: vec![
					ColumnDefinition {
						name: "content".to_string(),
						type_definition: FieldType::VarChar(1000u32),
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "created_at".to_string(),
						type_definition: FieldType::TimestampTz,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "id".to_string(),
						type_definition: FieldType::Uuid,
						not_null: true,
						unique: false,
						primary_key: true,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "is_read".to_string(),
						type_definition: FieldType::Boolean,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: Some("false".to_string()),
					},
					ColumnDefinition {
						name: "room_id".to_string(),
						type_definition: FieldType::Uuid,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "sender_id".to_string(),
						type_definition: FieldType::Uuid,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "updated_at".to_string(),
						type_definition: FieldType::TimestampTz,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
				],
				constraints: vec![],
				without_rowid: None,
				interleave_in_parent: None,
				partition: None,
			},
			Operation::CreateTable {
				name: "dm_room".to_string(),
				columns: vec![
					ColumnDefinition {
						name: "created_at".to_string(),
						type_definition: FieldType::TimestampTz,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "id".to_string(),
						type_definition: FieldType::Uuid,
						not_null: true,
						unique: false,
						primary_key: true,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "is_group".to_string(),
						type_definition: FieldType::Boolean,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: Some("false".to_string()),
					},
					ColumnDefinition {
						name: "name".to_string(),
						type_definition: FieldType::VarChar(100u32),
						not_null: false,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "updated_at".to_string(),
						type_definition: FieldType::TimestampTz,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
				],
				constraints: vec![],
				without_rowid: None,
				interleave_in_parent: None,
				partition: None,
			},
			Operation::CreateTable {
				name: "dm_room_members".to_string(),
				columns: vec![
					ColumnDefinition {
						name: "from_dm_room_id".to_string(),
						type_definition: FieldType::Uuid,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
					ColumnDefinition {
						name: "id".to_string(),
						type_definition: FieldType::Integer,
						not_null: true,
						unique: false,
						primary_key: true,
						auto_increment: true,
						default: None,
					},
					ColumnDefinition {
						name: "to_user_id".to_string(),
						type_definition: FieldType::Uuid,
						not_null: true,
						unique: false,
						primary_key: false,
						auto_increment: false,
						default: None,
					},
				],
				constraints: vec![
					Constraint::ForeignKey {
						name: "fk_dm_room_members_from_dm_room_id".to_string(),
						columns: vec!["from_dm_room_id".to_string()],
						referenced_table: "dm_room".to_string(),
						referenced_columns: vec!["id".to_string()],
						on_delete: ForeignKeyAction::Cascade,
						on_update: ForeignKeyAction::Cascade,
						deferrable: None,
					},
					Constraint::ForeignKey {
						name: "fk_dm_room_members_to_user_id".to_string(),
						columns: vec!["to_user_id".to_string()],
						// Workaround for reinhardt#4659 (tracked in reinhardt#4659).
						// Remove this workaround when reinhardt#4659 is resolved.
						//
						// `makemigrations` emitted `referenced_table: "dm_user"`
						// here, templating the FK target as `<current_app>_user`
						// instead of consulting `User::app_label()` (= `"auth"`).
						// Once #4659 is fixed, regenerating from the current model
						// will emit the line below directly.
						//
						// Ideal implementation (without workaround):
						//   referenced_table: "auth_user".to_string(),
						referenced_table: "auth_user".to_string(),
						referenced_columns: vec!["id".to_string()],
						on_delete: ForeignKeyAction::Cascade,
						on_update: ForeignKeyAction::Cascade,
						deferrable: None,
					},
					Constraint::Unique {
						name: "dm_room_members_unique".to_string(),
						columns: vec!["from_dm_room_id".to_string(), "to_user_id".to_string()],
					},
				],
				without_rowid: None,
				interleave_in_parent: None,
				partition: None,
			},
			// Workaround for reinhardt#4659 (tracked in reinhardt#4659).
			// Remove this workaround when reinhardt#4659 is resolved.
			//
			// `makemigrations` emitted a second `Operation::CreateTable
			// { name: "dm_dmroom_members", ... }` here for the same DMRoom <-> User
			// M2M relation under a legacy `<app>_<lowercase_struct>_members`
			// naming. The ORM's `ManyToManyAccessor::<DMRoom, User>` queries
			// only `dm_room_members` (with `from_dm_room_id` / `to_user_id`),
			// so the duplicate table is dead schema and was deleted by hand.
			// Once #4659 is fixed, regenerating from the current model will
			// emit only the `dm_room_members` `CreateTable` above and the
			// `operations` vec will end here directly.
			//
			// Ideal implementation (without workaround):
			//   (no additional operations — only the `dm_room_members`
			//   CreateTable defined above remains)
		],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
		initial: Some(true),
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
