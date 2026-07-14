use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::component::{
	ControlBinding, ControlBindingError, ControlKind, ControlValue, ControlWriteOutcome,
};
use crate::dom::{Element, EventHandle};
use crate::reactive::{Effect, EffectTiming, untracked};

pub(crate) struct ControlBindingController {
	_effect: Effect,
	_listeners: Vec<EventHandle>,
	_option_observer: Option<SelectOptionObserver>,
}

struct SelectOptionObserver {
	observer: web_sys::MutationObserver,
	_callback: Closure<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>,
}

impl Drop for SelectOptionObserver {
	fn drop(&mut self) {
		self.observer.disconnect();
	}
}

impl std::fmt::Debug for ControlBindingController {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter
			.debug_struct("ControlBindingController")
			.finish_non_exhaustive()
	}
}

#[derive(Default)]
struct CompositionState {
	composing: bool,
	skip_next_input: Option<CompletedComposition>,
}

struct CompletedComposition {
	value: ControlValue,
	signal_baseline: ControlValue,
}

impl ControlBindingController {
	pub(crate) fn mount(
		element: Element,
		binding: ControlBinding,
	) -> Result<Self, ControlBindingError> {
		validate_control(&element, binding.kind())?;
		let initial_value = untracked(|| binding.read());
		write_control(&element, binding.kind(), &initial_value)?;
		Self::install(element, binding, false)
	}

	pub(crate) fn hydrate(
		element: Element,
		binding: ControlBinding,
	) -> Result<(Self, bool), ControlBindingError> {
		validate_control(&element, binding.kind())?;
		let listeners = install_listeners(&element, &binding);
		let live_value = read_control(&element, binding.kind())?;
		let expected_value = untracked(|| binding.read());
		let adopted = if binding.kind() == ControlKind::SelectOne
			&& !select_has_option_value(&element, &expected_value)
		{
			write_control(&element, binding.kind(), &expected_value)?;
			false
		} else {
			matches!(
				binding.write(live_value.clone())?,
				ControlWriteOutcome::Committed
			) && expected_value != live_value
		};
		let option_observer = install_select_option_observer(&element, &binding);
		let effect = install_effect(element, binding, true);
		Ok((
			Self {
				_effect: effect,
				_listeners: listeners,
				_option_observer: option_observer,
			},
			adopted,
		))
	}

	fn install(
		element: Element,
		binding: ControlBinding,
		skip_first_write: bool,
	) -> Result<Self, ControlBindingError> {
		let listeners = install_listeners(&element, &binding);
		let option_observer = install_select_option_observer(&element, &binding);
		let effect = install_effect(element, binding, skip_first_write);
		Ok(Self {
			_effect: effect,
			_listeners: listeners,
			_option_observer: option_observer,
		})
	}
}

fn install_select_option_observer(
	element: &Element,
	binding: &ControlBinding,
) -> Option<SelectOptionObserver> {
	if !matches!(
		binding.kind(),
		ControlKind::SelectOne | ControlKind::SelectMany
	) {
		return None;
	}

	let observed_element = element.clone();
	let observed_binding = binding.clone();
	let callback = Closure::wrap(
		Box::new(move |_: js_sys::Array, _: web_sys::MutationObserver| {
			let value = untracked(|| observed_binding.read());
			let _ = write_control(&observed_element, observed_binding.kind(), &value);
		}) as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>,
	);
	let observer = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref()).ok()?;
	let options = web_sys::MutationObserverInit::new();
	options.set_child_list(true);
	options.set_subtree(true);
	observer
		.observe_with_options(element.as_web_sys(), &options)
		.ok()?;

	Some(SelectOptionObserver {
		observer,
		_callback: callback,
	})
}

