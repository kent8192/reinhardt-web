//! AST parser utilities for migration files
//!
//! Provides helper functions to extract migration metadata and operations
//! from parsed Rust ASTs.

use super::{Migration, Result};
use quote::ToTokens;
use reinhardt_query::value::Value as QueryValue;
use syn::{Expr, File, Item, ItemFn, Stmt};

/// Extract migration metadata from parsed AST
pub fn extract_migration_metadata(ast: &File, app_label: &str, name: &str) -> Result<Migration> {
	let dependencies = extract_dependencies(ast)?;
	let atomic = extract_atomic(ast).unwrap_or(true);
	let replaces = extract_replaces(ast).unwrap_or_default();
	let operations = extract_operations(ast).unwrap_or_default();
	let initial = extract_initial(ast);

	Ok(Migration {
		app_label: app_label.to_string(),
		name: name.to_string(),
		operations,
		dependencies,
		atomic,
		replaces,
		initial,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	})
}

/// Extract dependencies from `migration()` function
fn extract_dependencies(ast: &File) -> Result<Vec<(String, String)>> {
	// Find the migration() function
	for item in &ast.items {
		if let Item::Fn(func) = item
			&& func.sig.ident == "migration"
		{
			// Look for the Migration struct literal in the return value
			if let Some(Stmt::Expr(expr, _)) = func.block.stmts.last()
				&& let Some(dependencies) =
					extract_field_from_migration_struct(expr, "dependencies")
			{
				return parse_tuple_vec_expr(&dependencies);
			}
		}
	}
	Ok(vec![])
}

/// Extract atomic flag from `atomic()` function
fn extract_atomic(ast: &File) -> Option<bool> {
	for item in &ast.items {
		if let Item::Fn(func) = item
			&& func.sig.ident == "atomic"
		{
			return parse_bool_return(func);
		}
	}
	None
}

/// Extract replaces from `migration()` function
fn extract_replaces(ast: &File) -> Option<Vec<(String, String)>> {
	// Find the migration() function
	for item in &ast.items {
		if let Item::Fn(func) = item
			&& func.sig.ident == "migration"
		{
			// Look for the Migration struct literal in the return value
			if let Some(Stmt::Expr(expr, _)) = func.block.stmts.last()
				&& let Some(replaces) = extract_field_from_migration_struct(expr, "replaces")
			{
				return parse_tuple_vec_expr(&replaces).ok();
			}
		}
	}
	None
}

/// Extract initial flag from `migration()` function
fn extract_initial(ast: &File) -> Option<bool> {
	for item in &ast.items {
		if let Item::Fn(func) = item
			&& func.sig.ident == "migration"
			&& let Some(Stmt::Expr(expr, _)) = func.block.stmts.last()
			&& let Some(initial_expr) = extract_field_from_migration_struct(expr, "initial")
		{
			return parse_option_bool_expr(&initial_expr);
		}
	}
	None
}

/// Parse an `Option<bool>` expression (`Some(true)`, `Some(false)`, or `None`)
fn parse_option_bool_expr(expr: &Expr) -> Option<bool> {
	match expr {
		Expr::Call(call) => {
			// Some(true) or Some(false)
			if let Expr::Path(path) = &*call.func
				&& path.path.is_ident("Some")
				&& call.args.len() == 1
				&& let Expr::Lit(lit) = &call.args[0]
				&& let syn::Lit::Bool(b) = &lit.lit
			{
				return Some(b.value);
			}
			None
		}
		Expr::Path(path) if path.path.is_ident("None") => None,
		_ => None,
	}
}

/// Extract operations from `migration()` function
fn extract_operations(ast: &File) -> Result<Vec<super::Operation>> {
	let mut operations = Vec::new();

	// Find the migration() function
	for item in &ast.items {
		if let Item::Fn(func) = item
			&& func.sig.ident == "migration"
		{
			// Look for the Migration struct literal in the return value
			if let Some(Stmt::Expr(expr, _)) = func.block.stmts.last()
				&& let Some(ops_expr) = extract_field_from_migration_struct(expr, "operations")
			{
				operations = parse_operations_vec(&ops_expr);
			}
		}
	}

	Ok(operations)
}

/// Parse operations from vec![...] expression
fn parse_operations_vec(expr: &Expr) -> Vec<super::Operation> {
	let mut operations = Vec::new();

	match expr {
		// Handle vec![...] macro
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			// The tokens contain the operation expressions separated by commas
			// We need to parse them as expressions
			if let Ok(parsed) = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }) {
				for elem in &parsed.elems {
					if let Some(op) = parse_single_operation(elem) {
						operations.push(op);
					}
				}
			}
		}
		// Handle array literal [...]
		Expr::Array(expr_array) => {
			for elem in &expr_array.elems {
				if let Some(op) = parse_single_operation(elem) {
					operations.push(op);
				}
			}
		}
		_ => {}
	}

	operations
}

