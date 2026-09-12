//! Schema-driven widget presentation and incomplete editor regression coverage.

use super::*;
use reinhardt_core::model_form::{
	AllEditableModelFields, ModelFormCleanedPayload, ModelFormContract, ModelFormContractField,
	ModelFormContractSchema, ModelFormFieldDescriptor, ModelFormFieldKind as Kind,
	ModelFormPayload, ModelFormPayloadError, ModelFormValidatingPayload, NativeModelFormPayload,
};
use reinhardt_pages::component::{ControlValue, ControlWriteOutcome};
use reinhardt_pages::control_binding::__private::{
	NumberBinding, TextBinding, into_control_binding,
};
use std::collections::HashMap;

struct WidgetContract;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WidgetField(&'static str);
impl ModelFormContractField for WidgetField {
	fn name(self) -> &'static str {
		self.0
	}
}
const TEXT: Kind = Kind::Text {
	min_length: Some(1),
	max_length: Some(120),
	multiline: false,
};
const fn field(name: &'static str, kind: Kind) -> ModelFormFieldDescriptor {
	ModelFormFieldDescriptor {
		name,
		kind,
		required: false,
		has_default: false,
		nullable: matches!(kind, Kind::Boolean),
		editable: true,
		generated_relation_id: false,
		trim: false,
	}
}
const FIELDS: &[ModelFormFieldDescriptor] = &[
	field("text", TEXT),
	field("notes", TEXT),
	field(
		"email",
		Kind::Email {
			min_length: None,
			max_length: Some(150),
		},
	),
	field(
		"url",
		Kind::Url {
			min_length: None,
			max_length: Some(250),
		},
	),
	field("password", TEXT),
	field("hidden", TEXT),
	field("color", TEXT),
	field(
		"range",
		Kind::Integer {
			min: Some(0),
			max: Some(10),
		},
	),
	field(
		"integer",
		Kind::Integer {
			min: None,
			max: None,
		},
	),
	field(
		"decimal",
		Kind::Decimal {
			min: None,
			max: None,
		},
	),
	field("date", Kind::Date),
	field("time", Kind::Time),
	field("timestamp", Kind::DateTime),
	field("uuid", Kind::Uuid),
	field("json", Kind::Json),
	field("boolean", Kind::Boolean),
];
impl WidgetContract {
	const fn text() -> &'static ModelFormFieldDescriptor {
		&FIELDS[0]
	}
	const fn notes() -> &'static ModelFormFieldDescriptor {
		&FIELDS[1]
	}
	const fn password() -> &'static ModelFormFieldDescriptor {
		&FIELDS[4]
	}
	const fn hidden() -> &'static ModelFormFieldDescriptor {
		&FIELDS[5]
	}
	const fn color() -> &'static ModelFormFieldDescriptor {
		&FIELDS[6]
	}
	const fn range() -> &'static ModelFormFieldDescriptor {
		&FIELDS[7]
	}
}
impl ModelFormContractSchema for WidgetContract {
	fn contract_fields() -> &'static [ModelFormFieldDescriptor] {
		FIELDS
	}
}
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct WidgetData(HashMap<String, serde_json::Value>);
impl ModelFormPayload<AllEditableModelFields> for WidgetData {
	fn supplied_fields(&self) -> Vec<&'static str> {
		FIELDS
			.iter()
			.filter(|field| self.0.contains_key(field.name))
			.map(|field| field.name)
			.collect()
	}
	fn forbidden_fields(&self) -> &[&'static str] {
		&[]
	}
	fn get_json(&self, field: &str) -> Option<serde_json::Value> {
		self.0.get(field).cloned()
	}
	fn set_json(
		&mut self,
		field: &str,
		value: serde_json::Value,
	) -> Result<(), ModelFormPayloadError> {
		self.0.insert(field.to_owned(), value);
		Ok(())
	}
}
struct CleanedWidgetData(WidgetData);
impl ModelFormCleanedPayload for CleanedWidgetData {
	type Raw = WidgetData;
	fn into_raw(self) -> Self::Raw {
		self.0
	}
}
impl ModelFormValidatingPayload for WidgetData {
	type Cleaned = CleanedWidgetData;
	fn clean_and_validate(
		self,
	) -> Result<Self::Cleaned, reinhardt_core::validators::ValidationErrors> {
		// Conversion and field constraints belong to the schema in this widget fixture.
		Ok(CleanedWidgetData(self))
	}
}
impl NativeModelFormPayload for WidgetData {
	fn from_native_form_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
		serde_json::from_value(value)
	}
}
impl ModelFormContract for WidgetContract {
	type Data = WidgetData;
	type Schema = Self;
	type Field = WidgetField;
	type Policy = AllEditableModelFields;
	fn fields() -> &'static [WidgetField] {
		&[
			WidgetField("text"),
			WidgetField("notes"),
			WidgetField("email"),
			WidgetField("url"),
			WidgetField("password"),
			WidgetField("hidden"),
			WidgetField("color"),
			WidgetField("range"),
			WidgetField("integer"),
			WidgetField("decimal"),
			WidgetField("date"),
			WidgetField("time"),
			WidgetField("timestamp"),
			WidgetField("uuid"),
			WidgetField("json"),
			WidgetField("boolean"),
		]
	}
}
#[server_fn(model_form = true)]
async fn save_page_widgets(payload: WidgetData) -> Result<WidgetData, ServerFnError> {
	Ok(payload)
}