fn select_has_option_value(element: &Element, value: &ControlValue) -> bool {
	let ControlValue::Text(value) = value else {
		return true;
	};
	let Some(select) = element.as_web_sys().dyn_ref::<web_sys::HtmlSelectElement>() else {
		return true;
	};
	let options = select.options();
	(0..options.length()).any(|index| {
		options
			.item(index)
			.and_then(|option| option.dyn_into::<web_sys::HtmlOptionElement>().ok())
			.is_some_and(|option| option.value() == *value)
	})
}

fn install_effect(element: Element, binding: ControlBinding, skip_first_write: bool) -> Effect {
	let first_run = Rc::new(std::cell::Cell::new(skip_first_write));
	Effect::new_with_timing(
		move || {
			let value = binding.read();
			if first_run.replace(false) {
				return;
			}
			let _ = write_control(&element, binding.kind(), &value);
		},
		EffectTiming::Layout,
	)
}

fn install_listeners(element: &Element, binding: &ControlBinding) -> Vec<EventHandle> {
	let state = Rc::new(RefCell::new(CompositionState::default()));
	let mut listeners = Vec::new();

	match binding.kind() {
		ControlKind::Text | ControlKind::Number => {
			let input_element = element.clone();
			let input_binding = binding.clone();
			let input_state = Rc::clone(&state);
			listeners.push(
				element.add_event_listener_with_event("input", move |event| {
					let browser_is_composing = event
						.dyn_ref::<web_sys::InputEvent>()
						.is_some_and(web_sys::InputEvent::is_composing);
					let composing = input_state.borrow().composing;
					if browser_is_composing || composing {
						input_state.borrow_mut().skip_next_input = None;
						return;
					}

					let Ok(value) = read_control(&input_element, input_binding.kind()) else {
						return;
					};
					let completed = input_state.borrow_mut().skip_next_input.take();
					if let Some(completed) = completed
						&& completed.value == value
					{
						let current_value = untracked(|| input_binding.read());
						if current_value != completed.signal_baseline {
							let _ =
								write_control(&input_element, input_binding.kind(), &current_value);
						}
						return;
					}
					let _ = input_binding.write(value);
				}),
			);

			let start_state = Rc::clone(&state);
			listeners.push(
				element.add_event_listener_with_event("compositionstart", move |_| {
					let mut state = start_state.borrow_mut();
					state.composing = true;
					state.skip_next_input = None;
				}),
			);

			let end_element = element.clone();
			let end_binding = binding.clone();
			let end_state = Rc::clone(&state);
			listeners.push(
				element.add_event_listener_with_event("compositionend", move |_| {
					{
						let mut state = end_state.borrow_mut();
						state.composing = false;
						state.skip_next_input = None;
					}
					let Ok(value) = read_control(&end_element, end_binding.kind()) else {
						return;
					};
					let Ok(_) = end_binding.write(value.clone()) else {
						return;
					};
					let signal_baseline = untracked(|| end_binding.read());
					end_state.borrow_mut().skip_next_input = Some(CompletedComposition {
						value,
						signal_baseline,
					});
				}),
			);
		}
		ControlKind::Checkbox
		| ControlKind::Radio
		| ControlKind::SelectOne
		| ControlKind::SelectMany => {
			let change_element = element.clone();
			let change_binding = binding.clone();
			listeners.push(element.add_event_listener_with_event("change", move |_| {
				let Ok(value) = read_control(&change_element, change_binding.kind()) else {
					return;
				};
				let _ = change_binding.write(value);
			}));
		}
	}

	listeners
}

pub(crate) fn validate_control(
	element: &Element,
	kind: ControlKind,
) -> Result<(), ControlBindingError> {
	let tag = element.as_web_sys().tag_name().to_ascii_lowercase();
	let supported = match kind {
		ControlKind::Text => {
			tag == "textarea"
				|| (tag == "input"
					&& element
						.as_web_sys()
						.dyn_ref::<web_sys::HtmlInputElement>()
						.is_some_and(|input| input.type_() == "text"))
		}
		ControlKind::Number => input_has_type(element, &tag, "number"),
		ControlKind::Checkbox => input_has_type(element, &tag, "checkbox"),
		ControlKind::Radio => input_has_type(element, &tag, "radio"),
		ControlKind::SelectOne => select_has_multiple(element, &tag, false),
		ControlKind::SelectMany => select_has_multiple(element, &tag, true),
	};
	if supported {
		Ok(())
	} else {
		Err(ControlBindingError::UnsupportedElement {
			control: kind,
			actual_tag: tag,
		})
	}
}

