use hyper::header;
use reinhardt_core::model_form::{
	ModelFormCleanedPayload, ModelFormPayload, ModelFormPolicy, ModelFormUpload,
	ModelFormValidatingPayload, NativeModelFormPayload,
};
use reinhardt_core::parsers::multipart::MultipartPart;
use reinhardt_core::parsers::{MediaType, MultiPartParser, UploadedFile};
use reinhardt_http::Request;
use serde::de::DeserializeOwned;
use std::collections::HashSet;

use super::{ServerFnArgumentKind, ServerFnError, ServerFnMetadata};

const INVALID_REQUEST_MESSAGE: &str = "Invalid server function request";
const MODEL_FORM_PROTOCOL_PREFIXES: &[&str] = &[
	"__reinhardt_checkbox_",
	"__reinhardt_color_",
	"__reinhardt_range_",
	"__reinhardt_defaulted_",
	"__reinhardt_file_",
	"__reinhardt_no_script_",
	"__reinhardt_native_edited_",
	"__reinhardt_json_encoded_",
];

struct MultipartArgumentPolicy<M>(std::marker::PhantomData<fn() -> M>);

impl<M: ServerFnMetadata> ModelFormPolicy for MultipartArgumentPolicy<M> {
	fn allows(field: &str) -> bool {
		M::ARGUMENTS.iter().any(|argument| argument.name == field)
	}
}

/// Ordered multipart arguments decoded for a generated native server function.
#[doc(hidden)]
pub struct MultipartArguments {
	parts: Vec<MultipartPart>,
}

impl MultipartArguments {
	/// Parses one multipart request using the core ordered-part parser.
	pub async fn from_request(request: &Request) -> Result<Self, ServerFnError> {
		let content_type = request
			.headers
			.get(header::CONTENT_TYPE)
			.and_then(|value| value.to_str().ok())
			.ok_or_else(|| invalid_request("missing_content_type", None))?;
		let media_type = MediaType::parse(content_type).map_err(|error| {
			tracing::warn!(error = %error, "Failed to parse multipart content type");
			invalid_request("malformed_content_type", None)
		})?;
		if !media_type.matches("multipart/form-data") {
			return Err(invalid_request("unexpected_content_type", None));
		}
		let boundary = media_type
			.parameters
			.get("boundary")
			.ok_or_else(|| invalid_request("missing_boundary", None))?;
		let boundary = normalize_boundary(boundary);
		let body = request.read_body().map_err(|error| {
			tracing::warn!(error = %error, "Failed to read multipart request body");
			invalid_request("unavailable_body", None)
		})?;
		let parts = MultiPartParser::new()
			.parse_parts(boundary, body)
			.await
			.map_err(|error| {
				tracing::warn!(error = %error, "Failed to parse multipart request body");
				invalid_request("malformed_multipart", None)
			})?;
		if let Some(name) = duplicate_name(&parts) {
			return Err(invalid_request("duplicate_argument", Some(name)));
		}

		Ok(Self { parts })
	}