/// Parse a single Operation from an expression
fn parse_single_operation(expr: &Expr) -> Option<super::Operation> {
	// Handle Operation::CreateTable { ... }
	if let Expr::Struct(expr_struct) = expr {
		// Extract the variant name
		let variant_name = expr_struct.path.segments.last()?.ident.to_string();

		match variant_name.as_str() {
			"CreateTable" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				let columns = extract_columns_field(&expr_struct.fields)?;
				let constraints = extract_constraints_field(&expr_struct.fields);

				return Some(super::Operation::CreateTable {
					name,
					columns,
					constraints,
					without_rowid: None,
					interleave_in_parent: None,
					partition: None,
				});
			}
			"DropTable" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				return Some(super::Operation::DropTable { name });
			}
			"AddColumn" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let column = extract_column_definition_field(&expr_struct.fields, "column")?;
				return Some(super::Operation::AddColumn {
					table,
					column,
					mysql_options: None,
				});
			}
			"DropColumn" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let column = extract_string_field(&expr_struct.fields, "column")?;
				return Some(super::Operation::DropColumn { table, column });
			}
			"AlterColumn" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let column = extract_string_field(&expr_struct.fields, "column")?;
				let new_definition =
					extract_column_definition_field(&expr_struct.fields, "new_definition")?;
				return Some(super::Operation::AlterColumn {
					table,
					column,
					new_definition,
					old_definition: None,
					mysql_options: None,
				});
			}
			"RenameTable" => {
				let old_name = extract_string_field(&expr_struct.fields, "old_name")?;
				let new_name = extract_string_field(&expr_struct.fields, "new_name")?;
				return Some(super::Operation::RenameTable { old_name, new_name });
			}
			"RenameColumn" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let old_name = extract_string_field(&expr_struct.fields, "old_name")?;
				let new_name = extract_string_field(&expr_struct.fields, "new_name")?;
				return Some(super::Operation::RenameColumn {
					table,
					old_name,
					new_name,
				});
			}
			"CreateIndex" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let columns = extract_string_vec_field(&expr_struct.fields, "columns");
				let unique = extract_bool_field(&expr_struct.fields, "unique").unwrap_or(false);
				let index_type = extract_index_type_field(&expr_struct.fields, "index_type");
				let where_clause = extract_optional_str_field(&expr_struct.fields, "where_clause");
				let concurrently =
					extract_bool_field(&expr_struct.fields, "concurrently").unwrap_or(false);

				return Some(super::Operation::CreateIndex {
					table,
					columns,
					unique,
					index_type,
					where_clause,
					concurrently,
					expressions: None,
					mysql_options: None,
					operator_class: None,
				});
			}
			"DropIndex" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let columns = extract_string_vec_field(&expr_struct.fields, "columns");
				return Some(super::Operation::DropIndex { table, columns });
			}
			"AddConstraint" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let constraint_sql = extract_string_field(&expr_struct.fields, "constraint_sql")?;
				return Some(super::Operation::AddConstraint {
					table,
					constraint_sql,
				});
			}
			"DropConstraint" => {
				let table = extract_string_field(&expr_struct.fields, "table")?;
				let constraint_name = extract_string_field(&expr_struct.fields, "constraint_name")?;
				return Some(super::Operation::DropConstraint {
					table,
					constraint_name,
				});
			}
			"RunSQL" => {
				// Use extract_string_field to handle both literal and .to_string() patterns (#1336)
				let sql = extract_string_field(&expr_struct.fields, "sql")?;
				let reverse_sql = extract_optional_str_field(&expr_struct.fields, "reverse_sql");
				return Some(super::Operation::RunSQL { sql, reverse_sql });
			}
			_ => {
				// Log unhandled operation types
				eprintln!(
					"Warning: Unhandled operation type in AST parser: {}",
					variant_name
				);
			}
		}
	}

	None
}

/// Extract a boolean field from struct fields
fn extract_bool_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<bool> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
			&& let Expr::Lit(expr_lit) = &field.expr
			&& let syn::Lit::Bool(lit_bool) = &expr_lit.lit
		{
			return Some(lit_bool.value);
		}
	}
	None
}

/// Extract an optional string field (`Option<&'static str>`)
fn extract_optional_str_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<String> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			// Check for None
			if let Expr::Path(expr_path) = &field.expr
				&& expr_path.path.is_ident("None")
			{
				return None;
			}
			// Check for Some(...)
			if let Expr::Call(expr_call) = &field.expr
				&& let Expr::Path(func_path) = &*expr_call.func
				&& func_path.path.is_ident("Some")
				&& !expr_call.args.is_empty()
			{
				// Handle Some("str".to_string()) pattern
				if let Expr::MethodCall(method_call) = &expr_call.args[0]
					&& method_call.method == "to_string"
				{
					return extract_string_literal(&method_call.receiver);
				}
				return extract_string_literal(&expr_call.args[0]);
			}
		}
	}
	None
}

fn extract_optional_expr_tokens_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<String> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			if let Expr::Path(expr_path) = &field.expr
				&& expr_path.path.is_ident("None")
			{
				return None;
			}
			if let Expr::Call(expr_call) = &field.expr
				&& let Expr::Path(func_path) = &*expr_call.func
				&& func_path.path.is_ident("Some")
				&& expr_call.args.len() == 1
			{
				let expr = unwrap_box_new_expr(&expr_call.args[0]).unwrap_or(&expr_call.args[0]);
				return Some(expr.to_token_stream().to_string());
			}
		}
	}
	None
}

/// Extract a `Vec<&'static str>` field
fn extract_string_vec_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Vec<String> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			return extract_string_vec(&field.expr);
		}
	}
	Vec::new()
}

/// Extract `Vec<String>` from expression
fn extract_string_vec(expr: &Expr) -> Vec<String> {
	let mut result = Vec::new();

	match expr {
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			if let Ok(parsed) = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }) {
				for elem in &parsed.elems {
					if let Some(s) = extract_string_literal(elem) {
						result.push(s);
					}
				}
			}
		}
		Expr::Array(expr_array) => {
			for elem in &expr_array.elems {
				if let Some(s) = extract_string_literal(elem) {
					result.push(s);
				}
			}
		}
		_ => {}
	}

	result
}

/// Extract columns (`Vec<ColumnDefinition>`) field from struct
fn extract_columns_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
) -> Option<Vec<super::ColumnDefinition>> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == "columns"
		{
			return Some(parse_columns_vec(&field.expr));
		}
	}
	None
}