fn input_has_type(element: &Element, tag: &str, expected: &str) -> bool {
	tag == "input"
		&& element
			.as_web_sys()
			.dyn_ref::<web_sys::HtmlInputElement>()
			.is_some_and(|input| input.type_() == expected)
}

fn select_has_multiple(element: &Element, tag: &str, expected: bool) -> bool {
	tag == "select"
		&& element
			.as_web_sys()
			.dyn_ref::<web_sys::HtmlSelectElement>()
			.is_some_and(|select| select.multiple() == expected)
}

pub(crate) fn read_control(
	element: &Element,
	kind: ControlKind,
) -> Result<ControlValue, ControlBindingError> {
	validate_control(element, kind)?;
	match kind {
		ControlKind::Text => {
			if let Some(input) = element.as_web_sys().dyn_ref::<web_sys::HtmlInputElement>() {
				Ok(ControlValue::Text(input.value()))
			} else if let Some(textarea) = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlTextAreaElement>()
			{
				Ok(ControlValue::Text(textarea.value()))
			} else {
				Err(missing(kind, "value"))
			}
		}
		ControlKind::Number => element
			.as_web_sys()
			.dyn_ref::<web_sys::HtmlInputElement>()
			.map(|input| ControlValue::Text(input.value()))
			.ok_or_else(|| missing(kind, "value")),
		ControlKind::Checkbox | ControlKind::Radio => element
			.as_web_sys()
			.dyn_ref::<web_sys::HtmlInputElement>()
			.map(|input| ControlValue::Checked(input.checked()))
			.ok_or_else(|| missing(kind, "checked")),
		ControlKind::SelectOne => element
			.as_web_sys()
			.dyn_ref::<web_sys::HtmlSelectElement>()
			.map(|select| ControlValue::Text(select.value()))
			.ok_or_else(|| missing(kind, "value")),
		ControlKind::SelectMany => {
			let select = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlSelectElement>()
				.ok_or_else(|| missing(kind, "selectedOptions"))?;
			let options = select.options();
			let mut values = Vec::new();
			for index in 0..options.length() {
				if let Some(option) = options.item(index)
					&& let Ok(option) = option.dyn_into::<web_sys::HtmlOptionElement>()
					&& option.selected()
				{
					values.push(option.value());
				}
			}
			Ok(ControlValue::SelectedValues(values))
		}
	}
}

