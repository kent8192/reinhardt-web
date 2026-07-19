//! Public structured server-function error API compile test.

use reinhardt_pages::{
	ServerFnError, ServerFnErrorKind, ServerFnErrorPayload, ServerFnFieldError,
};
use reinhardt_pages::prelude::ServerFnErrorPayload as PreludeServerFnErrorPayload;
use reinhardt_pages::server_fn::ServerFnErrorPayload as ServerFnModuleErrorPayload;

fn accepts_payload(_: ServerFnErrorPayload) {}

fn accepts_prelude_payload(_: PreludeServerFnErrorPayload) {}

fn accepts_server_fn_module_payload(_: ServerFnModuleErrorPayload) {}

fn accepts_field_error(_: ServerFnFieldError) {}

fn handles_non_exhaustive_kind(kind: ServerFnErrorKind) {
	match kind {
		ServerFnErrorKind::Validation => {}
		_ => {}
	}
}

fn main() {
	let error = ServerFnError::validation([("email", "Enter a valid email address")]);

	assert_eq!(error.kind(), ServerFnErrorKind::Validation);
	assert_eq!(error.status(), Some(422));
	assert_eq!(error.user_message(), "Validation failed");
	assert_eq!(error.field_errors()[0].field(), "email");
	assert_eq!(error.field_errors()[0].message(), "Enter a valid email address");

	let _ = accepts_payload;
	let _ = accepts_prelude_payload;
	let _ = accepts_server_fn_module_payload;
	let _ = accepts_field_error;
	let _ = handles_non_exhaustive_kind;
}