/// Extract a string field that may use .to_string() pattern
fn extract_string_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<String> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			// Handle "string".to_string() pattern
			if let Expr::MethodCall(method_call) = &field.expr
				&& method_call.method == "to_string"
			{
				return extract_string_literal(&method_call.receiver);
			}
			// Handle direct string literal
			return extract_string_literal(&field.expr);
		}
	}
	None
}

/// Extract ForeignKeyAction enum from struct field
fn extract_foreign_key_action_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<super::ForeignKeyAction> {
	use super::ForeignKeyAction;

	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
			&& let Expr::Path(expr_path) = &field.expr
			&& let Some(last_segment) = expr_path.path.segments.last()
		{
			let variant = last_segment.ident.to_string();
			return match variant.as_str() {
				"Restrict" => Some(ForeignKeyAction::Restrict),
				"Cascade" => Some(ForeignKeyAction::Cascade),
				"SetNull" => Some(ForeignKeyAction::SetNull),
				"NoAction" => Some(ForeignKeyAction::NoAction),
				"SetDefault" => Some(ForeignKeyAction::SetDefault),
				_ => None,
			};
		}
	}
	None
}

/// Extract IndexType enum from struct field (for CreateIndex operation)
fn extract_index_type_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<super::IndexType> {
	use super::IndexType;

	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			// Check for None
			if let Expr::Path(expr_path) = &field.expr
				&& expr_path.path.is_ident("None")
			{
				return None;
			}

			// Check for Some(IndexType::Variant)
			if let Expr::Call(expr_call) = &field.expr
				&& let Expr::Path(func_path) = &*expr_call.func
				&& func_path.path.is_ident("Some")
				&& !expr_call.args.is_empty()
				&& let Expr::Path(variant_path) = &expr_call.args[0]
				&& let Some(last_segment) = variant_path.path.segments.last()
			{
				let variant = last_segment.ident.to_string();
				return match variant.as_str() {
					"BTree" => Some(IndexType::BTree),
					"Hash" => Some(IndexType::Hash),
					"Gin" => Some(IndexType::Gin),
					"Gist" => Some(IndexType::Gist),
					"Brin" => Some(IndexType::Brin),
					"Fulltext" => Some(IndexType::Fulltext),
					"Spatial" => Some(IndexType::Spatial),
					_ => None,
				};
			}
		}
	}
	None
}

/// Extract `Vec<String>` from vec!["str".to_string(), ...] pattern
fn extract_string_vec_from_to_string(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Vec<String> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			return parse_string_vec_with_to_string(&field.expr);
		}
	}
	Vec::new()
}

/// Parse `Vec<String>` from expression with .to_string() calls
fn parse_string_vec_with_to_string(expr: &Expr) -> Vec<String> {
	let mut result = Vec::new();

	match expr {
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			if let Ok(parsed) = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }) {
				for elem in &parsed.elems {
					// Handle "string".to_string() pattern
					if let Expr::MethodCall(method_call) = elem
						&& method_call.method == "to_string"
					{
						if let Some(s) = extract_string_literal(&method_call.receiver) {
							result.push(s);
						}
					}
					// Handle direct string literal
					else if let Some(s) = extract_string_literal(elem) {
						result.push(s);
					}
				}
			}
		}
		Expr::Array(expr_array) => {
			for elem in &expr_array.elems {
				if let Expr::MethodCall(method_call) = elem
					&& method_call.method == "to_string"
				{
					if let Some(s) = extract_string_literal(&method_call.receiver) {
						result.push(s);
					}
				} else if let Some(s) = extract_string_literal(elem) {
					result.push(s);
				}
			}
		}
		_ => {}
	}

	result
}

/// Parse a single Constraint from struct expression
fn parse_single_constraint(expr: &Expr) -> Option<super::Constraint> {
	if let Expr::Struct(expr_struct) = expr {
		let variant_name = expr_struct.path.segments.last()?.ident.to_string();

		match variant_name.as_str() {
			"ForeignKey" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				let columns = extract_string_vec_from_to_string(&expr_struct.fields, "columns");
				let referenced_table =
					extract_string_field(&expr_struct.fields, "referenced_table")?;
				let referenced_columns =
					extract_string_vec_from_to_string(&expr_struct.fields, "referenced_columns");
				let on_delete = extract_foreign_key_action_field(&expr_struct.fields, "on_delete")
					.unwrap_or(super::ForeignKeyAction::Restrict);
				let on_update = extract_foreign_key_action_field(&expr_struct.fields, "on_update")
					.unwrap_or(super::ForeignKeyAction::Restrict);

				return Some(super::Constraint::ForeignKey {
					name,
					columns,
					referenced_table,
					referenced_columns,
					on_delete,
					on_update,
					deferrable: None,
				});
			}
			"Unique" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				let columns = extract_string_vec_from_to_string(&expr_struct.fields, "columns");

				return Some(super::Constraint::Unique { name, columns });
			}
			"Check" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				let expression = extract_string_field(&expr_struct.fields, "expression")?;

				return Some(super::Constraint::Check { name, expression });
			}
			"OneToOne" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				let column = extract_string_field(&expr_struct.fields, "column")?;
				let referenced_table =
					extract_string_field(&expr_struct.fields, "referenced_table")?;
				let referenced_column =
					extract_string_field(&expr_struct.fields, "referenced_column")?;
				let on_delete = extract_foreign_key_action_field(&expr_struct.fields, "on_delete")
					.unwrap_or(super::ForeignKeyAction::Restrict);
				let on_update = extract_foreign_key_action_field(&expr_struct.fields, "on_update")
					.unwrap_or(super::ForeignKeyAction::NoAction);

				return Some(super::Constraint::OneToOne {
					name,
					column,
					referenced_table,
					referenced_column,
					on_delete,
					on_update,
					deferrable: None,
				});
			}
			"ManyToMany" => {
				let name = extract_string_field(&expr_struct.fields, "name")?;
				let through_table = extract_string_field(&expr_struct.fields, "through_table")?;
				let source_column = extract_string_field(&expr_struct.fields, "source_column")?;
				let target_column = extract_string_field(&expr_struct.fields, "target_column")?;
				let target_table = extract_string_field(&expr_struct.fields, "target_table")?;

				return Some(super::Constraint::ManyToMany {
					name,
					through_table,
					source_column,
					target_column,
					target_table,
				});
			}
			_ => {
				eprintln!(
					"Warning: Unhandled constraint type in AST parser: {}",
					variant_name
				);
			}
		}
	}

	None
}