	/// Revalidates model scalars and upload metadata before user-handler execution.
	///
	/// Argument allowlists, file kinds, and required uploads are checked before
	/// decoding model scalars, so forbidden fields retain structured validation errors.
	/// The concrete payload binds the generated rules and server-owned policy;
	/// trusted endpoint metadata narrows validation to its selected arguments.
	pub fn validate_model_form<D, P, M>(&mut self) -> Result<(), ServerFnError>
	where
		D: ModelFormPayload<P> + ModelFormValidatingPayload + NativeModelFormPayload,
		P: ModelFormPolicy,
		M: ServerFnMetadata,
	{
		let arguments = M::ARGUMENTS;
		for argument in arguments {
			if !P::allows(argument.name) {
				return Err(ServerFnError::validation([(
					argument.name,
					"This field is not allowed.",
				)]));
			}
			let part = self
				.parts
				.iter()
				.find(|part| part_name(part) == argument.name);
			match (argument.kind, part) {
				(ServerFnArgumentKind::File, Some(MultipartPart::File(file))) => {
					if is_empty_file_input(file) {
						return Err(invalid_request("empty_required_file", Some(argument.name)));
					}
				}
				(ServerFnArgumentKind::File, None) => {
					return Err(invalid_request("missing_argument", Some(argument.name)));
				}
				(
					ServerFnArgumentKind::File | ServerFnArgumentKind::OptionalFile,
					Some(part @ MultipartPart::Field { .. }),
				) => {
					return Err(kind_mismatch(argument.name, "file", part));
				}
				(ServerFnArgumentKind::Json, Some(part @ MultipartPart::File(_))) => {
					return Err(kind_mismatch(argument.name, "json", part));
				}
				_ => {}
			}
		}
		let mut uploads = Vec::new();
		for part in &self.parts {
			let name = part_name(part);
			if is_model_form_protocol_field(name) {
				continue;
			}
			if !P::allows(name) {
				return Err(ServerFnError::validation([(
					name,
					"This field is not allowed.",
				)]));
			}
			let Some(argument) = arguments.iter().find(|argument| argument.name == name) else {
				return Err(invalid_request("unexpected_argument", Some(name)));
			};
			if let MultipartPart::File(file) = part
				&& !is_empty_file_input(file)
			{
				uploads.push(ModelFormUpload {
					name: argument.name,
					filename: file.filename.clone(),
					content_type: file.content_type.clone(),
					size: file.data.len() as u64,
				});
			}
		}
		let native_value = self
			.parts
			.iter()
			.filter_map(|part| match part {
				MultipartPart::Field { name, data } => Some(
					decode_native_field_value(data)
						.map(|value| (name.clone(), value))
						.map_err(|_| invalid_request("malformed_native_field", Some(name))),
				),
				MultipartPart::File(_) => None,
			})
			.collect::<Result<serde_json::Map<_, _>, _>>()?;
		let payload = D::from_native_form_value(serde_json::Value::Object(native_value))
			.map_err(|_| invalid_request("malformed_native_model_form", None))?;
		self.parts
			.retain(|part| !is_model_form_protocol_field(part_name(part)));
		let deferred_files = arguments
			.iter()
			.filter(|argument| argument.kind == ServerFnArgumentKind::File)
			.map(|argument| argument.name)
			.collect::<Vec<_>>();
		let payload = payload
			.clean_and_validate_for_multipart::<MultipartArgumentPolicy<M>>(
				&deferred_files,
				&uploads,
			)?
			.into_raw();
		for argument in arguments {
			if argument.kind != ServerFnArgumentKind::Json {
				continue;
			}
			let value = payload.get_json(argument.name);
			let part = self
				.parts
				.iter_mut()
				.find(|part| part_name(part) == argument.name);
			match (part, value) {
				(Some(MultipartPart::Field { data, .. }), value) => {
					*data = serde_json::to_vec(&value.unwrap_or(serde_json::Value::Null))
						.map_err(|_| ServerFnError::server(500, "Internal server error"))?
						.into();
				}
				(None, Some(value)) => self.parts.push(MultipartPart::Field {
					name: argument.name.to_owned(),
					data: serde_json::to_vec(&value)
						.map_err(|_| ServerFnError::server(500, "Internal server error"))?
						.into(),
				}),
				_ => {}
			}
		}
		Ok(())
	}

	/// Removes and decodes one required JSON scalar part.
	pub fn take_json<T: DeserializeOwned>(
		&mut self,
		name: &'static str,
	) -> Result<T, ServerFnError> {
		match take_part(&mut self.parts, name) {
			Some(MultipartPart::Field { data, .. }) => {
				serde_json::from_slice(&data).map_err(|error| {
					tracing::warn!(argument = name, error = %error, "Failed to decode multipart JSON argument");
					invalid_request("malformed_json", Some(name))
				})
			}
			Some(part) => Err(kind_mismatch(name, "json", &part)),
			None => Err(invalid_request("missing_argument", Some(name))),
		}
	}