fn control(root: &web_sys::Element, name: &str) -> web_sys::Element {
	root.query_selector(&format!("[name='{name}']"))
		.unwrap()
		.unwrap()
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_preserves_generated_widget_overrides() {
	// Arrange
	let root = BodyRoot::new("page-widgets");
	let scope = ReactiveScope::new();
	let (form, runtime, action) = scope.enter(|| {
		let form = form! {
			name: WidgetPageForm,
			model_form: WidgetContract,
			server_fn: save_page_widgets,
			overrides: {
				text: {
					label: "Display name",
					help_text: "Up to 120 characters"
				},
				notes: { widget: Textarea },
				password: { widget: PasswordInput },
				hidden: { widget: HiddenInput },
				color: { widget: ColorInput },
				range: { widget: RangeInput },
			},
		};
		for (name, value) in [
			("password", "secret-not-for-ssr"),
			("text", "initial"),
			("notes", "notes"),
			("email", "owner@example.com"),
			("url", "https://example.com"),
			("hidden", "hidden value"),
			("color", "#123456"),
			("timestamp", "2026-09-10T12:34:56Z"),
		] {
			form.set_value(name, serde_json::json!(value)).unwrap();
		}
		form.set_value("range", serde_json::json!(3)).unwrap();
		form.set_value("json", serde_json::json!({"enabled":true}))
			.unwrap();
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		let page = action.page();
		assert_eq!(
			page.render_to_string()
				.matches("secret-not-for-ssr")
				.count(),
			0
		);
		page.mount(&Element::new(root.element.clone())).unwrap();
		(form, runtime, action)
	});

	// Assert: actual browser controls preserve metadata and configured values.
	for (name, tag, input_type) in [
		("text", "INPUT", Some("text")),
		("notes", "TEXTAREA", None),
		("email", "INPUT", Some("email")),
		("url", "INPUT", Some("url")),
		("password", "INPUT", Some("password")),
		("hidden", "INPUT", Some("hidden")),
		("color", "INPUT", Some("color")),
		("range", "INPUT", Some("range")),
		("decimal", "INPUT", Some("number")),
		("date", "INPUT", Some("date")),
		("time", "INPUT", Some("time")),
		("timestamp", "INPUT", Some("datetime-local")),
		("uuid", "INPUT", Some("text")),
		("json", "TEXTAREA", None),
		("boolean", "SELECT", None),
	] {
		let control = control(&root.element, name);
		assert_eq!(control.tag_name(), tag);
		assert_eq!(control.get_attribute("type").as_deref(), input_type);
		assert_eq!(
			root.element
				.query_selector_all(&format!("label[for='{}']", control.id()))
				.unwrap()
				.length(),
			1
		);
	}
	let text = control(&root.element, "text")
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap();
	assert_eq!(text.get_attribute("maxlength").as_deref(), Some("120"));
	assert_eq!(text.get_attribute("minlength").as_deref(), Some("1"));
	assert_eq!(
		control(&root.element, "range")
			.get_attribute("min")
			.as_deref(),
		Some("0")
	);
	assert_eq!(
		control(&root.element, "range")
			.get_attribute("max")
			.as_deref(),
		Some("10")
	);
	assert_eq!(
		control(&root.element, "decimal")
			.get_attribute("step")
			.as_deref(),
		Some("any")
	);
	assert_eq!(
		control(&root.element, "timestamp")
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap()
			.value(),
		"2026-09-10T12:34:56"
	);
	assert_eq!(
		control(&root.element, "password")
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap()
			.value(),
		"secret-not-for-ssr"
	);
	assert_eq!(
		control(&root.element, "json")
			.dyn_into::<web_sys::HtmlTextAreaElement>()
			.unwrap()
			.value(),
		r#"{"enabled":true}"#
	);

	// Act: nullable choices and alternate text editors update the attached source.
	let choice = control(&root.element, "boolean")
		.dyn_into::<web_sys::HtmlSelectElement>()
		.unwrap();
	choice.set_value("false");
	choice
		.dispatch_event(&web_sys::Event::new("change").unwrap())
		.unwrap();
	let notes = control(&root.element, "notes")
		.dyn_into::<web_sys::HtmlTextAreaElement>()
		.unwrap();
	notes.set_value("edited notes");
	notes
		.dispatch_event(&web_sys::Event::new("input").unwrap())
		.unwrap();
	assert_eq!(
		form.data().unwrap().get_json("boolean"),
		Some(serde_json::json!(false))
	);
	assert_eq!(form.value("notes"), Some(serde_json::json!("edited notes")));
	runtime.reset();
	settle_browser().await;
	assert_eq!(choice.value(), "");
	assert_eq!(notes.value(), "notes");
	assert!(action.result().is_none());
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_keeps_invalid_scalar_editor_text() {
	// Arrange
	let root = BodyRoot::new("page-scalar-editors");
	let fetch = FetchGuard::for_page_cluster();
	let scope = ReactiveScope::new();
	let (form, runtime, action) = scope.enter(|| {
		let form = form! {
			name: ScalarWidgetPageForm,
			model_form: WidgetContract,
			server_fn: save_page_widgets
		};
		form.set_value("integer", serde_json::json!(7)).unwrap();
		form.set_value("date", serde_json::json!("2026-09-10"))
			.unwrap();
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		action
			.page()
			.mount(&Element::new(root.element.clone()))
			.unwrap();
		(form, runtime, action)
	});
	let json = control(&root.element, "json")
		.dyn_into::<web_sys::HtmlTextAreaElement>()
		.unwrap();
	let decimal = control(&root.element, "decimal")
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap();
	edit_input(&decimal, "12.30");
	assert_eq!(
		form.data().unwrap().get_json("decimal"),
		Some(serde_json::json!("12.30"))
	);
	json.set_value("{incomplete");
	json.dispatch_event(&web_sys::Event::new("input").unwrap())
		.unwrap();
	let number =
		into_control_binding::<NumberBinding, _>(runtime.field(WidgetField("integer")), ());
	assert!(matches!(
		number.write(ControlValue::Text("-".into())),
		Ok(ControlWriteOutcome::Rejected(_))
	));
	let date = into_control_binding::<TextBinding, _>(runtime.field(WidgetField("date")), ());
	assert_eq!(
		date.write(ControlValue::Text("2026-0".into())),
		Ok(ControlWriteOutcome::Committed)
	);

	// Act: unrelated error rendering must preserve incomplete editor text and nodes.
	runtime.set_error(
		WidgetField("text"),
		reinhardt_pages::FieldError::new("unrelated error"),
	);
	settle_browser().await;
	assert_eq!(json.value(), "{incomplete");
	assert_eq!(date.read(), ControlValue::Text("2026-0".into()));
	assert_eq!(form.value("integer"), Some(serde_json::json!(7)));
	assert!(json.is_same_node(Some(control(&root.element, "json").as_ref())));
	assert_eq!(action.dispatch(), MutationDispatchOutcome::ValidationFailed);
	assert_eq!(fetch.create_requests(), 0);
	runtime.reset();
	settle_browser().await;

	// Assert
	assert_eq!(json.value(), "");
	assert_eq!(decimal.value(), "");
	assert_eq!(date.read(), ControlValue::Text("2026-09-10".into()));
	assert!(!(runtime.form_state().is_dirty.get()));
	assert!(runtime.form_state().field_errors.get().is_empty());
}

#[wasm_bindgen_test(async)]
#[serial(server_mutation_globals)]
async fn page_keeps_optional_native_defaults_unsupplied() {
	// Arrange
	let root = BodyRoot::new("page-optional-native-defaults");
	let scope = ReactiveScope::new();
	let (form, runtime) = scope.enter(|| {
		let form = form! {
			name: OptionalNativeDefaultsForm,
			model_form: WidgetContract,
			server_fn: save_page_widgets,
			overrides: {
				color: { widget: ColorInput },
				range: { widget: RangeInput }
			},
		};
		let runtime = use_form(&form).build();
		form.server_mutation(&runtime)
			.build()
			.page()
			.mount(&Element::new(root.element.clone()))
			.unwrap();
		(form, runtime)
	});
	settle_browser().await;
	let color = control(&root.element, "color")
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap();
	let range = control(&root.element, "range")
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap();

	// Assert: browser display defaults do not supply omitted model fields.
	assert_eq!(color.value(), "#000000");
	assert_eq!(range.value(), "5");
	assert_eq!(form.value("color"), None);
	assert_eq!(form.value("range"), None);
	assert_eq!(form.data().unwrap().supplied_fields(), Vec::<&str>::new());
	let manual_range =
		into_control_binding::<NumberBinding, _>(runtime.field(WidgetField("range")), ());
	assert_eq!(manual_range.read(), ControlValue::Text(String::new()));
	for field in ["color", "range"] {
		form.set_value(field, serde_json::json!("")).unwrap();
	}
	settle_browser().await;
	assert_eq!(form.value("color"), Some(serde_json::json!("")));
	assert_eq!(form.value("range"), Some(serde_json::json!("")));
	// An explicit empty text value remains supplied under the model contract.
	assert_eq!(form.data().unwrap().supplied_fields(), vec!["color"]);
	assert_eq!(
		form.data().unwrap().get_json("color"),
		Some(serde_json::json!(""))
	);
	runtime.reset();
	settle_browser().await;
	assert!(!(runtime.form_state().is_dirty.get()));
	assert_eq!(
		control(&root.element, "__reinhardt_range_range")
			.get_attribute("value")
			.as_deref(),
		Some("5")
	);

	// Act: an explicit edit supplies the field, and reset restores omission.
	edit_input(&color, "#123456");
	edit_input(&range, "7");
	assert_eq!(form.value("color"), Some(serde_json::json!("#123456")));
	assert_eq!(form.value("range"), Some(serde_json::json!(7)));
	runtime.reset();
	settle_browser().await;
	assert_eq!(form.value("color"), None);
	assert_eq!(form.value("range"), None);
	assert_eq!(color.value(), "#000000");
	assert_eq!(range.value(), "5");
	assert!(!(runtime.form_state().is_dirty.get()));

	edit_input(&color, "#abcdef");
	edit_input(&range, "8");
	let text = control(&root.element, "text")
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap();
	edit_input(&text, "edited text");
	let boolean = control(&root.element, "boolean")
		.dyn_into::<web_sys::HtmlSelectElement>()
		.unwrap();
	boolean.set_value("false");
	boolean
		.dispatch_event(&web_sys::Event::new("change").unwrap())
		.unwrap();
	control(&root.element, "range")
		.dyn_into::<web_sys::HtmlInputElement>()
		.unwrap()
		.form()
		.unwrap()
		.reset();
	settle_browser().await;
	assert_eq!(form.value("color"), None);
	assert_eq!(form.value("range"), None);
	assert_eq!(form.value("text"), None);
	assert_eq!(form.value("boolean"), None);
	assert_eq!(form.data().unwrap().supplied_fields(), Vec::<&str>::new());
	assert!(!(runtime.form_state().is_dirty.get()));
}