/// Parse constraints from vec![...] or array expression
fn parse_constraints_vec(expr: &Expr) -> Vec<super::Constraint> {
	let mut constraints = Vec::new();

	match expr {
		// Handle vec![...] macro
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			if let Ok(parsed) = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }) {
				for elem in &parsed.elems {
					if let Some(constraint) = parse_single_constraint(elem) {
						constraints.push(constraint);
					}
				}
			}
		}
		// Handle array literal [...]
		Expr::Array(expr_array) => {
			for elem in &expr_array.elems {
				if let Some(constraint) = parse_single_constraint(elem) {
					constraints.push(constraint);
				}
			}
		}
		_ => {}
	}

	constraints
}

/// Extract constraints from struct
fn extract_constraints_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
) -> Vec<super::Constraint> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == "constraints"
		{
			return parse_constraints_vec(&field.expr);
		}
	}
	Vec::new()
}

/// Extract a single ColumnDefinition field
fn extract_column_definition_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<super::ColumnDefinition> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			return parse_column_definition(&field.expr);
		}
	}
	None
}

/// Parse `Vec<ColumnDefinition>` from expression
fn parse_columns_vec(expr: &Expr) -> Vec<super::ColumnDefinition> {
	let mut columns = Vec::new();

	match expr {
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			if let Ok(parsed) = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }) {
				for elem in &parsed.elems {
					if let Some(col) = parse_column_definition(elem) {
						columns.push(col);
					}
				}
			}
		}
		Expr::Array(expr_array) => {
			for elem in &expr_array.elems {
				if let Some(col) = parse_column_definition(elem) {
					columns.push(col);
				}
			}
		}
		_ => {}
	}

	columns
}

/// Parse a single ColumnDefinition from struct expression
fn parse_column_definition(expr: &Expr) -> Option<super::ColumnDefinition> {
	if let Expr::Struct(expr_struct) = expr {
		// Verify it's a ColumnDefinition struct
		let struct_name = expr_struct.path.segments.last()?.ident.to_string();
		if struct_name != "ColumnDefinition" {
			return None;
		}

		let name = extract_string_field(&expr_struct.fields, "name")?;
		let type_definition = extract_field_type(&expr_struct.fields)
			.unwrap_or(super::FieldType::Custom("VARCHAR".to_string()));
		let not_null = extract_bool_field(&expr_struct.fields, "not_null").unwrap_or(false);
		let unique = extract_bool_field(&expr_struct.fields, "unique").unwrap_or(false);
		let primary_key = extract_bool_field(&expr_struct.fields, "primary_key").unwrap_or(false);
		let auto_increment =
			extract_bool_field(&expr_struct.fields, "auto_increment").unwrap_or(false);
		let default = extract_optional_str_field(&expr_struct.fields, "default");
		let generated = extract_generated_column_field(&expr_struct.fields);

		return Some(super::ColumnDefinition {
			name,
			type_definition,
			not_null,
			unique,
			primary_key,
			auto_increment,
			default,
			generated,
		});
	}

	None
}

fn extract_generated_column_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
) -> Option<super::GeneratedColumnDefinition> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == "generated"
		{
			return parse_optional_generated_column_definition(&field.expr);
		}
	}
	None
}

fn parse_optional_generated_column_definition(
	expr: &Expr,
) -> Option<super::GeneratedColumnDefinition> {
	if let Expr::Path(expr_path) = expr
		&& expr_path.path.is_ident("None")
	{
		return None;
	}

	if let Expr::Call(expr_call) = expr
		&& let Expr::Path(func_path) = &*expr_call.func
		&& func_path.path.is_ident("Some")
		&& expr_call.args.len() == 1
	{
		return parse_generated_column_definition(&expr_call.args[0]);
	}

	None
}

fn parse_generated_column_definition(expr: &Expr) -> Option<super::GeneratedColumnDefinition> {
	if let Expr::Struct(expr_struct) = expr {
		let struct_name = expr_struct.path.segments.last()?.ident.to_string();
		if struct_name != "GeneratedColumnDefinition" {
			return None;
		}

		let expr_tokens = extract_optional_str_field(&expr_struct.fields, "expr_tokens")
			.or_else(|| extract_optional_expr_tokens_field(&expr_struct.fields, "expr"));
		let expr = extract_optional_schema_expr_field(&expr_struct.fields, "expr")
			.or_else(|| expr_tokens.as_deref().and_then(parse_schema_expr_tokens));
		let raw_sql = extract_optional_str_field(&expr_struct.fields, "raw_sql");
		let storage = extract_generated_storage_field(&expr_struct.fields)
			.unwrap_or(super::GeneratedStorage::Stored);

		return Some(super::GeneratedColumnDefinition {
			expr: expr.map(Box::new),
			expr_tokens,
			raw_sql,
			storage,
		});
	}

	None
}

