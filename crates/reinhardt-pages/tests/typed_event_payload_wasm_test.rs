#![cfg(wasm)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::{Array, Function, Reflect};
#[cfg(feature = "i18n")]
use reinhardt_i18n::TranslationContext;
use reinhardt_pages::event::{
	AbortEvent, AnimationStartEvent, BeforeXrSelectEvent, BeginEvent, ChangeEvent, ClickEvent,
	CommandEvent, CompositionStartEvent, CopyEvent, DblClickEvent, DragStartEvent, EncryptedEvent,
	EnterPictureInPictureEvent, EventPayload, EventTargetError, FocusEvent, InputEvent,
	KeyDownEvent, PointerDownEvent, SecurityPolicyViolationEvent, SubmitEvent, ToggleEvent,
	TouchStartEvent, TransitionStartEvent, WheelEvent,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_test::*;

#[cfg(feature = "i18n")]
use reinhardt_pages::{
	I18nContext, provide_i18n_context, raw_async_event_handler, use_i18n_context,
};

wasm_bindgen_test_configure!(run_in_browser);

struct MountedElement(web_sys::Element);

impl MountedElement {
	fn new(tag: &str) -> Self {
		let document = web_sys::window()
			.expect("browser window must exist")
			.document()
			.expect("browser document must exist");
		let element = document
			.create_element(tag)
			.expect("test element must be created");
		document
			.body()
			.expect("browser document must have a body")
			.append_child(&element)
			.expect("test element must mount");
		Self(element)
	}
}

impl Drop for MountedElement {
	fn drop(&mut self) {
		self.0.remove();
	}
}

struct EventListener {
	target: web_sys::EventTarget,
	event_type: &'static str,
	callback: Closure<dyn FnMut(web_sys::Event)>,
}

impl EventListener {
	fn new(
		target: &web_sys::EventTarget,
		event_type: &'static str,
		callback: impl FnMut(web_sys::Event) + 'static,
	) -> Self {
		let callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut(web_sys::Event)>);
		target
			.add_event_listener_with_callback(event_type, callback.as_ref().unchecked_ref())
			.expect("test listener must attach");
		Self {
			target: target.clone(),
			event_type,
			callback,
		}
	}
}

impl Drop for EventListener {
	fn drop(&mut self) {
		self.target
			.remove_event_listener_with_callback(
				self.event_type,
				self.callback.as_ref().unchecked_ref(),
			)
			.expect("test listener must detach");
	}
}

fn bubbling_click() -> web_sys::MouseEvent {
	let init = web_sys::MouseEventInit::new();
	init.set_bubbles(true);
	init.set_cancelable(true);
	web_sys::MouseEvent::new_with_mouse_event_init_dict("click", &init)
		.expect("click event must be created")
}

#[wasm_bindgen_test(async)]
async fn nested_targets_and_async_current_target_are_snapshotted() {
	let parent = MountedElement::new("div");
	let child = web_sys::window()
		.expect("window")
		.document()
		.expect("document")
		.create_element("button")
		.expect("button");
	parent.0.append_child(&child).expect("child must mount");
	let captured = Rc::new(RefCell::new(None));
	let captured_for_listener = Rc::clone(&captured);
	let target: web_sys::EventTarget = parent.0.clone().unchecked_into();
	let _listener = EventListener::new(&target, "click", move |raw| {
		let payload = ClickEvent::try_from_raw(raw).expect("mouse click fallback must convert");
		*captured_for_listener.borrow_mut() = Some(payload);
	});

	child
		.dispatch_event(&bubbling_click())
		.expect("click dispatch must succeed");
	reinhardt_pages::platform::defer_yield().await;

	let payload = captured.borrow().clone().expect("listener must run");
	assert_eq!(
		payload.target().expect("origin target").tag_name(),
		"button"
	);
	assert_eq!(
		payload
			.current_target()
			.expect("listener target snapshot")
			.tag_name(),
		"div"
	);
	assert_eq!(payload.raw().type_(), "click");
}

