//! Independent syntax for rendering a named ClientForm's existing mutation.

use syn::{Expr, Ident, Path};

/// A named ClientForm view, preserving presentation expression order.
#[derive(Clone, Debug)]
pub struct ClientFormViewMacro {
	/// Path to the generated ClientForm companion.
	pub companion: Path,
	/// Configuration in source order, excluding the initial companion path.
	pub clauses: Vec<ClientFormViewClause>,
}

impl ClientFormViewMacro {
	/// Returns the mutation expression required by the parser.
	pub fn mutation(&self) -> &Expr {
		self.clauses
			.iter()
			.find_map(|clause| match clause {
				ClientFormViewClause::Mutation(value) => Some(value),
				_ => None,
			})
			.expect("a parsed ClientForm view has a mutation")
	}

	/// Returns customized Rust field names in source order.
	pub fn field_names(&self) -> Vec<String> {
		self.clauses
			.iter()
			.flat_map(|clause| match clause {
				ClientFormViewClause::Customize(fields) => fields.as_slice(),
				_ => &[],
			})
			.map(|field| field.field.to_string().trim_start_matches("r#").to_owned())
			.collect()
	}
}

/// One configuration clause in a named ClientForm view.
#[derive(Clone, Debug)]
pub enum ClientFormViewClause {
	/// Reference to the application-owned mutation.
	Mutation(Expr),
	/// Explicit form instance ID.
	Id(Expr),
	/// Shared presentation classes.
	Styling(Vec<PresentationProperty>),
	/// Typed field presentation overrides.
	Customize(Vec<FieldCustomization>),
	/// Submit button labels and class.
	Submit(Vec<PresentationProperty>),
	/// Validation summary label.
	Summary(Vec<PresentationProperty>),
}

/// A presentation property with its original expression and source span.
#[derive(Clone, Debug)]
pub struct PresentationProperty {
	/// Property name.
	pub name: Ident,
	/// Expression evaluated once during view assembly.
	pub value: Expr,
}

/// Partial presentation overrides for one exposed Rust DTO field.
#[derive(Clone, Debug)]
pub struct FieldCustomization {
	/// Rust field identifier, which may be raw.
	pub field: Ident,
	/// Overrides in source order.
	pub properties: Vec<PresentationProperty>,
}