pub(crate) fn parse_schema_expr_tokens(tokens: &str) -> Option<super::SchemaExpr> {
	syn::parse_str::<Expr>(tokens)
		.ok()
		.and_then(|expr| parse_schema_expr(&expr))
}

fn extract_optional_schema_expr_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
	field_name: &str,
) -> Option<super::SchemaExpr> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == field_name
		{
			return parse_optional_schema_expr(&field.expr);
		}
	}
	None
}

fn parse_optional_schema_expr(expr: &Expr) -> Option<super::SchemaExpr> {
	if let Expr::Path(expr_path) = expr
		&& expr_path.path.is_ident("None")
	{
		return None;
	}

	if let Expr::Call(expr_call) = expr
		&& let Expr::Path(func_path) = &*expr_call.func
		&& func_path.path.is_ident("Some")
		&& expr_call.args.len() == 1
	{
		let expr = unwrap_box_new_expr(&expr_call.args[0]).unwrap_or(&expr_call.args[0]);
		return parse_schema_expr(expr);
	}

	None
}

fn parse_schema_expr(expr: &Expr) -> Option<super::SchemaExpr> {
	match expr {
		Expr::Paren(expr_paren) => parse_schema_expr(&expr_paren.expr),
		Expr::Group(expr_group) => parse_schema_expr(&expr_group.expr),
		Expr::MethodCall(method_call) => {
			let receiver = parse_schema_expr(&method_call.receiver)?;
			match method_call.method.to_string().as_str() {
				"binary" if method_call.args.len() == 2 => {
					let op = parse_schema_bin_oper(&method_call.args[0])?;
					let right = parse_schema_expr(&method_call.args[1])?;
					Some(receiver.binary(op, right))
				}
				"cast" if method_call.args.len() == 1 => {
					let ty = parse_query_column_type(&method_call.args[0])?;
					Some(receiver.cast(ty))
				}
				_ => None,
			}
		}
		Expr::Call(expr_call) => {
			let Expr::Path(func_path) = &*expr_call.func else {
				return None;
			};
			let func = func_path.path.segments.last()?.ident.to_string();
			match func.as_str() {
				"col" if expr_call.args.len() == 1 => {
					let name = extract_string_literal(&expr_call.args[0])?;
					Some(super::SchemaExpr::col(name))
				}
				"val" if expr_call.args.len() == 1 => {
					let value = parse_schema_value(&expr_call.args[0])?;
					Some(super::SchemaExpr::Value(value))
				}
				"concat" if expr_call.args.len() == 1 => {
					let args = parse_schema_expr_items(&expr_call.args[0])?;
					Some(super::SchemaExpr::concat(args))
				}
				"coalesce" if expr_call.args.len() == 1 => {
					let args = parse_schema_expr_items(&expr_call.args[0])?;
					Some(super::SchemaExpr::coalesce(args))
				}
				_ => None,
			}
		}
		_ => None,
	}
}

fn parse_schema_expr_items(expr: &Expr) -> Option<Vec<super::SchemaExpr>> {
	match expr {
		Expr::Array(expr_array) => expr_array.elems.iter().map(parse_schema_expr).collect(),
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			let parsed = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }).ok()?;
			parsed.elems.iter().map(parse_schema_expr).collect()
		}
		_ => None,
	}
}

fn parse_schema_bin_oper(expr: &Expr) -> Option<super::SchemaBinOper> {
	let Expr::Path(expr_path) = expr else {
		return None;
	};
	match expr_path.path.segments.last()?.ident.to_string().as_str() {
		"Add" => Some(super::SchemaBinOper::Add),
		"Sub" => Some(super::SchemaBinOper::Sub),
		"Mul" => Some(super::SchemaBinOper::Mul),
		"Div" => Some(super::SchemaBinOper::Div),
		_ => None,
	}
}

fn parse_schema_value(expr: &Expr) -> Option<QueryValue> {
	match expr {
		Expr::Path(expr_path) => parse_option_none_schema_value(&expr_path.path),
		Expr::Lit(expr_lit) => match &expr_lit.lit {
			syn::Lit::Str(lit_str) => Some(QueryValue::String(Some(Box::new(lit_str.value())))),
			syn::Lit::Bool(lit_bool) => Some(QueryValue::Bool(Some(lit_bool.value))),
			syn::Lit::Char(lit_char) => Some(QueryValue::Char(Some(lit_char.value()))),
			syn::Lit::Int(lit_int) => lit_int
				.base10_parse::<i32>()
				.map(|value| QueryValue::Int(Some(value)))
				.or_else(|_| {
					lit_int
						.base10_parse::<i64>()
						.map(|value| QueryValue::BigInt(Some(value)))
				})
				.ok(),
			syn::Lit::Float(lit_float) => lit_float
				.base10_parse::<f64>()
				.map(|value| QueryValue::Double(Some(value)))
				.ok(),
			_ => None,
		},
		Expr::Unary(expr_unary) => {
			if !matches!(expr_unary.op, syn::UnOp::Neg(_)) {
				return None;
			}
			match parse_schema_value(&expr_unary.expr)? {
				QueryValue::Int(Some(value)) => Some(QueryValue::Int(Some(-value))),
				QueryValue::BigInt(Some(value)) => Some(QueryValue::BigInt(Some(-value))),
				QueryValue::Float(Some(value)) => Some(QueryValue::Float(Some(-value))),
				QueryValue::Double(Some(value)) => Some(QueryValue::Double(Some(-value))),
				_ => None,
			}
		}
		_ => None,
	}
}