	/// Removes and decodes one optional JSON scalar part.
	pub fn take_optional_json<T: DeserializeOwned>(
		&mut self,
		name: &'static str,
	) -> Result<Option<T>, ServerFnError> {
		match take_part(&mut self.parts, name) {
			Some(MultipartPart::Field { data, .. }) => {
				serde_json::from_slice(&data).map_err(|error| {
					tracing::warn!(argument = name, error = %error, "Failed to decode optional multipart JSON argument");
					invalid_request("malformed_json", Some(name))
				})
			}
			Some(part) => Err(kind_mismatch(name, "optional_json", &part)),
			None => Ok(None),
		}
	}

	/// Removes one required file part.
	pub fn take_file(&mut self, name: &'static str) -> Result<UploadedFile, ServerFnError> {
		match take_part(&mut self.parts, name) {
			Some(MultipartPart::File(file)) if !is_empty_file_input(&file) => Ok(file),
			Some(MultipartPart::File(_)) => Err(invalid_request("empty_required_file", Some(name))),
			Some(part) => Err(kind_mismatch(name, "file", &part)),
			None => Err(invalid_request("missing_argument", Some(name))),
		}
	}

	/// Removes one optional file part, treating an empty browser file input as absent.
	pub fn take_optional_file(
		&mut self,
		name: &'static str,
	) -> Result<Option<UploadedFile>, ServerFnError> {
		match take_part(&mut self.parts, name) {
			Some(MultipartPart::File(file)) if is_empty_file_input(&file) => Ok(None),
			Some(MultipartPart::File(file)) => Ok(Some(file)),
			Some(part) => Err(kind_mismatch(name, "optional_file", &part)),
			None => Ok(None),
		}
	}

	/// Removes all parsed parts in their original wire order.
	pub fn take_parts(&mut self) -> Vec<MultipartPart> {
		std::mem::take(&mut self.parts)
	}

	/// Rejects any unconsumed multipart parts.
	pub fn finish(self) -> Result<(), ServerFnError> {
		match self.parts.first() {
			Some(part) => Err(invalid_request(
				"unexpected_argument",
				Some(part_name(part)),
			)),
			None => Ok(()),
		}
	}
}

fn duplicate_name(parts: &[MultipartPart]) -> Option<&str> {
	let mut seen = HashSet::with_capacity(parts.len());
	for part in parts {
		let name = part_name(part);
		if !seen.insert(name) {
			return Some(name);
		}
	}
	None
}

fn normalize_boundary(boundary: &str) -> &str {
	boundary
		.strip_prefix('"')
		.and_then(|boundary| boundary.strip_suffix('"'))
		.unwrap_or(boundary)
}

fn take_part(parts: &mut Vec<MultipartPart>, name: &str) -> Option<MultipartPart> {
	parts
		.iter()
		.position(|part| part_name(part) == name)
		.map(|index| parts.remove(index))
}

fn part_name(part: &MultipartPart) -> &str {
	match part {
		MultipartPart::Field { name, .. } => name,
		MultipartPart::File(file) => &file.name,
	}
}

fn is_model_form_protocol_field(name: &str) -> bool {
	name == "csrfmiddlewaretoken"
		|| MODEL_FORM_PROTOCOL_PREFIXES
			.iter()
			.any(|prefix| name.starts_with(prefix))
}

/// Retains multipart wire text so the model-form schema can decode JSON fields
/// without losing the distinction between a JSON string and a JSON scalar.
fn decode_native_field_value(data: &[u8]) -> Result<serde_json::Value, std::str::Utf8Error> {
	let text = std::str::from_utf8(data)?;
	Ok(serde_json::Value::String(text.to_owned()))
}

fn part_kind(part: &MultipartPart) -> &'static str {
	match part {
		MultipartPart::Field { .. } => "json",
		MultipartPart::File(_) => "file",
	}
}

fn is_empty_file_input(file: &UploadedFile) -> bool {
	file.size == 0 && file.filename.as_deref().is_none_or(str::is_empty)
}

fn kind_mismatch(
	name: &'static str,
	expected: &'static str,
	part: &MultipartPart,
) -> ServerFnError {
	tracing::warn!(
		argument = name,
		expected_kind = expected,
		actual_kind = part_kind(part),
		"Multipart server function argument kind mismatch",
	);
	invalid_request("kind_mismatch", Some(name))
}

