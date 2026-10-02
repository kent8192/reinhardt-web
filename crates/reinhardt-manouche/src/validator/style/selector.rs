//! Selector and generated-accessor validation.

use std::collections::HashMap;

use convert_case::{Case, Casing};
use syn::Ident;

use crate::{
	StyleAttributeMatcher, StyleAttributeValue, StyleDiagnostic, StyleDiagnosticKind, StyleItem,
	StyleMacro, StyleRule, StyleRuleItem, StyleSelectorKind, StyleSimpleSelector, TypedStyleClass,
};

pub(super) fn validate_selectors(
	ast: &StyleMacro,
) -> Result<Vec<TypedStyleClass>, StyleDiagnostic> {
	let mut classes = Vec::new();
	let mut by_name = HashMap::new();
	let mut by_accessor = HashMap::new();
	for item in &ast.items {
		match item {
			StyleItem::Rule(rule) => {
				validate_rule(rule, true, &mut classes, &mut by_name, &mut by_accessor)?;
			}
			StyleItem::Media(media) => {
				validate_items(
					&media.items,
					true,
					&mut classes,
					&mut by_name,
					&mut by_accessor,
				)?;
			}
		}
	}
	Ok(classes)
}

fn validate_items(
	items: &[StyleRuleItem],
	top_level: bool,
	classes: &mut Vec<TypedStyleClass>,
	by_name: &mut HashMap<String, usize>,
	by_accessor: &mut HashMap<String, usize>,
) -> Result<(), StyleDiagnostic> {
	for item in items {
		match item {
			StyleRuleItem::Declaration(declaration) => {
				if top_level {
					return Err(StyleDiagnostic::new(
						StyleDiagnosticKind::UnanchoredTopLevelDeclaration {
							property: declaration.name.as_str().to_owned(),
						},
						declaration.span,
					));
				}
			}
			StyleRuleItem::Rule(rule) => {
				validate_rule(rule, top_level, classes, by_name, by_accessor)?;
			}
			StyleRuleItem::Media(media) => {
				validate_items(&media.items, top_level, classes, by_name, by_accessor)?;
			}
		}
	}
	Ok(())
}

fn validate_rule(
	rule: &StyleRule,
	top_level: bool,
	classes: &mut Vec<TypedStyleClass>,
	by_name: &mut HashMap<String, usize>,
	by_accessor: &mut HashMap<String, usize>,
) -> Result<(), StyleDiagnostic> {
	for selector in &rule.selectors.selectors {
		let simple = match &selector.kind {
			StyleSelectorKind::Root(simple) | StyleSelectorKind::SameElement(simple) => simple,
			StyleSelectorKind::Relative { selector, .. } => selector,
		};
		if top_level
			&& !matches!(
				selector.kind,
				StyleSelectorKind::Root(StyleSimpleSelector::Class(_))
			) {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::UnanchoredTopLevelSelector {
					selector: selector_text(&selector.kind),
				},
				selector.span,
			));
		}
		validate_simple_selector(simple, classes, by_name, by_accessor)?;
	}
	validate_items(&rule.items, false, classes, by_name, by_accessor)
}

fn validate_simple_selector(
	selector: &StyleSimpleSelector,
	classes: &mut Vec<TypedStyleClass>,
	by_name: &mut HashMap<String, usize>,
	by_accessor: &mut HashMap<String, usize>,
) -> Result<(), StyleDiagnostic> {
	match selector {
		StyleSimpleSelector::Class(name) => {
			collect_class(name.as_str(), name.span, classes, by_name, by_accessor)?;
		}
		StyleSimpleSelector::Pseudo(pseudo) => {
			if pseudo.name.as_str().eq_ignore_ascii_case("global") {
				return Err(StyleDiagnostic::new(
					StyleDiagnosticKind::UnanchoredTopLevelSelector {
						selector: ":global".into(),
					},
					pseudo.span,
				));
			}
			if let Some(selector_list) = pseudo
				.arguments
				.as_ref()
				.and_then(|arguments| arguments.selector_list.as_ref())
			{
				for branch in &selector_list.selectors {
					let nested = match &branch.kind {
						StyleSelectorKind::Root(selector)
						| StyleSelectorKind::SameElement(selector) => selector,
						StyleSelectorKind::Relative { selector, .. } => selector,
					};
					validate_simple_selector(nested, classes, by_name, by_accessor)?;
				}
			}
		}
		StyleSimpleSelector::Type(_)
		| StyleSimpleSelector::Id(_)
		| StyleSimpleSelector::Universal { .. }
		| StyleSimpleSelector::Attribute(_) => {}
	}
	Ok(())
}

