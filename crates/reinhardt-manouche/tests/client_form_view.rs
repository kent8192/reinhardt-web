//! Contract tests for the independent named ClientForm view syntax.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use reinhardt_manouche::parser::{is_client_form_view, parse_client_form_view};

#[test]
fn accepts_external_paths_raw_fields_and_ordered_expression_islands() {
	// Arrange
	let tokens = quote! {
		client_form: crate::auth::LoginClientForm,
		summary: { label: "Errors" },
		customize: {
			r#type: { label: String::from("Type"), widget: TextInput },
			password: { placeholder: { let join = |a, b| format!("{a},{b}"); join("a", "b") } },
		},
		mutation: get::<Request, Response>(&login),
	};
	// Act
	let ast = parse_client_form_view(tokens.clone()).unwrap();
	// Assert
	assert!(is_client_form_view(&tokens));
	assert_eq!(ast.field_names(), vec!["type", "password"]);
	assert_eq!(ast.clauses.len(), 3);
	assert_eq!(
		ast.companion.to_token_stream().to_string(),
		"crate :: auth :: LoginClientForm"
	);
	assert_eq!(
		ast.mutation().to_token_stream().to_string(),
		quote!(get::<Request, Response>(&login)).to_string()
	);
}

#[test]
fn mode_detection_requires_the_first_property() {
	assert!(is_client_form_view(
		&quote!(client_form: Login, mutation: &m)
	));
	assert!(!is_client_form_view(&quote!(name: Login, fields: {})));
	assert!(!is_client_form_view(
		&quote!(mutation: &m, client_form: Login)
	));
	assert!(!is_client_form_view(&quote!(client_form())));
}

#[test]
fn rejects_duplicate_entries_at_every_level() {
	let cases = [
		(
			quote!(client_form: Login, mutation: &m, mutation: &m),
			"duplicate `mutation` property",
		),
		(
			quote!(client_form: Login, mutation: &m, client_form: Login),
			"duplicate `client_form` property",
		),
		(
			quote!(client_form: Login, mutation: &m, customize: { name: {}, r#name: {} }),
			"duplicate `name` field customization",
		),
		(
			quote!(client_form: Login, mutation: &m, customize: { name: { label: "a", label: "b" } }),
			"duplicate `label` property",
		),
		(
			quote!(client_form: Login, mutation: &m, styling: { class: "a", class: "b" }),
			"duplicate `class` property",
		),
		(
			quote!(client_form: Login, mutation: &m, submit: { label: "a", label: "b" }),
			"duplicate `label` property",
		),
		(
			quote!(client_form: Login, mutation: &m, summary: { label: "a", label: "b" }),
			"duplicate `label` property",
		),
	];
	for (tokens, message) in cases {
		assert_eq!(
			parse_client_form_view(tokens).unwrap_err().to_string(),
			message
		);
	}
}

#[test]
fn rejects_standalone_configuration_and_unknown_properties() {
	for key in [
		"fields",
		"action",
		"method",
		"server_fn",
		"state",
		"validation",
		"on_submit",
		"on_success",
		"on_error",
		"name",
	] {
		let tokens: TokenStream = format!("client_form: Login, mutation: &m, {key}: {{}}")
			.parse()
			.unwrap();
		assert_eq!(
			parse_client_form_view(tokens).unwrap_err().to_string(),
			format!("`{key}` is not allowed in a named ClientForm view")
		);
	}
	for key in [
		"min",
		"max",
		"minlength",
		"maxlength",
		"pattern",
		"required",
		"unknown",
	] {
		let tokens: TokenStream =
			format!("client_form: Login, mutation: &m, customize: {{ name: {{ {key}: 1 }} }}")
				.parse()
				.unwrap();
		assert_eq!(
			parse_client_form_view(tokens).unwrap_err().to_string(),
			format!("unknown field presentation property `{key}`")
		);
	}
}

#[test]
fn validates_widget_tokens_ids_and_required_entries() {
	let cases = [
		(quote!(client_form: Login), "missing `mutation` property"),
		(
			quote!(mutation: &m, client_form: Login),
			"`client_form` must be the first property",
		),
		(
			quote!(client_form: Login, mutation: &m, id: ""),
			"`id` must be nonempty and contain no ASCII whitespace",
		),
		(
			quote!(client_form: Login, mutation: &m, id: "has space"),
			"`id` must be nonempty and contain no ASCII whitespace",
		),
		(
			quote!(client_form: Login, mutation: &m, customize: { name: { widget: widgets::TextInput } }),
			"expected a supported widget identifier",
		),
		(
			quote!(client_form: Login, mutation: &m, customize: { name: { widget: CustomInput } }),
			"expected a supported widget identifier",
		),
	];
	for (tokens, message) in cases {
		assert_eq!(
			parse_client_form_view(tokens).unwrap_err().to_string(),
			message
		);
	}
	assert!(
		parse_client_form_view(quote!(client_form: Login, mutation: &m, id: make_id())).is_ok()
	);
	assert!(
		parse_client_form_view(quote!(client_form: Login, mutation: &m, id: "valid:表" )).is_ok()
	);
	assert!(parse_client_form_view(quote!(client_form: Login, mutation: &m trailing)).is_err());
	assert!(
		parse_client_form_view(
			quote!(client_form: Login, mutation: &m, submit: { label: "Submit" trailing })
		)
		.is_err()
	);
}
