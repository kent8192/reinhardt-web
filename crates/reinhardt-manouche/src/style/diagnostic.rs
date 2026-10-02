//! Stable diagnostics shared by macro and source-extractor validation.

use std::fmt::{self, Display, Formatter};

use proc_macro2::Span;

/// Stable categories for every hard style-compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleDiagnosticKind {
	/// Rust-token style syntax could not be parsed.
	Syntax {
		/// Stable parser message without source-path information.
		message: String,
	},
	/// The attribute was attached to an unsupported Rust item envelope.
	UnsupportedStyleDefEnvelope {
		/// A short description of the encountered envelope.
		found: String,
	},
	/// A `style!` invocation was expanded without `#[style_def]`.
	StandaloneStyleExpansion,
	/// A top-level selector branch did not begin with a local class.
	UnanchoredTopLevelSelector {
		/// The rejected selector text.
		selector: String,
	},
	/// A declaration inside a top-level grouping rule had no inherited local selector.
	UnanchoredTopLevelDeclaration {
		/// The rejected canonical property name.
		property: String,
	},
	/// A selector head used an ambiguous flat form.
	AmbiguousFlatSelector {
		/// The rejected selector text.
		selector: String,
		/// The structurally parsed local-class anchor for the supported rewrite.
		anchor: String,
	},
	/// Multiple classes lower to the same generated Rust accessor.
	ClassAccessorCollision {
		/// The colliding generated accessor.
		accessor: String,
	},
	/// A local class is not an ASCII CSS identifier suitable for deterministic lowering.
	InvalidClassName {
		/// The rejected authored class name.
		name: String,
	},
	/// A local class lowers to a reserved Rust accessor name.
	ClassAccessorKeyword {
		/// The authored CSS class name.
		class_name: String,
		/// The rejected generated accessor.
		accessor: String,
	},
	/// A local class lowers to a method reserved by the generated style API.
	ClassAccessorReserved {
		/// The authored CSS class name.
		class_name: String,
		/// The rejected generated accessor.
		accessor: String,
	},
	/// A global binding name is not an ordinary ASCII snake-case identifier.
	InvalidGlobalName {
		/// The rejected authored binding name.
		name: String,
	},
	/// A component-variable name is not an ordinary ASCII snake-case identifier.
	InvalidVariableName {
		/// The rejected authored binding name.
		name: String,
	},
	/// A name was repeated in the global namespace.
	DuplicateGlobal {
		/// The duplicate name.
		name: String,
	},
	/// A name was repeated in the component-variable namespace.
	DuplicateVariable {
		/// The duplicate name.
		name: String,
	},
	/// A `globals` reference has no declaration.
	UndeclaredGlobalReference {
		/// The referenced name.
		name: String,
	},
	/// A `vars` reference has no declaration.
	UndeclaredVariableReference {
		/// The referenced name.
		name: String,
	},
	/// A component variable omitted its required default.
	MissingVariableDefault {
		/// The variable name.
		name: String,
	},
	/// Component-variable defaults form a dependency cycle.
	VariableDependencyCycle {
		/// The dependency chain, including the repeated closing name.
		names: Vec<String>,
	},
	/// A global or component variable declared a type outside the closed vocabulary.
	UnknownStyleType {
		/// The rejected DSL type name.
		name: String,
	},
	/// A numeric literal used a unit outside the checked registry.
	UnknownUnit {
		/// The rejected unit suffix.
		name: String,
	},
	/// A value does not satisfy its property's grammar.
	PropertyValueMismatch {
		/// The canonical CSS property name.
		property: String,
		/// A description of the accepted grammar.
		expected: String,
		/// The inferred value type.
		found: String,
	},
	/// Arithmetic operands use incompatible dimensions.
	InvalidArithmeticDimensions {
		/// The arithmetic operator.
		operation: String,
		/// The left operand dimension.
		left: String,
		/// The right operand dimension.
		right: String,
	},
	/// A declaration used an unregistered property.
	UnknownProperty {
		/// The rejected property name.
		name: String,
	},
	/// A call used an unregistered function.
	UnknownFunction {
		/// The rejected function path or name.
		name: String,
	},
	/// A value invoked `var(...)` directly.
	DirectVarCall,
	/// A value invoked `calc(...)` directly.
	DirectCalcCall,
	/// A function argument did not satisfy its constraint.
	InvalidFunctionArgument {
		/// The function path or name.
		function: String,
		/// The one-based argument position.
		index: usize,
		/// The expected constraint.
		expected: String,
		/// The inferred argument type.
		found: String,
	},
	/// A checked function was called with the wrong number of arguments.
	InvalidFunctionArity {
		/// The registered function path or method name.
		function: String,
		/// Stable description of the accepted arity.
		expected: String,
		/// Authored argument count.
		found: usize,
	},
	/// The unchecked escape appeared inside a checked expression container.
	InvalidUncheckedPlacement {
		/// The checked context that cannot contain an unchecked value.
		context: String,
	},
	/// A method is unavailable for its receiver type.
	InvalidReceiverMethod {
		/// The inferred receiver type.
		receiver: String,
		/// The rejected method name.
		method: String,
	},
	/// Two definitions have the same package-level style identity.
	DuplicatePackageStyleIdentity {
		/// The duplicate package-relative identity.
		identity: String,
	},
	/// Distinct style identities produced the same shortened scope hash.
	ShortenedScopeHashCollision {
		/// The colliding shortened hash.
		hash: String,
	},
	/// A physical file occupies the generated asset's reserved path.
	ReservedGeneratedAssetCollision {
		/// The colliding project-relative path.
		path: String,
	},
}