fn invalid_request(reason: &'static str, argument: Option<&str>) -> ServerFnError {
	tracing::warn!(
		reason,
		argument = argument.unwrap_or_default(),
		"Rejected multipart server function request",
	);
	ServerFnError::server(400, INVALID_REQUEST_MESSAGE)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use rstest::rstest;

	#[rstest]
	#[tokio::test]
	async fn take_parts_preserves_multipart_wire_order_and_allows_finish() {
		let request = reinhardt_http::Request::builder()
			.uri("/api/server_fn/upload")
			.header(header::CONTENT_TYPE, "multipart/form-data; boundary=boundary")
			.body(
				b"--boundary\r\nContent-Disposition: form-data; name=\"first\"\r\n\r\n\"one\"\r\n--boundary\r\nContent-Disposition: form-data; name=\"second\"\r\n\r\n\"two\"\r\n--boundary--\r\n"
					.as_slice()
					.into(),
			)
			.build()
			.expect("multipart request should build");

		let mut arguments = MultipartArguments::from_request(&request)
			.await
			.expect("multipart request should parse");
		let parts = arguments.take_parts();
		arguments
			.finish()
			.expect("draining parts should leave no unexpected arguments");

		let names = parts.iter().map(part_name).collect::<Vec<_>>();
		assert_eq!(names, ["first", "second"]);
	}

	#[test]
	fn duplicate_argument_names_are_detected_in_one_pass() {
		let parts = vec![
			MultipartPart::Field {
				name: "name".to_owned(),
				data: Bytes::from_static(b"first"),
			},
			MultipartPart::File(UploadedFile::new(
				"avatar".to_owned(),
				Bytes::from_static(b"file"),
			)),
			MultipartPart::Field {
				name: "name".to_owned(),
				data: Bytes::from_static(b"second"),
			},
		];

		assert_eq!(duplicate_name(&parts), Some("name"));
	}

	#[test]
	fn quoted_multipart_boundaries_are_unquoted_before_parsing() {
		assert_eq!(normalize_boundary("\"abc123\""), "abc123");
		assert_eq!(normalize_boundary("abc123"), "abc123");
	}

	#[test]
	fn optional_json_accepts_omitted_null_and_present_values() {
		let mut arguments = MultipartArguments { parts: Vec::new() };
		assert_eq!(
			arguments.take_optional_json::<String>("note").unwrap(),
			None
		);

		arguments.parts.push(MultipartPart::Field {
			name: "note".to_owned(),
			data: Bytes::from_static(b"null"),
		});
		assert_eq!(
			arguments.take_optional_json::<String>("note").unwrap(),
			None
		);

		arguments.parts.push(MultipartPart::Field {
			name: "note".to_owned(),
			data: Bytes::from_static(b"\"saved\""),
		});
		assert_eq!(
			arguments.take_optional_json::<String>("note").unwrap(),
			Some("saved".to_owned())
		);
	}

	#[rstest]
	#[case::browser_text(b"Report", "Report")]
	#[case::json_encoded_text(br#""Report""#, r#""Report""#)]
	fn native_multipart_fields_preserve_wire_text(#[case] input: &[u8], #[case] expected: &str) {
		let value = decode_native_field_value(input).expect("native field should be valid UTF-8");

		assert_eq!(value, serde_json::Value::String(expected.to_owned()));
	}

	#[rstest]
	#[case("csrfmiddlewaretoken")]
	#[case("__reinhardt_color_accent")]
	#[case("__reinhardt_native_edited_accent")]
	#[case("__reinhardt_json_encoded_title")]
	fn model_form_protocol_fields_are_reserved(#[case] name: &str) {
		assert!(is_model_form_protocol_field(name));
	}

	#[rstest]
	#[case("__reinhardt_note")]
	#[case("__reinhardt_custom_field")]
	fn model_form_fields_outside_reserved_prefixes_are_not_protocol_fields(#[case] name: &str) {
		assert!(!is_model_form_protocol_field(name));
	}
}