fn parse_option_none_schema_value(path: &syn::Path) -> Option<QueryValue> {
	let mut segments = path.segments.iter();
	let option_segment = segments.next()?;
	let none_segment = segments.next()?;
	if segments.next().is_some() || option_segment.ident != "Option" || none_segment.ident != "None"
	{
		return None;
	}

	let syn::PathArguments::AngleBracketed(arguments) = &option_segment.arguments else {
		return None;
	};
	if arguments.args.len() != 1 {
		return None;
	}
	let Some(syn::GenericArgument::Type(syn::Type::Path(type_path))) = arguments.args.first()
	else {
		return None;
	};
	let ident = type_path.path.segments.last()?.ident.to_string();
	match ident.as_str() {
		"bool" => Some(QueryValue::Bool(None)),
		"i8" => Some(QueryValue::TinyInt(None)),
		"i16" => Some(QueryValue::SmallInt(None)),
		"i32" => Some(QueryValue::Int(None)),
		"i64" => Some(QueryValue::BigInt(None)),
		"u8" => Some(QueryValue::TinyUnsigned(None)),
		"u16" => Some(QueryValue::SmallUnsigned(None)),
		"u32" => Some(QueryValue::Unsigned(None)),
		"u64" => Some(QueryValue::BigUnsigned(None)),
		"f32" => Some(QueryValue::Float(None)),
		"f64" => Some(QueryValue::Double(None)),
		"char" => Some(QueryValue::Char(None)),
		"String" => Some(QueryValue::String(None)),
		_ => None,
	}
}

fn parse_query_column_type(expr: &Expr) -> Option<super::ColumnType> {
	match expr {
		Expr::Path(expr_path) => match expr_path.path.segments.last()?.ident.to_string().as_str() {
			"Text" => Some(super::ColumnType::Text),
			"TinyInteger" => Some(super::ColumnType::TinyInteger),
			"SmallInteger" => Some(super::ColumnType::SmallInteger),
			"Integer" => Some(super::ColumnType::Integer),
			"BigInteger" => Some(super::ColumnType::BigInteger),
			"Float" => Some(super::ColumnType::Float),
			"Double" => Some(super::ColumnType::Double),
			"Boolean" => Some(super::ColumnType::Boolean),
			"Date" => Some(super::ColumnType::Date),
			"Time" => Some(super::ColumnType::Time),
			"DateTime" => Some(super::ColumnType::DateTime),
			"Timestamp" => Some(super::ColumnType::Timestamp),
			"TimestampWithTimeZone" => Some(super::ColumnType::TimestampWithTimeZone),
			"Blob" => Some(super::ColumnType::Blob),
			"Uuid" => Some(super::ColumnType::Uuid),
			"Json" => Some(super::ColumnType::Json),
			"JsonBinary" => Some(super::ColumnType::JsonBinary),
			_ => None,
		},
		Expr::Call(expr_call) => {
			let Expr::Path(func_path) = &*expr_call.func else {
				return None;
			};
			let variant = func_path.path.segments.last()?.ident.to_string();
			match variant.as_str() {
				"Char" if expr_call.args.len() == 1 => Some(super::ColumnType::Char(
					parse_optional_u32(&expr_call.args[0])?,
				)),
				"String" if expr_call.args.len() == 1 => Some(super::ColumnType::String(
					parse_optional_u32(&expr_call.args[0])?,
				)),
				"Decimal" if expr_call.args.len() == 1 => Some(super::ColumnType::Decimal(
					parse_optional_u32_pair(&expr_call.args[0])?,
				)),
				"Binary" if expr_call.args.len() == 1 => Some(super::ColumnType::Binary(
					parse_optional_u32(&expr_call.args[0])?,
				)),
				"VarBinary" if expr_call.args.len() == 1 => Some(super::ColumnType::VarBinary(
					parse_u32_literal(&expr_call.args[0])?,
				)),
				"Array" if expr_call.args.len() == 1 => {
					let inner = unwrap_box_new_expr(&expr_call.args[0])?;
					Some(super::ColumnType::Array(Box::new(parse_query_column_type(
						inner,
					)?)))
				}
				"Custom" if expr_call.args.len() == 1 => Some(super::ColumnType::Custom(
					extract_string_literal(&expr_call.args[0])?,
				)),
				_ => None,
			}
		}
		_ => None,
	}
}

fn parse_optional_u32(expr: &Expr) -> Option<Option<u32>> {
	if let Expr::Path(expr_path) = expr
		&& expr_path.path.is_ident("None")
	{
		return Some(None);
	}
	if let Expr::Call(expr_call) = expr
		&& let Expr::Path(func_path) = &*expr_call.func
		&& func_path.path.is_ident("Some")
		&& expr_call.args.len() == 1
	{
		return Some(Some(parse_u32_literal(&expr_call.args[0])?));
	}
	None
}

fn parse_optional_u32_pair(expr: &Expr) -> Option<Option<(u32, u32)>> {
	if let Expr::Path(expr_path) = expr
		&& expr_path.path.is_ident("None")
	{
		return Some(None);
	}
	if let Expr::Call(expr_call) = expr
		&& let Expr::Path(func_path) = &*expr_call.func
		&& func_path.path.is_ident("Some")
		&& expr_call.args.len() == 1
		&& let Expr::Tuple(tuple) = &expr_call.args[0]
		&& tuple.elems.len() == 2
	{
		return Some(Some((
			parse_u32_literal(&tuple.elems[0])?,
			parse_u32_literal(&tuple.elems[1])?,
		)));
	}
	None
}