#[cfg(feature = "i18n")]
#[wasm_bindgen_test(async)]
async fn raw_async_event_handlers_preserve_i18n_context_across_await() {
	let context = I18nContext::new(TranslationContext::new("ja", "en-US"));
	let observed_context = Rc::new(Cell::new(false));
	let handler = raw_async_event_handler({
		let observed_context = Rc::clone(&observed_context);
		move |_| {
			let observed_context = Rc::clone(&observed_context);
			async move {
				reinhardt_pages::platform::defer_yield().await;
				observed_context.set(use_i18n_context().is_some());
			}
		}
	});

	let event = web_sys::Event::new("custom").expect("custom event must be created");
	let _guard = provide_i18n_context(context);
	handler(event);
	drop(_guard);
	reinhardt_pages::platform::defer_yield().await;
	reinhardt_pages::platform::defer_yield().await;

	assert!(observed_context.get());
}

#[wasm_bindgen_test]
fn click_accepts_pointer_primary_and_mouse_fallback_interfaces() {
	let pointer = web_sys::PointerEvent::new("click").expect("pointer event");
	let pointer: web_sys::Event = pointer.unchecked_into();
	let pointer = ClickEvent::try_from_raw(pointer).expect("pointer click must convert");
	assert_eq!(pointer.event_type(), "click");

	let mouse = web_sys::MouseEvent::new("click").expect("mouse event");
	let mouse: web_sys::Event = mouse.unchecked_into();
	let mouse = ClickEvent::try_from_raw(mouse).expect("mouse fallback must convert");
	assert_eq!(mouse.event_type(), "click");

	let keyboard = web_sys::KeyboardEvent::new("click").expect("keyboard event");
	let keyboard: web_sys::Event = keyboard.unchecked_into();
	assert!(ClickEvent::try_from_raw(keyboard).is_err());
}

#[wasm_bindgen_test]
fn input_accepts_base_event_fallback_and_rejects_specialized_wrong_family() {
	let base = web_sys::Event::new("input").expect("base input event");
	InputEvent::try_from_raw(base).expect("base Event is the cataloged input fallback");

	let keyboard = web_sys::KeyboardEvent::new("input").expect("keyboard input event");
	let keyboard: web_sys::Event = keyboard.unchecked_into();
	assert!(InputEvent::try_from_raw(keyboard).is_err());
}

