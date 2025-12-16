use crate::{ColumnDefinition, Constraint, ForeignKeyAction, Operation};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

impl ToTokens for ForeignKeyAction {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let variant = match self {
			ForeignKeyAction::Restrict => quote! { ForeignKeyAction::Restrict },
			ForeignKeyAction::Cascade => quote! { ForeignKeyAction::Cascade },
			ForeignKeyAction::SetNull => quote! { ForeignKeyAction::SetNull },
			ForeignKeyAction::NoAction => quote! { ForeignKeyAction::NoAction },
			ForeignKeyAction::SetDefault => quote! { ForeignKeyAction::SetDefault },
		};
		tokens.extend(variant);
	}
}

impl ToTokens for Constraint {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self {
			Constraint::ForeignKey {
				name,
				columns,
				referenced_table,
				referenced_columns,
				on_delete,
				on_update,
			} => {
				let columns_iter = columns.iter();
				let ref_columns_iter = referenced_columns.iter();
				tokens.extend(quote! {
					Constraint::ForeignKey {
						name: #name.to_string(),
						columns: vec![#(#columns_iter.to_string()),*],
						referenced_table: #referenced_table.to_string(),
						referenced_columns: vec![#(#ref_columns_iter.to_string()),*],
						on_delete: #on_delete,
						on_update: #on_update,
					}
				});
			}
			Constraint::Unique { name, columns } => {
				let columns_iter = columns.iter();
				tokens.extend(quote! {
					Constraint::Unique {
						name: #name.to_string(),
						columns: vec![#(#columns_iter.to_string()),*],
					}
				});
			}
			Constraint::Check { name, expression } => {
				tokens.extend(quote! {
					Constraint::Check {
						name: #name.to_string(),
						expression: #expression.to_string(),
					}
				});
			}
			Constraint::OneToOne {
				name,
				column,
				referenced_table,
				referenced_column,
				on_delete,
				on_update,
			} => {
				tokens.extend(quote! {
					Constraint::OneToOne {
						name: #name.to_string(),
						column: #column.to_string(),
						referenced_table: #referenced_table.to_string(),
						referenced_column: #referenced_column.to_string(),
						on_delete: #on_delete,
						on_update: #on_update,
					}
				});
			}
			Constraint::ManyToMany {
				name,
				through_table,
				source_column,
				target_column,
				target_table,
			} => {
				tokens.extend(quote! {
					Constraint::ManyToMany {
						name: #name.to_string(),
						through_table: #through_table.to_string(),
						source_column: #source_column.to_string(),
						target_column: #target_column.to_string(),
						target_table: #target_table.to_string(),
					}
				});
			}
		}
	}
}

impl ToTokens for Operation {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		match self {
			Operation::CreateTable {
				name,
				columns,
				constraints,
			} => {
				let columns_tokens = columns.iter();
				let constraints_tokens = constraints.iter();
				tokens.extend(quote! {
					Operation::CreateTable {
						name: #name,
						columns: vec![#(#columns_tokens),*],
						constraints: vec![#(#constraints_tokens),*],
					}
				});
			}
			Operation::DropTable { name } => {
				tokens.extend(quote! {
					Operation::DropTable {
						name: #name,
					}
				});
			}
			Operation::AddColumn { table, column } => {
				tokens.extend(quote! {
					Operation::AddColumn {
						table: #table,
						column: #column,
					}
				});
			}
			Operation::DropColumn { table, column } => {
				tokens.extend(quote! {
					Operation::DropColumn {
						table: #table,
						column: #column,
					}
				});
			}
			Operation::AlterColumn {
				table,
				column,
				new_definition,
			} => {
				tokens.extend(quote! {
					Operation::AlterColumn {
						table: #table,
						column: #column,
						new_definition: #new_definition,
					}
				});
			}
			Operation::RenameTable { old_name, new_name } => {
				tokens.extend(quote! {
					Operation::RenameTable {
						old_name: #old_name,
						new_name: #new_name,
					}
				});
			}
			Operation::RenameColumn {
				table,
				old_name,
				new_name,
			} => {
				tokens.extend(quote! {
					Operation::RenameColumn {
						table: #table,
						old_name: #old_name,
						new_name: #new_name,
					}
				});
			}
			Operation::AddConstraint {
				table,
				constraint_sql,
			} => {
				tokens.extend(quote! {
					Operation::AddConstraint {
						table: #table,
						constraint_sql: #constraint_sql,
					}
				});
			}
			Operation::DropConstraint {
				table,
				constraint_name,
			} => {
				tokens.extend(quote! {
					Operation::DropConstraint {
						table: #table,
						constraint_name: #constraint_name,
					}
				});
			}
			Operation::CreateIndex {
				table,
				columns,
				unique,
			} => {
				let columns_iter = columns.iter();
				tokens.extend(quote! {
					Operation::CreateIndex {
						table: #table,
						columns: vec![#(#columns_iter),*],
						unique: #unique,
					}
				});
			}
			Operation::DropIndex { table, columns } => {
				let columns_iter = columns.iter();
				tokens.extend(quote! {
					Operation::DropIndex {
						table: #table,
						columns: vec![#(#columns_iter),*],
					}
				});
			}
			Operation::RunSQL { sql, reverse_sql } => {
				let reverse_sql_token = match reverse_sql {
					Some(s) => quote! { Some(#s) },
					None => quote! { None },
				};
				tokens.extend(quote! {
					Operation::RunSQL {
						sql: #sql,
						reverse_sql: #reverse_sql_token,
					}
				});
			}
			Operation::RunRust { code, reverse_code } => {
				let reverse_code_token = match reverse_code {
					Some(s) => quote! { Some(#s) },
					None => quote! { None },
				};
				tokens.extend(quote! {
					Operation::RunRust {
						code: #code,
						reverse_code: #reverse_code_token,
					}
				});
			}
			Operation::AlterTableComment { table, comment } => {
				let comment_token = match comment {
					Some(s) => quote! { Some(#s) },
					None => quote! { None },
				};
				tokens.extend(quote! {
					Operation::AlterTableComment {
						table: #table,
						comment: #comment_token,
					}
				});
			}
			Operation::AlterUniqueTogether {
				table,
				unique_together,
			} => {
				let unique_together_tokens = unique_together.iter().map(|fields| {
					let fields_iter = fields.iter();
					quote! { vec![#(#fields_iter),*] }
				});
				tokens.extend(quote! {
					Operation::AlterUniqueTogether {
						table: #table,
						unique_together: vec![#(#unique_together_tokens),*],
					}
				});
			}
			Operation::AlterModelOptions { table, options } => {
				let keys = options.keys();
				let values = options.values();
				tokens.extend(quote! {
					Operation::AlterModelOptions {
						table: #table,
						options: {
							let mut map = std::collections::HashMap::new();
							#(map.insert(#keys, #values);)*
							map
						},
					}
				});
			}
			Operation::CreateInheritedTable {
				name,
				columns,
				base_table,
				join_column,
			} => {
				let columns_tokens = columns.iter();
				tokens.extend(quote! {
					Operation::CreateInheritedTable {
						name: #name,
						columns: vec![#(#columns_tokens),*],
						base_table: #base_table,
						join_column: #join_column,
					}
				});
			}
			Operation::AddDiscriminatorColumn {
				table,
				column_name,
				default_value,
			} => {
				tokens.extend(quote! {
					Operation::AddDiscriminatorColumn {
						table: #table,
						column_name: #column_name,
						default_value: #default_value,
					}
				});
			}
			Operation::MoveModel {
				model_name,
				from_app,
				to_app,
				rename_table,
				old_table_name,
				new_table_name,
			} => {
				let old_table_token = match old_table_name {
					Some(s) => quote! { Some(#s) },
					None => quote! { None },
				};
				let new_table_token = match new_table_name {
					Some(s) => quote! { Some(#s) },
					None => quote! { None },
				};
				tokens.extend(quote! {
					Operation::MoveModel {
						model_name: #model_name,
						from_app: #from_app,
						to_app: #to_app,
						rename_table: #rename_table,
						old_table_name: #old_table_token,
						new_table_name: #new_table_token,
					}
				});
			}
		}
	}
}

impl ToTokens for ColumnDefinition {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let name = &self.name;
		let not_null = self.not_null;
		let unique = self.unique;
		let primary_key = self.primary_key;
		let auto_increment = self.auto_increment;

		let default_token = match &self.default {
			Some(s) => quote! { Some(#s) },
			None => quote! { None },
		};

		// Generate FieldType token based on the actual type
		let field_type_token = match &self.type_definition {
			// Integer types
			crate::FieldType::BigInteger => quote! { FieldType::BigInteger },
			crate::FieldType::Integer => quote! { FieldType::Integer },
			crate::FieldType::SmallInteger => quote! { FieldType::SmallInteger },
			crate::FieldType::TinyInt => quote! { FieldType::TinyInt },
			crate::FieldType::MediumInt => quote! { FieldType::MediumInt },

			// String types
			crate::FieldType::Char(len) => quote! { FieldType::Char(#len) },
			crate::FieldType::VarChar(len) => quote! { FieldType::VarChar(#len) },
			crate::FieldType::Text => quote! { FieldType::Text },
			crate::FieldType::TinyText => quote! { FieldType::TinyText },
			crate::FieldType::MediumText => quote! { FieldType::MediumText },
			crate::FieldType::LongText => quote! { FieldType::LongText },

			// Date/Time types
			crate::FieldType::Date => quote! { FieldType::Date },
			crate::FieldType::Time => quote! { FieldType::Time },
			crate::FieldType::DateTime => quote! { FieldType::DateTime },
			crate::FieldType::TimestampTz => quote! { FieldType::TimestampTz },

			// Numeric types
			crate::FieldType::Decimal { precision, scale } => {
				quote! { FieldType::Decimal { precision: #precision, scale: #scale } }
			}
			crate::FieldType::Float => quote! { FieldType::Float },
			crate::FieldType::Double => quote! { FieldType::Double },
			crate::FieldType::Real => quote! { FieldType::Real },

			// Boolean
			crate::FieldType::Boolean => quote! { FieldType::Boolean },

			// Binary types
			crate::FieldType::Binary => quote! { FieldType::Binary },
			crate::FieldType::Blob => quote! { FieldType::Blob },
			crate::FieldType::TinyBlob => quote! { FieldType::TinyBlob },
			crate::FieldType::MediumBlob => quote! { FieldType::MediumBlob },
			crate::FieldType::LongBlob => quote! { FieldType::LongBlob },
			crate::FieldType::Bytea => quote! { FieldType::Bytea },

			// JSON types
			crate::FieldType::Json => quote! { FieldType::Json },
			crate::FieldType::JsonBinary => quote! { FieldType::JsonBinary },

			// UUID and Year
			crate::FieldType::Uuid => quote! { FieldType::Uuid },
			crate::FieldType::Year => quote! { FieldType::Year },

			// Collection types
			crate::FieldType::Enum { values } => {
				quote! { FieldType::Enum { values: vec![#(#values.to_string()),*] } }
			}
			crate::FieldType::Set { values } => {
				quote! { FieldType::Set { values: vec![#(#values.to_string()),*] } }
			}

			// Relationship types
			crate::FieldType::OneToOne {
				to,
				on_delete,
				on_update,
			} => {
				quote! {
					FieldType::OneToOne {
						to: #to.to_string(),
						on_delete: #on_delete,
						on_update: #on_update,
					}
				}
			}
			crate::FieldType::ManyToMany { to, through } => {
				let through_token = match through {
					Some(t) => quote! { Some(#t.to_string()) },
					None => quote! { None },
				};
				quote! {
					FieldType::ManyToMany {
						to: #to.to_string(),
						through: #through_token,
					}
				}
			}

			// Custom types
			crate::FieldType::Custom(s) => quote! { FieldType::Custom(#s.to_string()) },
		};

		tokens.extend(quote! {
			ColumnDefinition {
				name: #name,
				type_definition: #field_type_token,
				not_null: #not_null,
				unique: #unique,
				primary_key: #primary_key,
				auto_increment: #auto_increment,
				default: #default_token,
			}
		});
	}
}