impl Display for StyleDiagnosticKind {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Syntax { message } => write!(formatter, "style syntax error: {message}"),
			Self::UnsupportedStyleDefEnvelope { found } => write!(
				formatter,
				"unsupported #[style_def] envelope `{found}`; use `#[style_def] static NAME: Style = style! {{ ... }};`"
			),
			Self::StandaloneStyleExpansion => write!(
				formatter,
				"style! cannot expand on its own; place it in `#[style_def] static NAME: Style = style! {{ ... }};`"
			),
			Self::UnanchoredTopLevelSelector { selector } => write!(
				formatter,
				"top-level selector `{selector}` is not anchored to a local class; start every branch with `.local-class`"
			),
			Self::UnanchoredTopLevelDeclaration { property } => write!(
				formatter,
				"declaration `{property}` inside a top-level grouping rule has no local-class anchor; put it inside a local class rule"
			),
			Self::AmbiguousFlatSelector { selector, anchor } => write!(
				formatter,
				"selector `{selector}` is flat or ambiguous; write the descendant as a nested selector under `{anchor}`"
			),
			Self::ClassAccessorCollision { accessor } => write!(
				formatter,
				"multiple classes generate accessor `{accessor}`; rename one class so every generated method is unique"
			),
			Self::InvalidClassName { name } => write!(
				formatter,
				"class `.{name}` is not an ASCII CSS identifier; use ASCII letters, digits, `_`, or `-` without a leading digit"
			),
			Self::ClassAccessorKeyword {
				class_name,
				accessor,
			} => write!(
				formatter,
				"class `.{class_name}` generates reserved Rust accessor `{accessor}`; rename the class to produce a non-keyword method name"
			),
			Self::ClassAccessorReserved {
				class_name,
				accessor,
			} => write!(
				formatter,
				"class `.{class_name}` generates style API accessor `{accessor}`, which is reserved; rename the class to produce a different method name"
			),
			Self::InvalidGlobalName { name } => write!(
				formatter,
				"global name `{name}` is invalid; use a non-keyword ASCII snake_case identifier matching `[a-z][a-z0-9]*(?:_[a-z0-9]+)*`"
			),
			Self::InvalidVariableName { name } => write!(
				formatter,
				"component variable name `{name}` is invalid; use a non-keyword ASCII snake_case identifier matching `[a-z][a-z0-9]*(?:_[a-z0-9]+)*`"
			),
			Self::DuplicateGlobal { name } => {
				write!(
					formatter,
					"duplicate global `{name}`; global names must be unique"
				)
			}
			Self::DuplicateVariable { name } => write!(
				formatter,
				"duplicate component variable `{name}`; variable names must be unique"
			),
			Self::UndeclaredGlobalReference { name } => write!(
				formatter,
				"undeclared global reference `globals.{name}`; declare it in the `globals` block"
			),
			Self::UndeclaredVariableReference { name } => write!(
				formatter,
				"undeclared component variable reference `vars.{name}`; declare it in the `vars` block"
			),
			Self::MissingVariableDefault { name } => write!(
				formatter,
				"component variable `{name}` is missing a default; add `= <typed value>`"
			),
			Self::VariableDependencyCycle { names } => write!(
				formatter,
				"component-variable dependency cycle: {}; break the cycle with an independent typed default",
				names.join(" -> ")
			),
			Self::UnknownStyleType { name } => write!(
				formatter,
				"unknown style type `{name}`; use Color, Length, LengthPercentage, Percentage, Angle, Time, Number, or Integer"
			),
			Self::UnknownUnit { name } => write!(
				formatter,
				"unknown style unit `{name}`; use a unit from the checked style registry"
			),
			Self::PropertyValueMismatch {
				property,
				expected,
				found,
			} => write!(
				formatter,
				"value for property `{property}` has type `{found}`; expected {expected}"
			),
			Self::InvalidArithmeticDimensions {
				operation,
				left,
				right,
			} => write!(
				formatter,
				"invalid dimensions for `{operation}`: `{left}` and `{right}` cannot be combined"
			),
			Self::UnknownProperty { name } => write!(
				formatter,
				"unknown property `{name}`; use a registered canonical CSS property name"
			),
			Self::UnknownFunction { name } => write!(
				formatter,
				"unknown style function `{name}`; use a registered function or wrap one balanced call in `unchecked_fn!(...)`"
			),
			Self::DirectVarCall => write!(
				formatter,
				"direct `var(...)` is unsupported; declare a typed `global` or component `var` and reference it"
			),
			Self::DirectCalcCall => write!(
				formatter,
				"direct `calc(...)` is unsupported; write arithmetic with style operators"
			),
			Self::InvalidFunctionArgument {
				function,
				index,
				expected,
				found,
			} => write!(
				formatter,
				"argument {index} to `{function}` has type `{found}`; expected {expected}"
			),
			Self::InvalidFunctionArity {
				function,
				expected,
				found,
			} => write!(
				formatter,
				"function `{function}` received {found} arguments; expected {expected}"
			),
			Self::InvalidUncheckedPlacement { context } => write!(
				formatter,
				"unchecked_fn! is only valid as an entire property value or typed variable default, not inside {context}"
			),
			Self::InvalidReceiverMethod { receiver, method } => write!(
				formatter,
				"method `{method}` is not available on `{receiver}`; use a method registered for that receiver type"
			),
			Self::DuplicatePackageStyleIdentity { identity } => write!(
				formatter,
				"duplicate package style identity `{identity}`; each style definition must have a unique module path and static name"
			),
			Self::ShortenedScopeHashCollision { hash } => write!(
				formatter,
				"shortened style scope hash collision `{hash}`; rename one style definition to produce a distinct identity"
			),
			Self::ReservedGeneratedAssetCollision { path } => write!(
				formatter,
				"physical file `{path}` collides with the reserved generated style asset; remove or rename the file"
			),
		}
	}
}