#[wasm_bindgen_test]
fn control_capabilities_read_browser_target_state() {
	let input = MountedElement::new("input");
	let input: web_sys::HtmlInputElement = input.0.clone().unchecked_into();
	input.set_value("Ada");
	let observed_value = Rc::new(RefCell::new(None));
	let observed_value_for_listener = Rc::clone(&observed_value);
	let target: web_sys::EventTarget = input.clone().unchecked_into();
	let _input_listener = EventListener::new(&target, "input", move |raw| {
		let payload = InputEvent::try_from_raw(raw).expect("input payload must convert");
		*observed_value_for_listener.borrow_mut() = Some(payload.value());
	});
	input
		.dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
		.expect("input dispatch");
	assert_eq!(
		observed_value.borrow().as_ref(),
		Some(&Ok::<String, reinhardt_pages::event::EventTargetError>(
			"Ada".to_owned()
		))
	);

	let checkbox = MountedElement::new("input");
	let checkbox: web_sys::HtmlInputElement = checkbox.0.clone().unchecked_into();
	checkbox.set_type("checkbox");
	checkbox.set_checked(true);
	let observed_checked = Rc::new(RefCell::new(None));
	let observed_checked_for_listener = Rc::clone(&observed_checked);
	let target: web_sys::EventTarget = checkbox.clone().unchecked_into();
	let _checkbox_listener = EventListener::new(&target, "change", move |raw| {
		let payload = ChangeEvent::try_from_raw(raw).expect("change payload must convert");
		*observed_checked_for_listener.borrow_mut() = Some(payload.checked());
	});
	checkbox
		.dispatch_event(&web_sys::Event::new("change").expect("change event"))
		.expect("checkbox dispatch");
	assert_eq!(
		observed_checked.borrow().as_ref(),
		Some(&Ok::<bool, reinhardt_pages::event::EventTargetError>(true))
	);

	let select = MountedElement::new("select");
	let select: web_sys::HtmlSelectElement = select.0.clone().unchecked_into();
	select.set_multiple(true);
	for (value, selected) in [("red", true), ("green", false), ("blue", true)] {
		let option = web_sys::window()
			.expect("window")
			.document()
			.expect("document")
			.create_element("option")
			.expect("option");
		let option: web_sys::HtmlOptionElement = option.unchecked_into();
		option.set_value(value);
		option.set_selected(selected);
		select.append_child(&option).expect("option must append");
	}
	let observed_selected = Rc::new(RefCell::new(None));
	let observed_selected_for_listener = Rc::clone(&observed_selected);
	let target: web_sys::EventTarget = select.clone().unchecked_into();
	let _select_listener = EventListener::new(&target, "change", move |raw| {
		let payload = ChangeEvent::try_from_raw(raw).expect("change payload must convert");
		*observed_selected_for_listener.borrow_mut() = Some(payload.selected_values());
	});
	select
		.dispatch_event(&web_sys::Event::new("change").expect("change event"))
		.expect("select dispatch");
	assert_eq!(
		observed_selected.borrow().as_ref(),
		Some(
			&Ok::<Vec<String>, reinhardt_pages::event::EventTargetError>(vec![
				"red".to_owned(),
				"blue".to_owned()
			])
		)
	);

	let file_input = MountedElement::new("input");
	let file_input: web_sys::HtmlInputElement = file_input.0.clone().unchecked_into();
	file_input.set_type("file");
	let observed_files = Rc::new(RefCell::new(None));
	let observed_files_for_listener = Rc::clone(&observed_files);
	let target: web_sys::EventTarget = file_input.clone().unchecked_into();
	let _file_listener = EventListener::new(&target, "change", move |raw| {
		let payload = ChangeEvent::try_from_raw(raw).expect("change payload must convert");
		*observed_files_for_listener.borrow_mut() = Some(payload.files());
	});
	file_input
		.dispatch_event(&web_sys::Event::new("change").expect("change event"))
		.expect("file dispatch");
	assert_eq!(
		observed_files.borrow().as_ref(),
		Some(&Ok::<
			Vec<reinhardt_pages::event::EventFile>,
			reinhardt_pages::event::EventTargetError,
		>(Vec::new()))
	);
}

#[wasm_bindgen_test]
fn value_capability_reads_textarea_and_contenteditable_state() {
	let textarea = MountedElement::new("textarea");
	let textarea_control: web_sys::HtmlTextAreaElement = textarea.0.clone().unchecked_into();
	textarea_control.set_value("first line\nsecond line");
	let observed_textarea = Rc::new(RefCell::new(None));
	let observed_textarea_for_listener = Rc::clone(&observed_textarea);
	let target: web_sys::EventTarget = textarea_control.clone().unchecked_into();
	let _textarea_listener = EventListener::new(&target, "input", move |raw| {
		let payload = InputEvent::try_from_raw(raw).expect("textarea input payload must convert");
		*observed_textarea_for_listener.borrow_mut() = Some(payload.value());
	});
	textarea_control
		.dispatch_event(&web_sys::InputEvent::new("input").expect("textarea input event"))
		.expect("textarea input dispatch");
	assert_eq!(
		observed_textarea.borrow().as_ref(),
		Some(&Ok::<String, EventTargetError>(
			"first line\nsecond line".to_owned()
		))
	);

	let editable = MountedElement::new("div");
	editable
		.0
		.set_attribute("contenteditable", "true")
		.expect("contenteditable attribute must be set");
	editable.0.set_text_content(Some("editable text"));
	let observed_editable = Rc::new(RefCell::new(None));
	let observed_editable_for_listener = Rc::clone(&observed_editable);
	let target: web_sys::EventTarget = editable.0.clone().unchecked_into();
	let _editable_listener = EventListener::new(&target, "input", move |raw| {
		let payload =
			InputEvent::try_from_raw(raw).expect("contenteditable input payload must convert");
		*observed_editable_for_listener.borrow_mut() = Some(payload.value());
	});
	editable
		.0
		.dispatch_event(&web_sys::InputEvent::new("input").expect("contenteditable input event"))
		.expect("contenteditable input dispatch");
	assert_eq!(
		observed_editable.borrow().as_ref(),
		Some(&Ok::<String, EventTargetError>("editable text".to_owned()))
	);
}