fn collect_class(
	name: &str,
	span: proc_macro2::Span,
	classes: &mut Vec<TypedStyleClass>,
	by_name: &mut HashMap<String, usize>,
	by_accessor: &mut HashMap<String, usize>,
) -> Result<(), StyleDiagnostic> {
	if !is_ascii_css_identifier(name) {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::InvalidClassName {
				name: name.to_owned(),
			},
			span,
		));
	}
	if by_name.contains_key(name) {
		return Ok(());
	}
	let accessor = name.to_case(Case::Snake);
	if is_rust_keyword(&accessor) {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::ClassAccessorKeyword {
				class_name: name.to_owned(),
				accessor,
			},
			span,
		));
	}
	if accessor == "vars" {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::ClassAccessorReserved {
				class_name: name.to_owned(),
				accessor,
			},
			span,
		));
	}
	if syn::parse_str::<Ident>(&accessor).is_err() {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::InvalidClassName {
				name: name.to_owned(),
			},
			span,
		));
	}
	if let Some(&first_index) = by_accessor.get(&accessor) {
		let first = &classes[first_index];
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::ClassAccessorCollision {
				accessor: accessor.clone(),
			},
			span,
		)
		.with_related(
			first.span,
			format!("first generated by class `.{}`", first.authored_name),
		)
		.with_related(span, format!("also generated by class `.{name}`")));
	}
	let index = classes.len();
	classes.push(TypedStyleClass {
		authored_name: name.to_owned(),
		accessor: accessor.clone(),
		span,
	});
	by_name.insert(name.to_owned(), index);
	by_accessor.insert(accessor, index);
	Ok(())
}