/// A secondary span and its relationship to the primary diagnostic.
#[derive(Debug, Clone)]
pub struct StyleRelatedLabel {
	/// The related source span.
	pub span: Span,
	/// A short explanation of the span's contribution.
	pub reason: String,
}

/// A style diagnostic with a primary span and any contributing spans.
#[derive(Debug, Clone)]
pub struct StyleDiagnostic {
	/// The stable diagnostic category and its message data.
	pub kind: StyleDiagnosticKind,
	/// The primary source span.
	pub primary_span: Span,
	/// Additional declarations or references that contributed to the error.
	pub related: Vec<StyleRelatedLabel>,
}

impl StyleDiagnostic {
	/// Creates a diagnostic without related labels.
	pub fn new(kind: StyleDiagnosticKind, primary_span: Span) -> Self {
		Self {
			kind,
			primary_span,
			related: Vec::new(),
		}
	}

	/// Converts one parser error into the stable style diagnostic boundary.
	pub(crate) fn from_syn_error(error: syn::Error) -> Self {
		let primary_span = error.span();
		Self::new(
			StyleDiagnosticKind::Syntax {
				message: error.to_string(),
			},
			primary_span,
		)
	}

	/// Appends a related span and explanation.
	pub fn with_related(mut self, span: Span, reason: impl Into<String>) -> Self {
		self.related.push(StyleRelatedLabel {
			span,
			reason: reason.into(),
		});
		self
	}

	/// Converts the diagnostic and every related label into one `syn::Error`.
	pub fn into_syn_error(self) -> syn::Error {
		let mut error = syn::Error::new(self.primary_span, self.kind.to_string());
		for related in self.related {
			error.combine(syn::Error::new(related.span, related.reason));
		}
		error
	}
}