#[wasm_bindgen_test]
fn checked_capability_reads_radio_state() {
	let radio = MountedElement::new("input");
	let radio_control: web_sys::HtmlInputElement = radio.0.clone().unchecked_into();
	radio_control.set_type("radio");
	radio_control.set_checked(true);
	let observed = Rc::new(RefCell::new(None));
	let observed_for_listener = Rc::clone(&observed);
	let target: web_sys::EventTarget = radio_control.clone().unchecked_into();
	let _listener = EventListener::new(&target, "change", move |raw| {
		let payload = ChangeEvent::try_from_raw(raw).expect("radio change payload must convert");
		*observed_for_listener.borrow_mut() = Some(payload.checked());
	});
	radio_control
		.dispatch_event(&web_sys::Event::new("change").expect("radio change event"))
		.expect("radio change dispatch");
	assert_eq!(
		observed.borrow().as_ref(),
		Some(&Ok::<bool, EventTargetError>(true))
	);
}

#[wasm_bindgen_test]
fn target_capability_errors_preserve_exact_context() {
	let raw: web_sys::Event = web_sys::InputEvent::new("input")
		.expect("detached input event")
		.unchecked_into();
	let detached = InputEvent::try_from_raw(raw).expect("detached input payload must convert");
	assert_eq!(
		detached.value(),
		Err(EventTargetError::MissingCurrentTarget { event: "input" })
	);

	let unsupported = MountedElement::new("div");
	let observed_unsupported = Rc::new(RefCell::new(None));
	let observed_unsupported_for_listener = Rc::clone(&observed_unsupported);
	let target: web_sys::EventTarget = unsupported.0.clone().unchecked_into();
	let _unsupported_listener = EventListener::new(&target, "input", move |raw| {
		let payload = InputEvent::try_from_raw(raw).expect("div input payload must convert");
		*observed_unsupported_for_listener.borrow_mut() = Some(payload.value());
	});
	unsupported
		.0
		.dispatch_event(&web_sys::InputEvent::new("input").expect("div input event"))
		.expect("div input dispatch");
	assert_eq!(
		observed_unsupported.borrow().as_ref(),
		Some(&Err::<String, EventTargetError>(
			EventTargetError::UnsupportedElement {
				event: "input",
				actual_tag: "div".to_owned(),
				expected: &["input", "textarea", "select", "contenteditable"],
			}
		))
	);

	let text_input = MountedElement::new("input");
	let text_control: web_sys::HtmlInputElement = text_input.0.clone().unchecked_into();
	text_control.set_type("text");
	let observed_property = Rc::new(RefCell::new(None));
	let observed_property_for_listener = Rc::clone(&observed_property);
	let target: web_sys::EventTarget = text_control.clone().unchecked_into();
	let _property_listener = EventListener::new(&target, "change", move |raw| {
		let payload = ChangeEvent::try_from_raw(raw).expect("text change payload must convert");
		*observed_property_for_listener.borrow_mut() = Some(payload.checked());
	});
	text_control
		.dispatch_event(&web_sys::Event::new("change").expect("text change event"))
		.expect("text change dispatch");
	assert_eq!(
		observed_property.borrow().as_ref(),
		Some(&Err::<bool, EventTargetError>(
			EventTargetError::UnsupportedProperty {
				event: "change",
				property: "checked",
				actual_tag: "input".to_owned(),
			}
		))
	);
}

