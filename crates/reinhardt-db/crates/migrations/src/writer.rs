//! Migration file writer
//!
//! Generates Rust migration files from Migration structs.
//!
//! ## AST-Based Code Generation
//!
//! This module uses Abstract Syntax Tree (AST) parsing via `syn` and `quote`
//! for robust migration file generation. Benefits include:
//!
//! - **Syntax Guarantee**: Generates syntactically correct Rust code
//! - **Consistent Formatting**: Uses `prettyplease` for standardized output
//! - **Maintainability**: Structural code generation via quote! macro
//! - **Extensibility**: Easy to add new operation types

use crate::{Migration, Operation, Result};
use std::fs;
use std::path::Path;

/// Writer for generating migration files
pub struct MigrationWriter {
	migration: Migration,
}

impl MigrationWriter {
	/// Create a new migration writer
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{Migration, writer::MigrationWriter};
	///
	/// let migration = Migration::new("0001_initial", "myapp");
	/// let writer = MigrationWriter::new(migration);
	/// ```
	pub fn new(migration: Migration) -> Self {
		Self { migration }
	}
	/// Generate the migration file content
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{Migration, Operation, ColumnDefinition, writer::MigrationWriter};
	///
	/// let migration = Migration::new("0001_initial", "myapp")
	///     .add_operation(Operation::CreateTable {
	///         name: "users".to_string(),
	///         columns: vec![ColumnDefinition::new("id", "INTEGER PRIMARY KEY")],
	///         constraints: vec![],
	///     });
	///
	/// let writer = MigrationWriter::new(migration);
	/// let content = writer.as_string();
	///
	/// assert!(content.contains("//! Name: 0001_initial"));
	/// assert!(content.contains("//! App: myapp"));
	/// assert!(content.contains("Migration::new"));
	/// ```
	pub fn as_string(&self) -> String {
		let mut content = String::new();

		// Add file header
		content.push_str("//! Auto-generated migration\n");
		content.push_str(&format!("//! Name: {}\n", self.migration.name));
		content.push_str(&format!("//! App: {}\n\n", self.migration.app_label));

		// Add imports
		content.push_str("use reinhardt_migrations::{\n");
		content.push_str("    Migration, Operation, CreateTable, AddColumn, AlterColumn,\n");
		content.push_str("    ColumnDefinition,\n");
		content.push_str("};\n\n");

		// Generate migration function
		content.push_str(&format!(
			"pub fn migration_{}() -> Migration {{\n",
			self.migration.name.replace('-', "_")
		));
		content.push_str(&format!(
			"    Migration::new(\"{}\", \"{}\")\n",
			self.migration.name, self.migration.app_label
		));

		// Add dependencies
		for (dep_app, dep_name) in &self.migration.dependencies {
			content.push_str(&format!(
				"        .add_dependency(\"{}\", \"{}\")\n",
				dep_app, dep_name
			));
		}

		// Add operations
		for operation in &self.migration.operations {
			content.push_str(&self.serialize_operation(operation, 2));
		}

		content.push_str("}\n");

		content
	}