fn parse_u32_literal(expr: &Expr) -> Option<u32> {
	let Expr::Lit(expr_lit) = expr else {
		return None;
	};
	let syn::Lit::Int(lit_int) = &expr_lit.lit else {
		return None;
	};
	lit_int.base10_parse::<u32>().ok()
}

fn unwrap_box_new_expr(expr: &Expr) -> Option<&Expr> {
	let Expr::Call(expr_call) = expr else {
		return None;
	};
	let Expr::Path(func_path) = &*expr_call.func else {
		return None;
	};
	if func_path
		.path
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "new")
		&& func_path
			.path
			.segments
			.iter()
			.any(|segment| segment.ident == "Box")
		&& expr_call.args.len() == 1
	{
		return Some(&expr_call.args[0]);
	}
	None
}

fn extract_generated_storage_field(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
) -> Option<super::GeneratedStorage> {
	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == "storage"
			&& let Expr::Path(expr_path) = &field.expr
			&& let Some(last_segment) = expr_path.path.segments.last()
		{
			return match last_segment.ident.to_string().as_str() {
				"Stored" => Some(super::GeneratedStorage::Stored),
				"Virtual" => Some(super::GeneratedStorage::Virtual),
				_ => None,
			};
		}
	}
	None
}

/// Extract a field value from Migration struct literal
fn extract_field_from_migration_struct(expr: &Expr, field_name: &str) -> Option<Expr> {
	if let Expr::Struct(expr_struct) = expr {
		// Check if this is a Migration struct
		if expr_struct.path.segments.last()?.ident == "Migration" {
			// Find the field we're looking for
			for field in &expr_struct.fields {
				if let syn::Member::Named(ident) = &field.member
					&& ident == field_name
				{
					return Some(field.expr.clone());
				}
			}
		}
	}
	None
}