fn browser_file(name: &str, contents: &str, media_type: &str, last_modified: f64) -> web_sys::File {
	let bits = Array::new();
	bits.push(&wasm_bindgen::JsValue::from_str(contents));
	let options = js_sys::Object::new();
	Reflect::set(
		&options,
		&wasm_bindgen::JsValue::from_str("type"),
		&wasm_bindgen::JsValue::from_str(media_type),
	)
	.expect("file media type must be set");
	Reflect::set(
		&options,
		&wasm_bindgen::JsValue::from_str("lastModified"),
		&wasm_bindgen::JsValue::from_f64(last_modified),
	)
	.expect("file timestamp must be set");
	let arguments = Array::new();
	arguments.push(&bits);
	arguments.push(&wasm_bindgen::JsValue::from_str(name));
	arguments.push(&options);
	let constructor = Reflect::get(&js_sys::global(), &wasm_bindgen::JsValue::from_str("File"))
		.expect("File constructor must exist")
		.dyn_into::<Function>()
		.expect("File must be a constructor");
	Reflect::construct(&constructor, &arguments)
		.expect("browser file must be constructed")
		.unchecked_into()
}

#[wasm_bindgen_test]
fn files_capability_preserves_browser_metadata_and_raw_file() {
	let source = browser_file("notes.txt", "hello", "text/plain", 1_700_000_000_000.0);
	let transfer = web_sys::DataTransfer::new().expect("DataTransfer must be constructible");
	let items = Reflect::get(transfer.as_ref(), &wasm_bindgen::JsValue::from_str("items"))
		.expect("DataTransfer items must exist");
	let add = Reflect::get(&items, &wasm_bindgen::JsValue::from_str("add"))
		.expect("DataTransfer items.add must exist")
		.dyn_into::<Function>()
		.expect("DataTransfer items.add must be callable");
	add.call1(&items, source.as_ref())
		.expect("file must be added to DataTransfer");

	let file_input = MountedElement::new("input");
	let file_control: web_sys::HtmlInputElement = file_input.0.clone().unchecked_into();
	file_control.set_type("file");
	file_control.set_files(transfer.files().as_ref());
	let observed = Rc::new(RefCell::new(None));
	let observed_for_listener = Rc::clone(&observed);
	let target: web_sys::EventTarget = file_control.clone().unchecked_into();
	let _listener = EventListener::new(&target, "change", move |raw| {
		let payload = ChangeEvent::try_from_raw(raw).expect("file change payload must convert");
		*observed_for_listener.borrow_mut() = Some(payload.files());
	});
	file_control
		.dispatch_event(&web_sys::Event::new("change").expect("file change event"))
		.expect("file change dispatch");

	let observed = observed.borrow();
	let files = observed
		.as_ref()
		.expect("file listener must run")
		.as_ref()
		.expect("file capability must succeed");
	assert_eq!(files.len(), 1);
	let file = &files[0];
	assert_eq!(file.name(), "notes.txt");
	assert_eq!(file.media_type(), "text/plain");
	assert_eq!(file.size(), 5);
	assert_eq!(file.last_modified(), 1_700_000_000_000);
	assert!(js_sys::Object::is(file.raw().as_ref(), source.as_ref()));
}