fn is_ascii_css_identifier(name: &str) -> bool {
	let bytes = name.as_bytes();
	let Some(&first) = bytes.first() else {
		return false;
	};
	if !first.is_ascii_alphabetic() && first != b'_' && first != b'-' {
		return false;
	}
	if first == b'-' && (bytes.len() == 1 || bytes.get(1).is_some_and(u8::is_ascii_digit)) {
		return false;
	}
	bytes
		.iter()
		.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub(super) fn is_rust_keyword(name: &str) -> bool {
	matches!(
		name,
		"as" | "async"
			| "await" | "break"
			| "const" | "continue"
			| "crate" | "dyn"
			| "else" | "enum"
			| "extern"
			| "false" | "fn"
			| "for" | "gen"
			| "if" | "impl"
			| "in" | "let"
			| "loop" | "match"
			| "mod" | "move"
			| "mut" | "pub"
			| "ref" | "return"
			| "self" | "Self"
			| "static"
			| "struct"
			| "super" | "trait"
			| "true" | "type"
			| "unsafe"
			| "use" | "where"
			| "while" | "abstract"
			| "become"
			| "box" | "do"
			| "final" | "macro"
			| "override"
			| "priv" | "try"
			| "typeof"
			| "unsized"
			| "virtual"
			| "yield"
	)
}

fn selector_text(kind: &StyleSelectorKind) -> String {
	let (prefix, simple) = match kind {
		StyleSelectorKind::Root(simple) => ("", simple),
		StyleSelectorKind::SameElement(simple) => ("&", simple),
		StyleSelectorKind::Relative { selector, .. } => ("", selector),
	};
	format!("{prefix}{}", simple_selector_text(simple))
}

fn simple_selector_text(selector: &StyleSimpleSelector) -> String {
	match selector {
		StyleSimpleSelector::Class(name) => format!(".{}", name.as_str()),
		StyleSimpleSelector::Type(name) => name.as_str().to_owned(),
		StyleSimpleSelector::Id(name) => format!("#{}", name.as_str()),
		StyleSimpleSelector::Universal { .. } => "*".into(),
		StyleSimpleSelector::Attribute(attribute) => {
			let mut text = format!("[{}", attribute.name.as_str());
			if let (Some(matcher), Some(value)) = (attribute.matcher, &attribute.value) {
				text.push_str(match matcher {
					StyleAttributeMatcher::Equals => "=",
					StyleAttributeMatcher::Includes => "~=",
					StyleAttributeMatcher::DashMatch => "|=",
					StyleAttributeMatcher::Prefix => "^=",
					StyleAttributeMatcher::Suffix => "$=",
					StyleAttributeMatcher::Substring => "*=",
				});
				match value {
					StyleAttributeValue::Identifier(name) => text.push_str(name.as_str()),
					StyleAttributeValue::String { value, .. } => {
						text.push('"');
						text.push_str(value);
						text.push('"');
					}
				}
			}
			text.push(']');
			text
		}
		StyleSimpleSelector::Pseudo(pseudo) => format!(":{}", pseudo.name.as_str()),
	}
}

#[cfg(test)]
mod tests {
	use quote::quote;
	use rstest::rstest;

	use crate::{
		StyleDiagnosticKind, StyleItem, StyleSelectorKind, StyleSimpleSelector,
		parser::parse_style, validator::validate_style,
	};

	#[rstest]
	#[case(quote! { button {} }, "button")]
	#[case(quote! { :root {} }, ":root")]
	#[case(quote! { html {} }, "html")]
	#[case(quote! { body {} }, "body")]
	#[case(quote! { * {} }, "*")]
	#[case(quote! { [data-theme] {} }, "[data-theme]")]
	fn rejects_unanchored_top_level_roots(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected_selector: &str,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::UnanchoredTopLevelSelector {
				selector: expected_selector.into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn rejects_a_declaration_directly_inside_top_level_media() {
		// Arrange
		let ast = parse_style(quote! {
			@media (width > 1px) { color: red; }
		})
		.unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::UnanchoredTopLevelDeclaration {
				property: "color".into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn accepts_a_declaration_inside_media_nested_under_a_local_rule() {
		// Arrange
		let ast = parse_style(quote! {
			.card { @media (width > 1px) { color: red; } }
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(typed.classes.len(), 1);
		assert_eq!(typed.classes[0].authored_name, "card");
	}

	#[rstest]
	fn rejects_colliding_generated_class_accessors() {
		// Arrange
		let ast = parse_style(quote! { .foo-bar {} .foo_bar {} }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorCollision {
				accessor: "foo_bar".into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 2);
		assert_eq!(
			diagnostic.related[0].reason,
			"first generated by class `.foo-bar`"
		);
		assert_eq!(
			diagnostic.related[1].reason,
			"also generated by class `.foo_bar`"
		);
	}

	#[rstest]
	fn repeated_class_rules_share_first_occurrence_metadata() {
		// Arrange
		let ast = parse_style(quote! { .card {} .label {} .card {} }).unwrap();
		let StyleItem::Rule(first_rule) = &ast.items[0] else {
			panic!("expected the first style rule");
		};
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(first_class)) =
			&first_rule.selectors.selectors[0].kind
		else {
			panic!("expected the first local class");
		};

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(typed.classes.len(), 2);
		assert_eq!(typed.classes[0].authored_name, "card");
		assert_eq!(typed.classes[0].accessor, "card");
		assert_eq!(
			format!("{:?}", typed.classes[0].span),
			format!("{:?}", first_class.span)
		);
		assert_eq!(typed.classes[1].authored_name, "label");
	}

	#[rstest]
	#[case(quote! { .123 {} }, "123")]
	#[case(quote! { .café {} }, "café")]
	fn rejects_class_names_that_cannot_generate_ascii_accessors(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected_name: &str,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::InvalidClassName {
				name: expected_name.into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	#[case(quote! { .type {} }, "type")]
	#[case(quote! { .match {} }, "match")]
	#[case(quote! { .self {} }, "self")]
	#[case(quote! { .crate {} }, "crate")]
	#[case(quote! { .super {} }, "super")]
	#[case(quote! { .gen {} }, "gen")]
	#[case(quote! { .r#type {} }, "type")]
	fn rejects_class_accessors_that_are_rust_keywords(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected_keyword: &str,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorKeyword {
				class_name: expected_keyword.into(),
				accessor: expected_keyword.into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn rejects_class_accessors_reserved_by_the_generated_style_api() {
		// Arrange
		let ast = parse_style(quote! { .vars {} }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorReserved {
				class_name: "vars".into(),
				accessor: "vars".into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn nested_selectors_inherit_anchor_and_collect_only_local_classes() {
		// Arrange
		let hash = proc_macro2::Punct::new('#', proc_macro2::Spacing::Alone);
		let ast = parse_style(quote! {
			.card {
				&:hover {}
				&[data-state=open] {}
				&.featured {}
				> .label {}
				+ button {}
				#hash child {}
				* {}
				.icon {}
				@media (width > 1px) {
					~ .sibling {}
				}
			}
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.classes
				.iter()
				.map(|class| (class.authored_name.as_str(), class.accessor.as_str()))
				.collect::<Vec<_>>(),
			vec![
				("card", "card"),
				("featured", "featured"),
				("label", "label"),
				("icon", "icon"),
				("sibling", "sibling"),
			]
		);
	}

	#[rstest]
	#[case(quote! { :global(.external) {} })]
	#[case(quote! { .card { &:global(.external) {} } })]
	#[case(quote! { .card { :global(.external) {} } })]
	fn rejects_global_selector_escape(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::UnanchoredTopLevelSelector {
				selector: ":global".into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn selector_list_pseudo_class_is_collected_as_local_metadata() {
		// Arrange
		let ast = parse_style(quote! { .card { &:is(.foo) {} } }).unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.classes
				.iter()
				.map(|class| class.authored_name.as_str())
				.collect::<Vec<_>>(),
			vec!["card", "foo"]
		);
	}

	#[rstest]
	#[case(quote! { .card { &:is(:global(.external)) {} } })]
	#[case(quote! { .card { &:IS(:GLOBAL(.external)) {} } })]
	fn nested_global_escape_in_selector_list_pseudo_is_rejected(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::UnanchoredTopLevelSelector {
				selector: ":global".into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn rust_keyword_class_in_selector_list_pseudo_is_rejected() {
		// Arrange
		let ast = parse_style(quote! { .card { &:is(.type) {} } }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorKeyword {
				class_name: "type".into(),
				accessor: "type".into(),
			}
		);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn accessor_collision_inside_selector_list_pseudo_reports_both_classes() {
		// Arrange
		let ast = parse_style(quote! { .card { &:is(.foo-bar, .foo_bar) {} } }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorCollision {
				accessor: "foo_bar".into(),
			}
		);
		assert_eq!(
			diagnostic
				.related
				.iter()
				.map(|label| label.reason.as_str())
				.collect::<Vec<_>>(),
			vec![
				"first generated by class `.foo-bar`",
				"also generated by class `.foo_bar`",
			]
		);
	}

	#[rstest]
	fn nested_selector_list_pseudos_are_walked_recursively() {
		// Arrange
		let ast = parse_style(quote! { .card { &:is(:not(:where(.deep))) {} } }).unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.classes
				.iter()
				.map(|class| class.authored_name.as_str())
				.collect::<Vec<_>>(),
			vec!["card", "deep"]
		);
	}

	#[rstest]
	fn every_supported_selector_list_pseudo_preserves_class_first_occurrence_order() {
		// Arrange
		let ast = parse_style(quote! {
			.card {
				&:not(.excluded) {}
				&:where(.context) {}
				&:has(.descendant) {}
			}
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.classes
				.iter()
				.map(|class| class.authored_name.as_str())
				.collect::<Vec<_>>(),
			vec!["card", "excluded", "context", "descendant"]
		);
	}

	#[rstest]
	#[case(quote! { .foo-bar { &:is(.foo_bar) {} } })]
	#[case(quote! { .card { &:is(.foo-bar) {} &:where(.foo_bar) {} } })]
	fn selector_function_classes_collide_with_outer_and_other_pseudo_classes(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorCollision {
				accessor: "foo_bar".into(),
			}
		);
		assert_eq!(
			diagnostic
				.related
				.iter()
				.map(|label| label.reason.as_str())
				.collect::<Vec<_>>(),
			vec![
				"first generated by class `.foo-bar`",
				"also generated by class `.foo_bar`",
			]
		);
	}

	#[rstest]
	fn non_class_selector_function_arguments_remain_unscoped() {
		// Arrange
		let input = ".card { &:is(button, [data-state], #item, *, :hover, .local) {} }"
			.parse()
			.unwrap();
		let ast = parse_style(input).unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.classes
				.iter()
				.map(|class| class.authored_name.as_str())
				.collect::<Vec<_>>(),
			vec!["card", "local"]
		);
	}

	#[rstest]
	#[case(quote! { .card { &:nth-child(2n of .foo) {} } }, "foo")]
	#[case(
		quote! { .card { &:NTH-LAST-CHILD(odd OF .last) {} } },
		"last"
	)]
	fn nth_selector_arguments_collect_local_classes(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected_class: &str,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.classes
				.iter()
				.map(|class| class.authored_name.as_str())
				.collect::<Vec<_>>(),
			vec!["card", expected_class]
		);
	}

	#[rstest]
	fn nth_selector_arguments_reject_nested_global_escape() {
		// Arrange
		let ast =
			parse_style(quote! { .card { &:nth-child(2n of :global(.external)) {} } }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::UnanchoredTopLevelSelector {
				selector: ":global".into(),
			}
		);
	}

	#[rstest]
	fn nth_selector_arguments_reject_rust_keyword_class() {
		// Arrange
		let ast = parse_style(quote! { .card { &:nth-last-child(odd of .type) {} } }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorKeyword {
				class_name: "type".into(),
				accessor: "type".into(),
			}
		);
	}

	#[rstest]
	fn nth_selector_arguments_report_accessor_collisions() {
		// Arrange
		let ast =
			parse_style(quote! { .card { &:nth-child(2n of .foo-bar, .foo_bar) {} } }).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::ClassAccessorCollision {
				accessor: "foo_bar".into(),
			}
		);
		assert_eq!(
			diagnostic
				.related
				.iter()
				.map(|label| label.reason.as_str())
				.collect::<Vec<_>>(),
			vec![
				"first generated by class `.foo-bar`",
				"also generated by class `.foo_bar`",
			]
		);
	}
}