/// Parse a vec![...] or array expression containing tuples of strings
fn parse_tuple_vec_expr(expr: &Expr) -> Result<Vec<(String, String)>> {
	let mut result = Vec::new();

	match expr {
		// Handle vec![...] macro
		Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
			let tokens = &expr_macro.mac.tokens;
			// Wrap tokens in array brackets so syn can parse comma-separated items
			if let Ok(parsed) = syn::parse2::<syn::ExprArray>(quote::quote! { [#tokens] }) {
				for item in &parsed.elems {
					if let Some(tuple) = extract_string_tuple(item) {
						result.push(tuple);
					}
				}
			}
		}
		// Handle array literal [...]
		Expr::Array(expr_array) => {
			for item in &expr_array.elems {
				if let Some(tuple) = extract_string_tuple(item) {
					result.push(tuple);
				}
			}
		}
		_ => {}
	}

	Ok(result)
}

/// Extract a tuple of two strings from an expression like ("app", "name")
fn extract_string_tuple(expr: &Expr) -> Option<(String, String)> {
	if let Expr::Tuple(expr_tuple) = expr
		&& expr_tuple.elems.len() == 2
	{
		let first = extract_string_literal(&expr_tuple.elems[0])?;
		let second = extract_string_literal(&expr_tuple.elems[1])?;
		return Some((first, second));
	}
	None
}

/// Extract string value from a literal expression or `.to_string()` method call
fn extract_string_literal(expr: &Expr) -> Option<String> {
	// Handle direct string literal: "foo"
	if let Expr::Lit(expr_lit) = expr
		&& let syn::Lit::Str(lit_str) = &expr_lit.lit
	{
		return Some(lit_str.value());
	}
	// Handle "foo".to_string() pattern
	if let Expr::MethodCall(method_call) = expr
		&& method_call.method == "to_string"
	{
		return extract_string_literal(&method_call.receiver);
	}
	None
}

/// Helper to parse `true` or `false` return
fn parse_bool_return(func: &ItemFn) -> Option<bool> {
	if let Some(Stmt::Expr(Expr::Lit(expr_lit), _)) = func.block.stmts.last()
		&& let syn::Lit::Bool(lit_bool) = &expr_lit.lit
	{
		return Some(lit_bool.value);
	}
	None
}

#[cfg(test)]
mod tests {
	use super::extract_migration_metadata;
	use crate::migrations::{GeneratedStorage, Operation, SchemaExpr};

	#[test]
	fn extract_migration_metadata_restores_typed_generated_expression() {
		let source = r#"
use reinhardt_db::migrations::prelude::*;

pub(super) fn migration() -> Migration {
	Migration {
		app_label: "accounts".to_string(),
		name: "0002_full_name".to_string(),
		operations: vec![
			Operation::AddColumn {
				table: "users".to_string(),
				column: ColumnDefinition {
					name: "full_name".to_string(),
					type_definition: FieldType::VarChar(201),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					generated: Some(GeneratedColumnDefinition {
						expr: Some(Box::new(SchemaExpr::concat([
							SchemaExpr::col("first_name"),
							SchemaExpr::val(" "),
							SchemaExpr::col("last_name"),
						]))),
						expr_tokens: Some("SchemaExpr::concat([SchemaExpr::col(\"first_name\"), SchemaExpr::val(\" \"), SchemaExpr::col(\"last_name\")])".to_string()),
						raw_sql: None,
						storage: GeneratedStorage::Stored,
					}),
				},
				mysql_options: None,
			},
		],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
"#;
		let ast = syn::parse_file(source).expect("migration source must parse");

		let migration = extract_migration_metadata(&ast, "accounts", "0002_full_name")
			.expect("migration metadata must parse");

		let Operation::AddColumn { column, .. } = &migration.operations[0] else {
			panic!("expected AddColumn operation");
		};
		let generated = column
			.generated
			.as_ref()
			.expect("generated metadata must be restored");
		assert_eq!(generated.storage, GeneratedStorage::Stored);
		assert_eq!(
			generated.expr.as_deref(),
			Some(&SchemaExpr::concat([
				SchemaExpr::col("first_name"),
				SchemaExpr::val(" "),
				SchemaExpr::col("last_name"),
			]))
		);
	}
}

/// Extract FieldType from type_definition field
fn extract_field_type(
	fields: &syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma>,
) -> Option<super::FieldType> {
	use super::FieldType;

	for field in fields {
		if let syn::Member::Named(ident) = &field.member
			&& ident == "type_definition"
		{
			// Handle FieldType::Variant or path::to::FieldType::Variant
			if let Expr::Path(expr_path) = &field.expr {
				let segments: Vec<_> = expr_path
					.path
					.segments
					.iter()
					.map(|s| s.ident.to_string())
					.collect();

				// Get the last segment as the variant name
				if let Some(last_segment) = expr_path.path.segments.last() {
					let variant = last_segment.ident.to_string();

					return match variant.as_str() {
						"Integer" => Some(FieldType::Integer),
						"BigInteger" => Some(FieldType::BigInteger),
						"SmallInteger" => Some(FieldType::SmallInteger),
						"TinyInt" => Some(FieldType::TinyInt),
						"MediumInt" => Some(FieldType::MediumInt),
						"Text" => Some(FieldType::Text),
						"TinyText" => Some(FieldType::TinyText),
						"MediumText" => Some(FieldType::MediumText),
						"LongText" => Some(FieldType::LongText),
						"Date" => Some(FieldType::Date),
						"Time" => Some(FieldType::Time),
						"DateTime" => Some(FieldType::DateTime),
						"TimestampTz" => Some(FieldType::TimestampTz),
						"Float" => Some(FieldType::Float),
						"Double" => Some(FieldType::Double),
						"Real" => Some(FieldType::Real),
						"Boolean" => Some(FieldType::Boolean),
						"Binary" => Some(FieldType::Binary),
						"Blob" => Some(FieldType::Blob),
						"TinyBlob" => Some(FieldType::TinyBlob),
						"MediumBlob" => Some(FieldType::MediumBlob),
						"LongBlob" => Some(FieldType::LongBlob),
						"Bytea" => Some(FieldType::Bytea),
						"Json" => Some(FieldType::Json),
						"JsonBinary" => Some(FieldType::JsonBinary),
						"Uuid" => Some(FieldType::Uuid),
						"Year" => Some(FieldType::Year),
						_ => Some(FieldType::Custom(segments.join("::"))),
					};
				}
			}
			// Handle FieldType::VarChar(n) or FieldType::Char(n)
			else if let Expr::Call(expr_call) = &field.expr {
				if let Expr::Path(func_path) = &*expr_call.func
					&& let Some(last_segment) = func_path.path.segments.last()
				{
					let variant = last_segment.ident.to_string();

					if !expr_call.args.is_empty()
						&& let Expr::Lit(expr_lit) = &expr_call.args[0]
						&& let syn::Lit::Int(lit_int) = &expr_lit.lit
						&& let Ok(size) = lit_int.base10_parse::<u32>()
					{
						return match variant.as_str() {
							"VarChar" => Some(FieldType::VarChar(size)),
							"Char" => Some(FieldType::Char(size)),
							_ => None,
						};
					}
				}
			}
			// Handle FieldType::Decimal { precision, scale }
			// Handle FieldType::OneToOne { to, on_delete, on_update }
			// Handle FieldType::ManyToMany { to, through }
			else if let Expr::Struct(expr_struct) = &field.expr {
				if let Some(last_segment) = expr_struct.path.segments.last() {
					let variant = last_segment.ident.to_string();

					match variant.as_str() {
						"Decimal" => {
							let mut precision = 10u32;
							let mut scale = 0u32;

							for field_value in &expr_struct.fields {
								if let syn::Member::Named(field_ident) = &field_value.member
									&& let Expr::Lit(expr_lit) = &field_value.expr
									&& let syn::Lit::Int(lit_int) = &expr_lit.lit
									&& let Ok(val) = lit_int.base10_parse::<u32>()
								{
									if field_ident == "precision" {
										precision = val;
									} else if field_ident == "scale" {
										scale = val;
									}
								}
							}

							return Some(FieldType::Decimal { precision, scale });
						}
						"OneToOne" => {
							// Extract required field: to
							let to = extract_string_field(&expr_struct.fields, "to")?;

							// Extract optional fields with defaults
							let on_delete =
								extract_foreign_key_action_field(&expr_struct.fields, "on_delete")
									.unwrap_or(super::ForeignKeyAction::Restrict);
							let on_update =
								extract_foreign_key_action_field(&expr_struct.fields, "on_update")
									.unwrap_or(super::ForeignKeyAction::NoAction);

							return Some(FieldType::OneToOne {
								to,
								on_delete,
								on_update,
							});
						}
						"ManyToMany" => {
							// Extract required field: to
							let to = extract_string_field(&expr_struct.fields, "to")?;

							// Extract optional field: through
							let through =
								extract_optional_str_field(&expr_struct.fields, "through");

							return Some(FieldType::ManyToMany { to, through });
						}
						_ => {}
					}
				}
			}
			// Handle FieldType::Custom("...")
			else if let Expr::Call(expr_call) = &field.expr
				&& let Expr::Path(func_path) = &*expr_call.func
				&& let Some(last_segment) = func_path.path.segments.last()
				&& last_segment.ident == "Custom"
				&& !expr_call.args.is_empty()
				&& let Some(s) = extract_string_literal(&expr_call.args[0])
			{
				return Some(FieldType::Custom(s));
			}
		}
	}
	None
}