#[wasm_bindgen_test]
fn propagation_and_default_prevention_follow_browser_dispatch() {
	let grandparent = MountedElement::new("section");
	let parent = web_sys::window()
		.expect("window")
		.document()
		.expect("document")
		.create_element("div")
		.expect("parent");
	let child = web_sys::window()
		.expect("window")
		.document()
		.expect("document")
		.create_element("button")
		.expect("child");
	grandparent.0.append_child(&parent).expect("parent mount");
	parent.append_child(&child).expect("child mount");
	let parent_called = Rc::new(Cell::new(false));
	let parent_called_for_listener = Rc::clone(&parent_called);
	let parent_target: web_sys::EventTarget = parent.unchecked_into();
	let _parent_listener = EventListener::new(&parent_target, "click", move |raw| {
		let payload = ClickEvent::try_from_raw(raw).expect("click payload must convert");
		parent_called_for_listener.set(true);
		payload.prevent_default();
		payload.stop_propagation();
	});
	let grandparent_called = Rc::new(Cell::new(false));
	let grandparent_called_for_listener = Rc::clone(&grandparent_called);
	let grandparent_target: web_sys::EventTarget = grandparent.0.clone().unchecked_into();
	let _grandparent_listener = EventListener::new(&grandparent_target, "click", move |_| {
		grandparent_called_for_listener.set(true);
	});
	let click = bubbling_click();

	let dispatch_result = child
		.dispatch_event(&click)
		.expect("click dispatch must complete");

	assert!(!dispatch_result);
	assert!(click.default_prevented());
	assert!(parent_called.get());
	assert!(!grandparent_called.get());
}

fn construct_browser_event(constructor_name: &str, event_name: &str) -> Option<web_sys::Event> {
	let constructor = Reflect::get(
		&js_sys::global(),
		&wasm_bindgen::JsValue::from_str(constructor_name),
	)
	.ok()?
	.dyn_into::<Function>()
	.ok()?;
	let arguments = Array::new();
	arguments.push(&wasm_bindgen::JsValue::from_str(event_name));
	Reflect::construct(&constructor, &arguments)
		.ok()?
		.dyn_into::<web_sys::Event>()
		.ok()
}

fn assert_browser_family<P: EventPayload>(constructor_name: &str, event_name: &str) -> bool {
	let Some(raw) = construct_browser_event(constructor_name, event_name) else {
		return false;
	};
	P::try_from_raw(raw).expect("available browser interface must convert");
	true
}

#[wasm_bindgen_test]
fn browser_constructors_cover_stable_interface_families() {
	let covered = [
		assert_browser_family::<AbortEvent>("Event", "abort"),
		assert_browser_family::<AnimationStartEvent>("AnimationEvent", "animationstart"),
		assert_browser_family::<CopyEvent>("ClipboardEvent", "copy"),
		assert_browser_family::<CompositionStartEvent>("CompositionEvent", "compositionstart"),
		assert_browser_family::<DragStartEvent>("DragEvent", "dragstart"),
		assert_browser_family::<FocusEvent>("FocusEvent", "focus"),
		assert_browser_family::<InputEvent>("InputEvent", "input"),
		assert_browser_family::<KeyDownEvent>("KeyboardEvent", "keydown"),
		assert_browser_family::<DblClickEvent>("MouseEvent", "dblclick"),
		assert_browser_family::<PointerDownEvent>("PointerEvent", "pointerdown"),
		assert_browser_family::<SecurityPolicyViolationEvent>(
			"SecurityPolicyViolationEvent",
			"securitypolicyviolation",
		),
		assert_browser_family::<SubmitEvent>("SubmitEvent", "submit"),
		assert_browser_family::<TouchStartEvent>("TouchEvent", "touchstart"),
		assert_browser_family::<TransitionStartEvent>("TransitionEvent", "transitionstart"),
		assert_browser_family::<WheelEvent>("WheelEvent", "wheel"),
	];
	assert!(covered.into_iter().all(|available| available));

	let _optional_experimental_coverage = [
		assert_browser_family::<CommandEvent>("CommandEvent", "command"),
		assert_browser_family::<EncryptedEvent>("MediaEncryptedEvent", "encrypted"),
		assert_browser_family::<EnterPictureInPictureEvent>(
			"PictureInPictureEvent",
			"enterpictureinpicture",
		),
		assert_browser_family::<BeginEvent>("TimeEvent", "beginEvent"),
		assert_browser_family::<ToggleEvent>("ToggleEvent", "toggle"),
		assert_browser_family::<BeforeXrSelectEvent>("XRInputSourceEvent", "beforexrselect"),
	];
}