pub(crate) fn write_control(
	element: &Element,
	kind: ControlKind,
	value: &ControlValue,
) -> Result<bool, ControlBindingError> {
	validate_control(element, kind)?;
	match (kind, value) {
		(ControlKind::Text, ControlValue::Text(value)) => {
			if let Some(input) = element.as_web_sys().dyn_ref::<web_sys::HtmlInputElement>() {
				if input.value() == *value {
					Ok(false)
				} else {
					input.set_value(value);
					Ok(true)
				}
			} else if let Some(textarea) = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlTextAreaElement>()
			{
				if textarea.value() == *value {
					Ok(false)
				} else {
					textarea.set_value(value);
					Ok(true)
				}
			} else {
				Err(missing(kind, "value"))
			}
		}
		(ControlKind::Number, ControlValue::Text(value)) => {
			let input = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlInputElement>()
				.ok_or_else(|| missing(kind, "value"))?;
			if input.value() == *value {
				Ok(false)
			} else {
				input.set_value(value);
				Ok(true)
			}
		}
		(ControlKind::Checkbox | ControlKind::Radio, ControlValue::Checked(value)) => {
			let input = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlInputElement>()
				.ok_or_else(|| missing(kind, "checked"))?;
			if input.checked() == *value {
				Ok(false)
			} else {
				input.set_checked(*value);
				Ok(true)
			}
		}
		(ControlKind::SelectOne, ControlValue::Text(value)) => {
			let select = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlSelectElement>()
				.ok_or_else(|| missing(kind, "value"))?;
			if select.value() == *value {
				Ok(false)
			} else {
				select.set_value(value);
				Ok(true)
			}
		}
		(ControlKind::SelectMany, ControlValue::SelectedValues(values)) => {
			let select = element
				.as_web_sys()
				.dyn_ref::<web_sys::HtmlSelectElement>()
				.ok_or_else(|| missing(kind, "selectedOptions"))?;
			let options = select.options();
			let mut changed = false;
			for index in 0..options.length() {
				if let Some(option) = options.item(index)
					&& let Ok(option) = option.dyn_into::<web_sys::HtmlOptionElement>()
				{
					let selected = values.iter().any(|value| value == &option.value());
					if option.selected() != selected {
						option.set_selected(selected);
						changed = true;
					}
				}
			}
			Ok(changed)
		}
		(_, actual) => Err(ControlBindingError::ValueKindMismatch {
			control: kind,
			actual: match actual {
				ControlValue::Text(_) => "text",
				ControlValue::Checked(_) => "checked",
				ControlValue::SelectedValues(_) => "selected-values",
			},
		}),
	}
}