impl Display for StyleDiagnostic {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		Display::fmt(&self.kind, formatter)
	}
}

#[cfg(test)]
mod tests {
	use proc_macro2::Span;
	use quote::quote;
	use rstest::rstest;

	use super::{StyleDiagnostic, StyleDiagnosticKind};

	#[rstest]
	#[case(
		StyleDiagnosticKind::Syntax { message: "expected a style value expression".into() },
		"style syntax error: expected a style value expression"
	)]
	#[case(
		StyleDiagnosticKind::UnsupportedStyleDefEnvelope { found: "const item".into() },
		"unsupported #[style_def] envelope `const item`; use `#[style_def] static NAME: Style = style! { ... };`"
	)]
	#[case(
		StyleDiagnosticKind::StandaloneStyleExpansion,
		"style! cannot expand on its own; place it in `#[style_def] static NAME: Style = style! { ... };`"
	)]
	#[case(
		StyleDiagnosticKind::UnanchoredTopLevelSelector { selector: "button".into() },
		"top-level selector `button` is not anchored to a local class; start every branch with `.local-class`"
	)]
	#[case(
		StyleDiagnosticKind::UnanchoredTopLevelDeclaration { property: "color".into() },
		"declaration `color` inside a top-level grouping rule has no local-class anchor; put it inside a local class rule"
	)]
	#[case(
		StyleDiagnosticKind::AmbiguousFlatSelector {
			selector: ".card button".into(),
			anchor: ".card".into(),
		},
		"selector `.card button` is flat or ambiguous; write the descendant as a nested selector under `.card`"
	)]
	#[case(
		StyleDiagnosticKind::AmbiguousFlatSelector {
			selector: ".panel button".into(),
			anchor: ".panel".into(),
		},
		"selector `.panel button` is flat or ambiguous; write the descendant as a nested selector under `.panel`"
	)]
	#[case(
		StyleDiagnosticKind::ClassAccessorCollision { accessor: "foo_bar".into() },
		"multiple classes generate accessor `foo_bar`; rename one class so every generated method is unique"
	)]
	#[case(
		StyleDiagnosticKind::InvalidClassName { name: "123".into() },
		"class `.123` is not an ASCII CSS identifier; use ASCII letters, digits, `_`, or `-` without a leading digit"
	)]
	#[case(
		StyleDiagnosticKind::ClassAccessorKeyword { class_name: "type".into(), accessor: "type".into() },
		"class `.type` generates reserved Rust accessor `type`; rename the class to produce a non-keyword method name"
	)]
	#[case(
		StyleDiagnosticKind::ClassAccessorReserved { class_name: "vars".into(), accessor: "vars".into() },
		"class `.vars` generates style API accessor `vars`, which is reserved; rename the class to produce a different method name"
	)]
	#[case(
		StyleDiagnosticKind::InvalidGlobalName { name: "Accent".into() },
		"global name `Accent` is invalid; use a non-keyword ASCII snake_case identifier matching `[a-z][a-z0-9]*(?:_[a-z0-9]+)*`"
	)]
	#[case(
		StyleDiagnosticKind::InvalidVariableName { name: "large__gap".into() },
		"component variable name `large__gap` is invalid; use a non-keyword ASCII snake_case identifier matching `[a-z][a-z0-9]*(?:_[a-z0-9]+)*`"
	)]
	#[case(
		StyleDiagnosticKind::DuplicateGlobal { name: "accent".into() },
		"duplicate global `accent`; global names must be unique"
	)]
	#[case(
		StyleDiagnosticKind::DuplicateVariable { name: "gutter".into() },
		"duplicate component variable `gutter`; variable names must be unique"
	)]
	#[case(
		StyleDiagnosticKind::UndeclaredGlobalReference { name: "accent".into() },
		"undeclared global reference `globals.accent`; declare it in the `globals` block"
	)]
	#[case(
		StyleDiagnosticKind::UndeclaredVariableReference { name: "gutter".into() },
		"undeclared component variable reference `vars.gutter`; declare it in the `vars` block"
	)]
	#[case(
		StyleDiagnosticKind::MissingVariableDefault { name: "gutter".into() },
		"component variable `gutter` is missing a default; add `= <typed value>`"
	)]
	#[case(
		StyleDiagnosticKind::VariableDependencyCycle { names: vec!["a".into(), "b".into(), "a".into()] },
		"component-variable dependency cycle: a -> b -> a; break the cycle with an independent typed default"
	)]
	#[case(
		StyleDiagnosticKind::UnknownStyleType { name: "Distance".into() },
		"unknown style type `Distance`; use Color, Length, LengthPercentage, Percentage, Angle, Time, Number, or Integer"
	)]
	#[case(
		StyleDiagnosticKind::UnknownUnit { name: "furlong".into() },
		"unknown style unit `furlong`; use a unit from the checked style registry"
	)]
	#[case(
		StyleDiagnosticKind::PropertyValueMismatch { property: "width".into(), expected: "LengthPercentage or size keyword".into(), found: "Color".into() },
		"value for property `width` has type `Color`; expected LengthPercentage or size keyword"
	)]
	#[case(
		StyleDiagnosticKind::InvalidArithmeticDimensions { operation: "+".into(), left: "Length".into(), right: "Time".into() },
		"invalid dimensions for `+`: `Length` and `Time` cannot be combined"
	)]
	#[case(
		StyleDiagnosticKind::UnknownProperty { name: "colour".into() },
		"unknown property `colour`; use a registered canonical CSS property name"
	)]
	#[case(
		StyleDiagnosticKind::UnknownFunction { name: "paint".into() },
		"unknown style function `paint`; use a registered function or wrap one balanced call in `unchecked_fn!(...)`"
	)]
	#[case(
		StyleDiagnosticKind::DirectVarCall,
		"direct `var(...)` is unsupported; declare a typed `global` or component `var` and reference it"
	)]
	#[case(
		StyleDiagnosticKind::DirectCalcCall,
		"direct `calc(...)` is unsupported; write arithmetic with style operators"
	)]
	#[case(
		StyleDiagnosticKind::InvalidFunctionArgument { function: "rotate".into(), index: 1, expected: "Angle".into(), found: "Length".into() },
		"argument 1 to `rotate` has type `Length`; expected Angle"
	)]
	#[case(
		StyleDiagnosticKind::InvalidFunctionArity { function: "clamp".into(), expected: "exactly 3".into(), found: 2 },
		"function `clamp` received 2 arguments; expected exactly 3"
	)]
	#[case(
		StyleDiagnosticKind::InvalidUncheckedPlacement { context: "arithmetic".into() },
		"unchecked_fn! is only valid as an entire property value or typed variable default, not inside arithmetic"
	)]
	#[case(
		StyleDiagnosticKind::InvalidReceiverMethod { receiver: "Length".into(), method: "mix".into() },
		"method `mix` is not available on `Length`; use a method registered for that receiver type"
	)]
	#[case(
		StyleDiagnosticKind::DuplicatePackageStyleIdentity { identity: "app::CARD".into() },
		"duplicate package style identity `app::CARD`; each style definition must have a unique module path and static name"
	)]
	#[case(
		StyleDiagnosticKind::ShortenedScopeHashCollision { hash: "a1b2c3d4e5f6".into() },
		"shortened style scope hash collision `a1b2c3d4e5f6`; rename one style definition to produce a distinct identity"
	)]
	#[case(
		StyleDiagnosticKind::ReservedGeneratedAssetCollision { path: "static/.reinhardt/styles.css".into() },
		"physical file `static/.reinhardt/styles.css` collides with the reserved generated style asset; remove or rename the file"
	)]
	fn diagnostic_kind_has_stable_message(
		#[case] kind: StyleDiagnosticKind,
		#[case] expected: &str,
	) {
		// Arrange
		let diagnostic = StyleDiagnostic::new(kind, Span::call_site());

		// Act
		let actual = diagnostic.to_string();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn syn_error_combines_primary_and_related_labels() {
		// Arrange
		let diagnostic = StyleDiagnostic::new(
			StyleDiagnosticKind::ClassAccessorCollision {
				accessor: "foo_bar".into(),
			},
			Span::call_site(),
		)
		.with_related(Span::call_site(), "first generated by `.foo-bar`")
		.with_related(Span::call_site(), "also generated by `.foo_bar`");

		// Act
		let compile_error = diagnostic.into_syn_error().into_compile_error();

		// Assert
		assert_eq!(
			compile_error.to_string(),
			quote! {
				::core::compile_error! { "multiple classes generate accessor `foo_bar`; rename one class so every generated method is unique" }
				::core::compile_error! { "first generated by `.foo-bar`" }
				::core::compile_error! { "also generated by `.foo_bar`" }
			}
			.to_string()
		);
	}
}