	/// Generate migration file content using AST
	///
	/// This method uses Abstract Syntax Tree parsing to generate
	/// syntactically correct, well-formatted Rust code.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_migrations::{Migration, Operation, ColumnDefinition, writer::MigrationWriter};
	///
	/// let migration = Migration::new("0001_initial", "myapp")
	///     .add_operation(Operation::CreateTable {
	///         name: "users".to_string(),
	///         columns: vec![ColumnDefinition::new("id", "INTEGER PRIMARY KEY")],
	///         constraints: vec![],
	///     });
	///
	/// let writer = MigrationWriter::new(migration);
	/// let content = writer.as_string_ast();
	///
	/// assert!(content.contains("Name: 0001_initial"));
	/// assert!(content.contains("App: myapp"));
	/// assert!(content.contains("Migration::new"));
	/// ```
	pub fn as_string_ast(&self) -> String {
		use syn::{File, Item, ItemFn, ItemUse};

		// Generate imports
		let use_item: ItemUse = Self::generate_imports();

		// Generate function signature
		let signature = self.generate_function_signature();

		// Build migration expression with method chain
		let mut migration_expr = self.generate_migration_new();

		// Add dependencies
		for (dep_app, dep_name) in &self.migration.dependencies {
			let dep_call = self.generate_add_dependency(dep_app, dep_name);
			migration_expr = syn::parse_quote! {
				#migration_expr.#dep_call
			};
		}

		// Add operations
		for operation in &self.migration.operations {
			let op_call = self.generate_add_operation_ast(operation);
			migration_expr = syn::parse_quote! {
				#migration_expr.#op_call
			};
		}

		// Create function body
		let block: syn::Block = syn::parse_quote! {
			{
				#migration_expr
			}
		};

		// Create complete function
		let func = ItemFn {
			attrs: vec![],
			vis: syn::parse_quote!(pub),
			sig: signature,
			block: Box::new(block),
		};

		// Build AST
		let mut ast = File {
			shebang: None,
			attrs: vec![],
			items: vec![],
		};

		// Add file header as inner attributes
		let header_doc_1: syn::Attribute = syn::parse_quote!(#![doc = " Auto-generated migration"]);
		let header_doc_2: syn::Attribute =
			syn::parse_quote!(#![doc = concat!(" Name: ", stringify!(#(self.migration.name)))] );
		let header_doc_3: syn::Attribute = syn::parse_quote!(#![doc = concat!(" App: ", stringify!(#(self.migration.app_label)))] );

		ast.attrs.push(header_doc_1);
		ast.attrs.push(header_doc_2);
		ast.attrs.push(header_doc_3);

		// Add use statement and function
		ast.items.push(Item::Use(use_item));
		ast.items.push(Item::Fn(func));

		// Format with prettyplease
		prettyplease::unparse(&ast)
	}

	/// Serialize an operation to Rust code
	fn serialize_operation(&self, operation: &Operation, indent_level: usize) -> String {
		let indent = "    ".repeat(indent_level);
		let mut result = String::new();

		match operation {
			Operation::CreateTable {
				name,
				columns,
				constraints,
			} => {
				result.push_str(&format!(
					"{}    .add_operation(Operation::CreateTable {{\n",
					indent
				));
				result.push_str(&format!(
					"{}        name: \"{}\".to_string(),\n",
					indent, name
				));
				result.push_str(&format!("{}        columns: vec![\n", indent));

				for column in columns {
					result.push_str(&self.serialize_column(column, indent_level + 3));
				}

				result.push_str(&format!("{}        ],\n", indent));
				result.push_str(&format!("{}        constraints: vec![\n", indent));

				for constraint in constraints {
					result.push_str(&format!(
						"{}            \"{}\".to_string(),\n",
						indent, constraint
					));
				}

				result.push_str(&format!("{}        ],\n", indent));
				result.push_str(&format!("{}    }})\n", indent));
			}
			Operation::DropTable { name } => {
				result.push_str(&format!(
					"{}    .add_operation(Operation::DropTable {{\n",
					indent
				));
				result.push_str(&format!(
					"{}        name: \"{}\".to_string(),\n",
					indent, name
				));
				result.push_str(&format!("{}    }})\n", indent));
			}
			Operation::AddColumn { table, column } => {
				result.push_str(&format!(
					"{}    .add_operation(Operation::AddColumn {{\n",
					indent
				));
				result.push_str(&format!(
					"{}        table: \"{}\".to_string(),\n",
					indent, table
				));
				result.push_str(&format!("{}        column: ", indent));
				result.push_str(&self.serialize_column(column, indent_level + 2));
				result.push_str(&format!("{}    }})\n", indent));
			}
			Operation::AlterColumn {
				table,
				column,
				new_definition,
			} => {
				result.push_str(&format!(
					"{}    .add_operation(Operation::AlterColumn {{\n",
					indent
				));
				result.push_str(&format!(
					"{}        table: \"{}\".to_string(),\n",
					indent, table
				));
				result.push_str(&format!(
					"{}        column: \"{}\".to_string(),\n",
					indent, column
				));
				result.push_str(&format!("{}        new_definition: ", indent));
				result.push_str(&self.serialize_column(new_definition, indent_level + 2));
				result.push_str(&format!("{}    }})\n", indent));
			}
			Operation::DropColumn { table, column } => {
				result.push_str(&format!(
					"{}    .add_operation(Operation::DropColumn {{\n",
					indent
				));
				result.push_str(&format!(
					"{}        table: \"{}\".to_string(),\n",
					indent, table
				));
				result.push_str(&format!(
					"{}        column: \"{}\".to_string(),\n",
					indent, column
				));
				result.push_str(&format!("{}    }})\n", indent));
			}
			_ => {
				// Other operations not yet supported
				result.push_str(&format!("{}    // Unsupported operation\n", indent));
			}
		}

		result
	}

	/// Serialize a column definition to Rust code
	fn serialize_column(&self, column: &crate::ColumnDefinition, indent_level: usize) -> String {
		let indent = "    ".repeat(indent_level);
		let mut result = String::new();

		result.push_str("ColumnDefinition {\n");
		result.push_str(&format!(
			"{}    name: \"{}\".to_string(),\n",
			indent, column.name
		));
		result.push_str(&format!(
			"{}    type_definition: \"{}\".to_string(),\n",
			indent, column.type_definition
		));
		result.push_str(&format!("{}}},\n", indent));

		result
	}
	/// Write migration to file
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_migrations::{Migration, writer::MigrationWriter};
	/// use std::path::PathBuf;
	///
	/// let migration = Migration::new("0001_initial", "myapp");
	/// let writer = MigrationWriter::new(migration);
	///
	/// let temp_dir = PathBuf::from("/tmp/migrations");
	/// let filepath = writer.write_to_file(&temp_dir).unwrap();
	/// assert!(filepath.ends_with("0001_initial.rs"));
	/// ```
	pub fn write_to_file<P: AsRef<Path>>(&self, directory: P) -> Result<String> {
		let dir_path = directory.as_ref();
		fs::create_dir_all(dir_path)?;

		let filename = format!("{}.rs", self.migration.name);
		let filepath = dir_path.join(&filename);

		fs::write(&filepath, self.as_string())?;

		Ok(filepath.to_string_lossy().into_owned())
	}

	// =============================================================
	// AST-Based Code Generation Methods
	// =============================================================

	/// Generate file header doc comments as AST attributes
	#[allow(dead_code)]
	fn generate_file_header(&self) -> Vec<syn::Attribute> {
		vec![
			syn::parse_quote!(#![doc = " Auto-generated migration"]),
			syn::parse_quote!(#![doc = concat!(" Name: ", #(self.migration.name))]),
			syn::parse_quote!(#![doc = concat!(" App: ", #(self.migration.app_label))]),
		]
	}

	/// Generate use statement for migration imports
	fn generate_imports() -> syn::ItemUse {
		syn::parse_quote! {
			use reinhardt_migrations::{
				Migration, Operation, CreateTable, AddColumn, AlterColumn, ColumnDefinition
			};
		}
	}

	/// Generate migration function signature
	fn generate_function_signature(&self) -> syn::Signature {
		let func_name_str = format!("migration_{}", self.migration.name.replace('-', "_"));
		let func_name = syn::Ident::new(&func_name_str, proc_macro2::Span::call_site());

		// Build signature manually since parse_quote needs a complete function
		use syn::ReturnType;

		syn::Signature {
			constness: None,
			asyncness: None,
			unsafety: None,
			abi: None,
			fn_token: Default::default(),
			ident: func_name,
			generics: Default::default(),
			paren_token: Default::default(),
			inputs: Default::default(),
			variadic: None,
			output: ReturnType::Type(Default::default(), Box::new(syn::parse_quote!(Migration))),
		}
	}

	/// Generate Migration::new() expression
	fn generate_migration_new(&self) -> syn::Expr {
		let name = &self.migration.name;
		let app_label = &self.migration.app_label;

		syn::parse_quote! {
			Migration::new(#name, #app_label)
		}
	}

	/// Generate .add_dependency() method call expression
	fn generate_add_dependency(&self, dep_app: &str, dep_name: &str) -> syn::Expr {
		syn::parse_quote! {
			add_dependency(#dep_app, #dep_name)
		}
	}

	/// Generate .add_operation() method call expression (AST version)
	fn generate_add_operation_ast(&self, operation: &Operation) -> syn::Expr {
		match operation {
			Operation::CreateTable {
				name,
				columns,
				constraints,
			} => {
				let column_exprs: Vec<syn::Expr> = columns
					.iter()
					.map(|col| self.generate_column_definition_ast(col))
					.collect();

				let constraint_strs: Vec<&String> = constraints.iter().collect();

				syn::parse_quote! {
					add_operation(Operation::CreateTable {
						name: #name.to_string(),
						columns: vec![#(#column_exprs),*],
						constraints: vec![#(#constraint_strs.to_string()),*],
					})
				}
			}
			Operation::DropTable { name } => {
				syn::parse_quote! {
					add_operation(Operation::DropTable {
						name: #name.to_string(),
					})
				}
			}
			Operation::AddColumn { table, column } => {
				let column_expr = self.generate_column_definition_ast(column);

				syn::parse_quote! {
					add_operation(Operation::AddColumn {
						table: #table.to_string(),
						column: #column_expr,
					})
				}
			}
			Operation::AlterColumn {
				table,
				column,
				new_definition,
			} => {
				let new_def_expr = self.generate_column_definition_ast(new_definition);

				syn::parse_quote! {
					add_operation(Operation::AlterColumn {
						table: #table.to_string(),
						column: #column.to_string(),
						new_definition: #new_def_expr,
					})
				}
			}
			Operation::DropColumn { table, column } => {
				syn::parse_quote! {
					add_operation(Operation::DropColumn {
						table: #table.to_string(),
						column: #column.to_string(),
					})
				}
			}
			_ => {
				// Unsupported operations - generate a comment
				syn::parse_quote! {
					add_operation(Operation::RunSQL {
						sql: "-- Unsupported operation".to_string(),
						reverse_sql: None,
					})
				}
			}
		}
	}

	/// Generate ColumnDefinition struct expression
	fn generate_column_definition_ast(&self, column: &crate::ColumnDefinition) -> syn::Expr {
		let name = &column.name;
		let type_def = &column.type_definition;

		syn::parse_quote! {
			ColumnDefinition {
				name: #name.to_string(),
				type_definition: #type_def.to_string(),
			}
		}
	}
}

// Tests are in tests/test_writer.rs
