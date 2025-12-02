use crate::{ColumnDefinition, Operation};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

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
		}
	}
}

impl ToTokens for ColumnDefinition {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let name = &self.name;
		// Convert type_definition string to an identifier (e.g., "CharField" -> CharField)
		let type_definition = syn::Ident::new(self.type_definition, proc_macro2::Span::call_site());
		let not_null = self.not_null;
		let unique = self.unique;
		let primary_key = self.primary_key;
		let auto_increment = self.auto_increment;

		let default_token = match &self.default {
			Some(s) => quote! { Some(#s) },
			None => quote! { None },
		};

		let max_length_token = match self.max_length {
			Some(l) => quote! { Some(#l) },
			None => quote! { None },
		};

		tokens.extend(quote! {
			ColumnDefinition {
				name: #name,
				type_definition: #type_definition,
				not_null: #not_null,
				unique: #unique,
				primary_key: #primary_key,
				auto_increment: #auto_increment,
				default: #default_token,
				max_length: #max_length_token,
			}
		});
	}
}