fn missing(control: ControlKind, property: &'static str) -> ControlBindingError {
	ControlBindingError::MissingProperty { control, property }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::component::{ControlBinding, ControlBindingError, ControlKind};
	use crate::dom::Element;
	use crate::reactive::Signal;
	use wasm_bindgen::JsCast;
	use wasm_bindgen_test::*;

	wasm_bindgen_test_configure!(run_in_browser);

	fn element(tag: &str) -> Element {
		let raw = web_sys::window()
			.expect("window")
			.document()
			.expect("document")
			.create_element(tag)
			.expect("element");
		Element::new(raw)
	}

	#[wasm_bindgen_test]
	fn mounted_text_control_synchronizes_both_directions() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		let signal = Signal::new("signal".to_owned());
		let _controller =
			ControlBindingController::mount(element, ControlBinding::text(signal.clone()))
				.expect("binding");

		assert_eq!(input.value(), "signal");
		input.set_value("dom");
		input
			.dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
			.expect("dispatch");
		assert_eq!(signal.get(), "dom");
		signal.set("updated".to_owned());
		assert_eq!(input.value(), "updated");
	}

	#[wasm_bindgen_test]
	fn hydration_adopts_live_dom_without_initial_write() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		input.set_value("restored");
		input.set_selection_range(3, 3).expect("selection");
		let signal = Signal::new("server".to_owned());

		let _controller =
			ControlBindingController::hydrate(element, ControlBinding::text(signal.clone()))
				.expect("binding");

		assert_eq!(signal.get(), "restored");
		assert_eq!(input.selection_start().expect("selection"), Some(3));
	}

	#[wasm_bindgen_test]
	fn same_value_signal_write_preserves_caret() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		let signal = Signal::new("hello".to_owned());
		let _controller =
			ControlBindingController::mount(element, ControlBinding::text(signal.clone()))
				.expect("binding");
		input.set_selection_range(2, 2).expect("selection");

		signal.set("hello".to_owned());

		assert_eq!(input.selection_start().expect("selection"), Some(2));
	}

	#[wasm_bindgen_test]
	fn invalid_numeric_raw_is_preserved_until_signal_changes() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		input.set_type("number");
		let signal = Signal::new(12_i32);
		let _controller =
			ControlBindingController::mount(element, ControlBinding::number(signal.clone()))
				.expect("binding");
		input.set_value("2147483648");

		input
			.dispatch_event(&web_sys::InputEvent::new("input").expect("input event"))
			.expect("dispatch");

		assert_eq!(signal.get(), 12);
		assert_eq!(input.value(), "2147483648");
		signal.set(13);
		assert_eq!(input.value(), "13");
	}

	#[wasm_bindgen_test]
	fn checkbox_radio_and_select_kinds_synchronize() {
		let checkbox = element("input");
		let checkbox_input: web_sys::HtmlInputElement =
			checkbox.as_web_sys().clone().unchecked_into();
		checkbox_input.set_type("checkbox");
		let checked = Signal::new(false);
		let _checkbox_controller =
			ControlBindingController::mount(checkbox, ControlBinding::checkbox(checked.clone()))
				.expect("checkbox");
		checkbox_input.set_checked(true);
		checkbox_input
			.dispatch_event(&web_sys::Event::new("change").expect("change"))
			.expect("dispatch");
		assert!(checked.get());

		let radio = element("input");
		let radio_input: web_sys::HtmlInputElement = radio.as_web_sys().clone().unchecked_into();
		radio_input.set_type("radio");
		let selected = Signal::new("other".to_owned());
		let _radio_controller = ControlBindingController::mount(
			radio,
			ControlBinding::radio(selected.clone(), "choice".to_owned()),
		)
		.expect("radio");
		radio_input.set_checked(true);
		radio_input
			.dispatch_event(&web_sys::Event::new("change").expect("change"))
			.expect("dispatch");
		assert_eq!(selected.get(), "choice");

		let select = element("select");
		let select_input: web_sys::HtmlSelectElement = select.as_web_sys().clone().unchecked_into();
		select_input.set_multiple(true);
		for value in ["a", "b", "c"] {
			let option = web_sys::window()
				.expect("window")
				.document()
				.expect("document")
				.create_element("option")
				.expect("option");
			let option: web_sys::HtmlOptionElement = option.unchecked_into();
			option.set_value(value);
			select_input.append_child(&option).expect("append option");
		}
		let selected_many = Signal::new(vec!["b".to_owned()]);
		let _select_controller = ControlBindingController::mount(
			select,
			ControlBinding::select_many(selected_many.clone()),
		)
		.expect("select");
		assert!(
			select_input
				.options()
				.item(1)
				.expect("option")
				.unchecked_into::<web_sys::HtmlOptionElement>()
				.selected()
		);

		let select_one = element("select");
		let select_one_input: web_sys::HtmlSelectElement =
			select_one.as_web_sys().clone().unchecked_into();
		for value in ["first", "second"] {
			let option = web_sys::window()
				.expect("window")
				.document()
				.expect("document")
				.create_element("option")
				.expect("option");
			let option: web_sys::HtmlOptionElement = option.unchecked_into();
			option.set_value(value);
			select_one_input
				.append_child(&option)
				.expect("append option");
		}
		let selected_one = Signal::new("second".to_owned());
		let _select_one_controller = ControlBindingController::mount(
			select_one,
			ControlBinding::select_one(selected_one.clone()),
		)
		.expect("select one");
		assert_eq!(select_one_input.value(), "second");
	}

	#[wasm_bindgen_test]
	fn dropping_controller_detaches_listeners_and_effect() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		let signal = Signal::new("initial".to_owned());
		let controller =
			ControlBindingController::mount(element, ControlBinding::text(signal.clone()))
				.expect("binding");
		drop(controller);

		input.set_value("dom");
		input
			.dispatch_event(&web_sys::InputEvent::new("input").expect("input"))
			.expect("dispatch");
		assert_eq!(signal.get(), "initial");
		signal.set("signal".to_owned());
		assert_eq!(input.value(), "dom");
	}

	#[wasm_bindgen_test]
	fn composition_commits_once_and_deduplicates_final_input() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		let signal = Signal::new(String::new());
		let commits = Rc::new(std::cell::Cell::new(0));
		let effect_signal = signal.clone();
		let effect_commits = Rc::clone(&commits);
		let _commit_observer = Effect::new_with_timing(
			move || {
				let _ = effect_signal.get();
				effect_commits.set(effect_commits.get() + 1);
			},
			EffectTiming::Layout,
		);
		let _controller =
			ControlBindingController::mount(element, ControlBinding::text(signal.clone()))
				.expect("binding");
		input
			.dispatch_event(&web_sys::CompositionEvent::new("compositionstart").expect("start"))
			.expect("dispatch");
		input.set_value("あ");
		input
			.dispatch_event(&web_sys::InputEvent::new("input").expect("input"))
			.expect("dispatch");
		assert_eq!(signal.get(), "");
		input
			.dispatch_event(&web_sys::CompositionEvent::new("compositionend").expect("end"))
			.expect("dispatch");
		assert_eq!(signal.get(), "あ");
		input.set_selection_range(0, 0).expect("selection");
		input
			.dispatch_event(&web_sys::InputEvent::new("input").expect("input"))
			.expect("dispatch");
		assert_eq!(signal.get(), "あ");
		assert_eq!(commits.get(), 2);
		assert_eq!(input.selection_start().expect("selection"), Some(0));
	}

	#[wasm_bindgen_test]
	fn isolated_composing_input_invalidates_stale_composition_dedupe() {
		let element = element("input");
		let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
		let signal = Signal::new(String::new());
		let commits = Rc::new(std::cell::Cell::new(0));
		let effect_signal = signal.clone();
		let effect_commits = Rc::clone(&commits);
		let _commit_observer = Effect::new_with_timing(
			move || {
				let _ = effect_signal.get();
				effect_commits.set(effect_commits.get() + 1);
			},
			EffectTiming::Layout,
		);
		let _controller =
			ControlBindingController::mount(element, ControlBinding::text(signal.clone()))
				.expect("binding");
		input.set_value("same");
		input
			.dispatch_event(&web_sys::CompositionEvent::new("compositionend").expect("end"))
			.expect("dispatch");
		input
			.dispatch_event(&{
				let init = web_sys::InputEventInit::new();
				init.set_is_composing(true);
				web_sys::InputEvent::new_with_event_init_dict("input", &init)
					.expect("composing input")
					.into()
			})
			.expect("dispatch");
		input
			.dispatch_event(&web_sys::InputEvent::new("input").expect("input"))
			.expect("dispatch");
		assert_eq!(signal.get(), "same");
		assert_eq!(commits.get(), 3);
	}

	#[wasm_bindgen_test]
	fn actual_tag_mismatch_is_structured() {
		let signal = Signal::new(false);
		let error =
			ControlBindingController::mount(element("select"), ControlBinding::checkbox(signal))
				.expect_err("mismatch");
		assert_eq!(
			error,
			ControlBindingError::UnsupportedElement {
				control: ControlKind::Checkbox,
				actual_tag: "select".to_owned(),
			}
		);
	}

	#[wasm_bindgen_test]
	fn text_binding_rejects_non_text_input_types_without_writing_file_value() {
		for input_type in ["search", "email", "file", "range", "password", "url"] {
			let element = element("input");
			let input: web_sys::HtmlInputElement = element.as_web_sys().clone().unchecked_into();
			input.set_type(input_type);
			let error = ControlBindingController::mount(
				element,
				ControlBinding::text(Signal::new("non-empty".to_owned())),
			)
			.expect_err("non-text input type should fail");

			assert_eq!(
				error,
				ControlBindingError::UnsupportedElement {
					control: ControlKind::Text,
					actual_tag: "input".to_owned(),
				}
			);
			if input_type == "file" {
				assert_eq!(input.value(), "");
			}
		}
	}

	#[wasm_bindgen_test]
	fn text_binding_accepts_textarea() {
		let element = element("textarea");
		let textarea: web_sys::HtmlTextAreaElement = element.as_web_sys().clone().unchecked_into();
		let signal = Signal::new("bound".to_owned());

		let _controller = ControlBindingController::mount(element, ControlBinding::text(signal))
			.expect("textarea binding");

		assert_eq!(textarea.value(), "bound");
	}
}
