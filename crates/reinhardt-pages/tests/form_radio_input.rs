//! Native markup and browser behavior for a single `form!` radio control.

use reinhardt_pages::{form, use_form};

#[cfg(wasm)]
use wasm_bindgen_test::*;

#[cfg(wasm)]
wasm_bindgen_test_configure!(run_in_browser);

#[cfg_attr(wasm, wasm_bindgen_test)]
#[cfg_attr(not(wasm), rstest::rstest)]
fn radio_input_renders_fixed_value_and_reactive_checked_state() {
	// Arrange
	let radio = form! {
		name: RadioForm,
		action: "/answer",
		method: Get,
		fields: {
			answer: ChoiceField<String> {
				widget: RadioInput,
				label: "Answer",
			}
		}
	};
	let page = radio.clone().into_page();
	let unchecked = concat!(
		"<form id=\"radio-form\" action=\"/answer\" method=\"get\" class=\"reinhardt-form\">",
		"<div class=\"reinhardt-field\"><label for=\"answer\" class=\"reinhardt-label\">Answer</label>",
		"<input type=\"radio\" name=\"answer\" id=\"answer\" class=\"reinhardt-input\" value=\"on\" />",
		"</div></form>",
	);

	// Act and assert: an unselected radio retains its fixed submission value.
	assert_eq!(radio.answer().get(), "");
	assert_eq!(page.render_to_string(), unchecked);
	radio.answer().set("on".into());
	assert_eq!(
		page.render_to_string(),
		unchecked.replace("value=\"on\"", "value=\"on\" checked=\"checked\""),
	);
	radio.answer().set("other".into());
	assert_eq!(page.render_to_string(), unchecked);
}

#[cfg_attr(wasm, wasm_bindgen_test)]
#[cfg_attr(not(wasm), rstest::rstest)]
fn radio_input_renders_option_labels_disabled_and_unbound_defaults() {
	// Arrange
	let radio = form! {
		name: RadioOptions,
		action: "/answer",
		method: Get,
		fields: {
			answer: ChoiceField {
				widget: RadioInput,
				choices: [("yes", "Yes") { disabled }],
				autocomplete: "off",
				autofocus,
				initial: "yes",
				required,
			}
			snapshot: ChoiceField<::std::string::String> {
				widget: RadioInput,
				choices: [("accepted", "Option label")],
				label: "Explicit label",
				initial: "accepted",
				bind: false,
				autofocus: true,
			}
		}
	};
	let page = radio.clone().into_page();
	let expected = concat!(
		"<form id=\"radio-options\" action=\"/answer\" method=\"get\" class=\"reinhardt-form\">",
		"<div class=\"reinhardt-field\"><label for=\"answer\" class=\"reinhardt-label\">Yes</label>",
		"<input type=\"radio\" name=\"answer\" id=\"answer\" class=\"reinhardt-input\" autocomplete=\"off\" required=\"required\" disabled=\"disabled\" autofocus=\"autofocus\" value=\"yes\" checked=\"checked\" />",
		"</div><div class=\"reinhardt-field\"><label for=\"snapshot\" class=\"reinhardt-label\">Explicit label</label>",
		"<input type=\"radio\" name=\"snapshot\" id=\"snapshot\" value=\"accepted\" class=\"reinhardt-input\" checked=\"checked\" autofocus=\"autofocus\" />",
		"</div></form>",
	);

	// Act and assert: bind:false snapshots checked state when the page is built.
	assert_eq!(page.render_to_string(), expected);
	radio.snapshot().set(String::new());
	assert_eq!(page.render_to_string(), expected);
}

fn input_tags(html: &str) -> Vec<String> {
	html.split("<input ")
		.skip(1)
		.map(|rest| {
			let (attributes, _) = rest.split_once(" />").expect("input tag closes");
			format!("<input {attributes} />")
		})
		.collect()
}

#[cfg_attr(wasm, wasm_bindgen_test)]
#[cfg_attr(not(wasm), rstest::rstest)]
fn native_radio_reset_compatibility_clears_nested_touched_state() {
	// Arrange: application-owned values have already been synchronized from the DOM.
	let radio = form! {
		name: CompatibilityRadios,
		fields: {
			answer: ChoiceField<String> { widget: RadioInput }
			answers: FieldArray {
				fields: {
					answer: CharField {}
				}
			}
		}
	};
	let runtime = use_form(&radio).build();
	let collection = radio.answers_collection();
	let key = runtime.push_item(collection, radio.new_answers_item());
	let path = radio.answers_answer_path(key);
	runtime.set_path_value(path.clone(), String::from("browser value"));
	assert!(runtime.get_collection_state(collection).is_touched);
	assert!(runtime.get_path_state(path.clone()).is_touched);

	// Act
	runtime.sync_after_native_reset();

	// Assert: compatibility synchronization clears nested flags without resetting values.
	assert!(!runtime.get_collection_state(collection).is_touched);
	assert!(!runtime.get_path_state(path).is_touched);
	assert!(!runtime.form_state().is_touched.get());
	assert!(runtime.form_state().is_dirty.get());
	assert_eq!(runtime.get_values().answers[0].answer, "browser value");
}

#[cfg_attr(wasm, wasm_bindgen_test)]
#[cfg_attr(not(wasm), rstest::rstest)]
fn collection_radio_input_renders_indexed_names_and_programmatic_values() {
	// Arrange
	let radio = form! {
		name: CollectionRadios,
		action: "/answer",
		method: Get,
		fields: {
			answers: FieldArray {
				fields: {
					answer: ChoiceField<String> {
						widget: RadioInput,
						choices: [("yes", "Yes")],
						autocomplete: "off",
					}
				}
			}
		}
	};
	let runtime = use_form(&radio).build();
	let key = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
	let path = radio.answers_answer_path(key);
	let page = radio.into_page();
	let unchecked = "<input type=\"radio\" name=\"answers[0][answer]\" id=\"answers_0_answer\" class=\"reinhardt-input\" autocomplete=\"off\" value=\"yes\" />";

	// Act and assert
	assert_eq!(input_tags(&page.render_to_string()), [unchecked]);
	runtime.set_path_value(path, String::from("yes"));
	assert_eq!(
		input_tags(&page.render_to_string()),
		[unchecked.replace(" />", " checked=\"checked\" />")],
	);
}

#[cfg_attr(wasm, wasm_bindgen_test)]
#[cfg_attr(not(wasm), rstest::rstest)]
#[cfg_attr(not(wasm), tokio::test)]
async fn native_radio_reset_epoch_clears_runtime_state_after_nested_batches() {
	use reinhardt_pages::{FieldError, RevalidateOn, reactive::batch};
	use std::{cell::Cell, rc::Rc};

	// Arrange: metadata synchronization must compute dirtiness from the final source snapshot.
	let radio = form! {
		name: ResetEpochRadios,
		fields: {
			answer: ChoiceField<String> { widget: RadioInput }
			answers: FieldArray {
				fields: {
					answer: CharField {}
				}
			}
		}
	};
	let runtime = use_form(&radio).revalidate_on(RevalidateOn::Change).build();
	let key = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
	let path = radio.answers_answer_path(key);
	runtime.set_path_value(path.clone(), String::from("kept"));
	runtime.set_path_error(path.clone(), FieldError::new("nested error"));
	runtime.set_value(radio.answer_field(), String::from("on"));
	runtime.set_error(radio.answer_field(), FieldError::new("radio error"));
	let event_count = Rc::new(Cell::new(0));
	let observed_events = Rc::clone(&event_count);
	let _subscription = runtime.subscribe(move |_| observed_events.set(observed_events.get() + 1));

	// Act: the epoch survives the outer batch even when inner reset writes are deferred.
	batch(|| {
		batch(|| {
			radio.answer().set(String::new());
			radio
				.__native_reset_epoch
				.update(|epoch| *epoch = epoch.wrapping_add(1));
		});
	});

	#[cfg(wasm)]
	gloo_timers::future::TimeoutFuture::new(0).await;

	// Assert: native reset preserves unrelated values and clears every touched/error map.
	assert_eq!(runtime.get_values().answer, "");
	assert_eq!(runtime.watch().get().answer, "");
	assert_eq!(runtime.get_values().answers[0].answer, "kept");
	assert!(runtime.form_state().is_dirty.get());
	assert!(!runtime.form_state().is_touched.get());
	assert!(!runtime.get_field_state(radio.answer_field()).is_touched);
	assert!(
		!runtime
			.get_collection_state(radio.answers_collection())
			.is_touched
	);
	assert!(!runtime.get_path_state(path.clone()).is_touched);
	assert_eq!(runtime.get_field_state(radio.answer_field()).error, None);
	assert_eq!(runtime.get_path_state(path).error, None);
	assert_eq!(runtime.form_state().error.get(), None);
	assert_eq!(event_count.get(), 0);

	// An unchanged value still needs to discard errors left after validation.
	runtime.set_value(radio.answer_field(), String::new());
	runtime.set_error(radio.answer_field(), FieldError::new("unchanged error"));
	event_count.set(0);
	radio
		.__native_reset_epoch
		.update(|epoch| *epoch = epoch.wrapping_add(1));
	assert!(!runtime.form_state().is_touched.get());
	assert_eq!(runtime.form_state().error.get(), None);
	assert_eq!(event_count.get(), 0);

	// Ordinary changes after a reset still update the watched values and notify subscribers.
	radio.answer().set(String::from("on"));
	assert_eq!(runtime.watch().get().answer, "on");
	assert!(runtime.form_state().is_touched.get());
	assert_eq!(event_count.get(), 2);
}

#[rstest::rstest]
#[cfg_attr(wasm, test_attr(wasm_bindgen_test))]
fn required_radio_input_rejects_unselected_scalar_and_collection_values() {
	use reinhardt_pages::UseFormSubmitOutcome;

	// Arrange: required radios compare against their option, including an empty option.
	let radio = form! {
		name: RequiredRadios,
		fields: {
			answer: ChoiceField<String> {
				widget: RadioInput,
				required
			}
			empty: ChoiceField<String> {
				widget: RadioInput,
				required,
				choices: [("", "Empty")],
				initial: "other"
			}
			answers: FieldArray {
				fields: {
					answer: ChoiceField<String> {
						widget: RadioInput,
						required,
						choices: [("yes", "Yes")]
					}
				}
			}
		}
	};
	let runtime = use_form(&radio).build();
	let key = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
	let path = radio.answers_answer_path(key);

	// Act and assert: neither empty nor nonmatching values can submit.
	for value in ["", "other"] {
		runtime.set_value(radio.answer_field(), String::from(value));
		runtime.set_path_value(path.clone(), String::from(value));
		assert_eq!(
			runtime.handle_submit(),
			UseFormSubmitOutcome::ValidationFailed
		);
		assert_eq!(
			runtime
				.get_field_state(radio.answer_field())
				.error
				.unwrap()
				.message(),
			"answer is required"
		);
		assert_eq!(
			runtime
				.get_field_state(radio.empty_field())
				.error
				.unwrap()
				.message(),
			"empty is required"
		);
		assert_eq!(
			runtime
				.get_path_state(path.clone())
				.error
				.unwrap()
				.message(),
			"answers.answer is required"
		);
	}
	runtime.set_value(radio.answer_field(), String::from("on"));
	runtime.set_value(radio.empty_field(), String::new());
	runtime.set_path_value(path, String::from("yes"));
	assert_eq!(runtime.trigger(), Ok(()));
}

#[cfg(wasm)]
mod browser {
	use super::*;
	use reinhardt_pages::component::{
		IntoPage, Page, PageElement, PageExt, cleanup_reactive_nodes,
	};
	use reinhardt_pages::dom::Element;
	use reinhardt_pages::{FieldError, RevalidateOn};
	use std::{cell::Cell, rc::Rc};
	use wasm_bindgen::JsCast;

	struct HydratedPage(Page);

	impl reinhardt_pages::component::Component for HydratedPage {
		fn name() -> &'static str {
			"HydratedRadioPage"
		}
		fn render(&self) -> Page {
			self.0.clone()
		}
	}

	struct SsrStateElement(web_sys::Element);

	impl Drop for SsrStateElement {
		fn drop(&mut self) {
			self.0.remove();
		}
	}

	struct TestContainer(web_sys::Element, reinhardt_pages::reactive::ReactiveScope);

	impl TestContainer {
		fn mount(page: Page) -> Self {
			let document = web_sys::window().unwrap().document().unwrap();
			let container = Self(
				document.create_element("div").unwrap(),
				reinhardt_pages::reactive::ReactiveScope::new(),
			);
			document.body().unwrap().append_child(&container.0).unwrap();
			container
				.1
				.enter(|| page.mount(&Element::new(container.0.clone())))
				.unwrap();
			container
		}

		fn hydrate(&self, page: Page) {
			let document = web_sys::window().unwrap().document().unwrap();
			let state = SsrStateElement(document.create_element("script").unwrap());
			state.0.set_id("ssr-state");
			document.body().unwrap().append_child(&state.0).unwrap();
			let root = Element::new(self.0.first_element_child().unwrap());
			self.1
				.enter(|| reinhardt_pages::hydration::hydrate(&HydratedPage(page), &root))
				.unwrap();
		}

		fn input(&self, id: &str) -> web_sys::HtmlInputElement {
			self.0
				.query_selector(&format!("#{id}"))
				.unwrap()
				.expect("radio is mounted")
				.dyn_into()
				.unwrap()
		}

		fn native_form(&self) -> web_sys::HtmlFormElement {
			self.0
				.query_selector("form")
				.unwrap()
				.unwrap()
				.dyn_into()
				.unwrap()
		}
	}

	impl Drop for TestContainer {
		fn drop(&mut self) {
			cleanup_reactive_nodes();
			self.0.remove();
		}
	}

	fn assert_radio(
		input: &web_sys::HtmlInputElement,
		name: &str,
		id: &str,
		value: &str,
		checked: bool,
	) {
		assert_eq!(input.type_(), "radio");
		assert_eq!(input.name(), name);
		assert_eq!(input.id(), id);
		assert_eq!(input.value(), value);
		assert_eq!(input.checked(), checked);
	}

	fn assert_visible_radio(input: &web_sys::HtmlInputElement) {
		input.scroll_into_view();
		let width = web_sys::window()
			.unwrap()
			.inner_width()
			.unwrap()
			.as_f64()
			.unwrap();
		console_log!("RadioInput CSS viewport width: {width}");
		let visible = js_sys::Function::new_with_args(
			"input",
			"const r = input.getBoundingClientRect();
			return r.width > 0 && r.height > 0 && r.left >= 0 && r.right <= window.innerWidth
			    && document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) === input;",
		)
		.call1(&wasm_bindgen::JsValue::NULL, input.as_ref())
		.unwrap()
		.as_bool()
		.unwrap();
		assert!(
			visible,
			"radio must be visible and clickable within the {width}px viewport"
		);
	}

	#[rstest::rstest]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	async fn native_radio_reset_uses_updated_runtime_defaults() {
		// Arrange: defaults change after the DOM and a keyed row have mounted.
		let radio = form! {
			name: UpdatedRadioDefaults,
			fields: {
				answer: ChoiceField<String> { widget: RadioInput }
				name: CharField {
					initial: "initial"
				}
				answers: FieldArray {
					fields: {
						answer: ChoiceField<String> { widget: RadioInput }
					}
				}
			}
		};
		let runtime = use_form(&radio).build();
		let container = TestContainer::mount(radio.clone().into_page());
		let key = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
		let path = radio.answers_answer_path(key);
		runtime.set_value(radio.answer_field(), String::from("on"));
		runtime.set_value(radio.name_field(), String::from("saved"));
		runtime.set_path_value(path.clone(), String::from("on"));
		runtime.reset_default_values();
		runtime.set_value(radio.answer_field(), String::new());
		runtime.set_value(radio.name_field(), String::from("edited"));
		runtime.set_path_value(path.clone(), String::new());

		// Act
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;

		// Assert: native values and runtime dirtiness share the saved baseline.
		assert_eq!(runtime.get_values().answer, "on");
		assert_eq!(runtime.get_values().name, "saved");
		assert_eq!(runtime.get_values().answers[0].answer, "on");
		assert!(container.input("answer").checked());
		assert!(container.input("answers_0_answer").checked());
		assert_eq!(container.input("name").value(), "saved");
		assert!(!runtime.form_state().is_dirty.get());
		assert!(!runtime.get_path_state(path).is_dirty);
		assert_eq!(radio.answers().get()[0].key(), key);
	}

	#[rstest::rstest]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	async fn native_radio_reset_owns_password_synchronization() {
		// Arrange
		let radio = form! {
			name: PasswordRadioReset,
			fields: {
				answer: ChoiceField<String> { widget: RadioInput }
				password: CharField {
					widget: PasswordInput,
					initial: "initial"
				}
				name: CharField {
					initial: "saved"
				}
			}
		};
		let runtime = use_form(&radio).build();
		let container = TestContainer::mount(radio.clone().into_page());
		runtime.set_value(radio.answer_field(), String::from("on"));
		runtime.set_value(radio.password_field(), String::from("edited"));
		runtime.set_value(radio.name_field(), String::from("edited"));

		// Act
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;

		// Assert: the password's internal reset cannot supersede its sibling fields.
		assert_eq!(runtime.get_values().answer, "");
		assert_eq!(runtime.get_values().password, "initial");
		assert_eq!(runtime.get_values().name, "saved");
		assert_eq!(container.input("password").value(), "initial");
		assert!(!container.input("answer").checked());
		assert!(!runtime.form_state().is_dirty.get());
	}

	#[rstest::rstest]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	async fn native_radio_reset_stops_after_unmount_and_scope_disposal() {
		// Arrange: shared source state outlives the mounted form.
		let radio = form! {
			name: UnmountedRadioReset,
			fields: {
				answer: ChoiceField<String> { widget: RadioInput }
			}
		};
		radio.answer().set(String::from("on"));
		let container = TestContainer::mount(radio.clone().into_page());

		// Act: unmount while reset is pending, then retain the old DOM without its bindings.
		let retired_form = container.native_form();
		retired_form.reset();
		drop(container);
		let retained_dom = TestContainer::mount(Page::Empty);
		retained_dom.0.append_child(&retired_form).unwrap();
		assert!(retired_form.is_connected());
		gloo_timers::future::TimeoutFuture::new(0).await;

		// Assert: the disposed binding generation cannot overwrite retained state.
		assert_eq!(radio.answer().get(), "on");
		assert_eq!(radio.__native_reset_epoch.get(), 0);

		// A route scope may also dispose every source before the task resumes.
		let scope = reinhardt_pages::reactive::ReactiveScope::new();
		let scoped = scope.enter(|| {
			form! {
				name: DisposedRadioReset,
				fields: {
					answer: ChoiceField<String> { widget: RadioInput }
				}
			}
		});
		let answer = *scoped.answer();
		answer.set(String::from("on"));
		let container = TestContainer::mount(scoped.into_page());
		container.native_form().reset();
		drop(scope);
		assert!(container.native_form().is_connected());
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert!(answer.try_get_untracked().is_err());
	}

	#[rstest::rstest]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	fn radio_hydration_preserves_field_setters_and_adopts_unrelated_edits() {
		// Arrange: SSR is stale for two fields, while the third has a browser edit.
		let radio = form! {
			name: PreferredRadioHydration,
			fields: {
				answer: ChoiceField<String> {
					widget: RadioInput,
					initial: "on"
				}
				reset: ChoiceField<String> { widget: RadioInput }
				browser: ChoiceField<String> { widget: RadioInput }
			}
		};
		let runtime = use_form(&radio).build();
		radio.reset().set(String::from("on"));
		let page = radio.clone().into_page();
		let container = TestContainer::mount(Page::Empty);
		container.0.set_inner_html(&page.render_to_string());
		container.input("browser").set_checked(true);
		runtime.set_value(radio.answer_field(), String::new());
		runtime.reset_field(radio.reset_field());

		// Act
		container.hydrate(page);

		// Assert
		assert_eq!(radio.answer().get(), "");
		assert_eq!(radio.reset().get(), "");
		assert_eq!(radio.browser().get(), "on");
		assert!(!container.input("answer").checked());
		assert!(!container.input("reset").checked());
		assert!(container.input("browser").checked());
	}

	#[rstest::rstest]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	async fn native_radio_reset_clears_numeric_parse_errors() {
		// Arrange: a scalar radio selects the form-level reset path for numeric controls.
		let radio = form! {
			name: NumericRadioReset,
			fields: {
				answer: ChoiceField<String> { widget: RadioInput }
				amount: IntegerField { initial: 7 }
				ratio: FloatField { initial: 1.5 }
			}
		};
		let runtime = use_form(&radio).revalidate_on(RevalidateOn::Change).build();
		let container = TestContainer::mount(radio.clone().into_page());
		for changed_source in [false, true] {
			if changed_source {
				runtime.set_value(radio.amount_field(), 9_i64);
				runtime.set_value(radio.ratio_field(), 2.5);
			}
			for name in ["amount", "ratio"] {
				let input = container.input(name);
				input.set_value("");
				input
					.dispatch_event(&web_sys::Event::new("input").unwrap())
					.unwrap();
			}
			assert_eq!(runtime.trigger().unwrap_err().field_errors().len(), 2);

			// Act
			container.native_form().reset();
			gloo_timers::future::TimeoutFuture::new(0).await;

			// Assert: restored editors remain valid on revalidation and later source writes.
			assert_eq!(container.input("amount").value(), "7");
			assert_eq!(container.input("ratio").value(), "1.5");
			assert_eq!(runtime.get_values().amount, 7);
			assert_eq!(runtime.get_values().ratio, 1.5);
			assert_eq!(runtime.trigger(), Ok(()));
			radio.answer().set(String::from("on"));
			assert_eq!(runtime.get_field_state(radio.amount_field()).error, None);
			assert_eq!(runtime.get_field_state(radio.ratio_field()).error, None);
		}
	}

	#[rstest::rstest]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	async fn native_radio_reset_waits_for_a_task_and_honors_later_cancellation() {
		// Arrange
		let radio = form! {
			name: ResetTaskRadios,
			fields: {
				answer: ChoiceField<String> { widget: RadioInput }
			}
		};
		let container = TestContainer::mount(radio.clone().into_page());
		radio.answer().set(String::from("on"));

		// Act: allow ready microtasks to run, while the browser task is still pending.
		container.native_form().reset();
		wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
			&wasm_bindgen::JsValue::NULL,
		))
		.await
		.unwrap();

		// Assert: synchronization is deferred until a task, not just a microtask.
		assert_eq!(radio.answer().get(), "on");
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(radio.answer().get(), "");
		assert!(!container.input("answer").checked());

		// A subsequently installed reset listener can still cancel before synchronization.
		radio.answer().set(String::from("on"));
		let form_element = Element::new(container.native_form().into());
		let _cancel_reset = form_element.add_event_listener_with_event("reset", |event| {
			event.prevent_default();
		});
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(radio.answer().get(), "on");
		assert!(container.input("answer").checked());
	}

	#[wasm_bindgen_test]
	async fn radio_input_supports_label_selection_programmatic_changes_and_resets() {
		// Arrange
		let radio = form! {
			name: BrowserRadios,
			action: "/answer",
			method: Get,
			fields: {
				answer: ChoiceField<String> {
					widget: RadioInput,
					choices: [("yes", "Yes")],
				}
				selected: ChoiceField<String> {
					widget: RadioInput,
					label: "Initially selected",
					initial: "on",
				}
				reset: ResetButton {
					label: "Reset"
				}
			}
		};
		let runtime = use_form(&radio).revalidate_on(RevalidateOn::Change).build();
		let container = TestContainer::mount(radio.clone().into_page());
		let event_count = Rc::new(Cell::new(0));
		let observed_events = Rc::clone(&event_count);
		let _subscription =
			runtime.subscribe(move |_| observed_events.set(observed_events.get() + 1));
		assert_radio(&container.input("answer"), "answer", "answer", "yes", false);
		assert_visible_radio(&container.input("answer"));
		assert_radio(
			&container.input("selected"),
			"selected",
			"selected",
			"on",
			true,
		);
		let label = container
			.0
			.query_selector("label[for=\"answer\"]")
			.unwrap()
			.unwrap();
		assert_eq!(label.text_content().as_deref(), Some("Yes"));
		assert_eq!(
			label.get_attribute("for").as_deref(),
			Some(container.input("answer").id().as_str())
		);

		// Act and assert: the native label supplies both naming and selection.
		let document = web_sys::window().unwrap().document().unwrap();
		container.input("answer").focus().unwrap();
		assert_eq!(document.active_element().unwrap().id(), "answer");
		label.dyn_into::<web_sys::HtmlElement>().unwrap().click();
		assert_eq!(document.active_element().unwrap().id(), "answer");
		assert_eq!(runtime.get_values().answer, "yes");
		assert_radio(&container.input("answer"), "answer", "answer", "yes", true);
		assert_visible_radio(&container.input("answer"));
		assert!(container.input("selected").checked());

		// A change event from an unchecked radio must not select its option.
		runtime.set_value(radio.answer_field(), String::new());
		container
			.input("answer")
			.dispatch_event(&web_sys::Event::new("change").unwrap())
			.unwrap();
		assert_eq!(radio.answer().get(), "");
		assert!(!container.input("answer").checked());
		assert_eq!(document.active_element().unwrap().id(), "answer");

		radio.answer().set("yes".into());
		assert!(container.input("answer").checked());
		runtime.reset_field(radio.answer_field());
		assert_radio(&container.input("answer"), "answer", "answer", "yes", false);
		runtime.set_value(radio.selected_field(), String::new());
		assert!(!container.input("selected").checked());
		runtime.reset();
		assert!(container.input("selected").checked());

		container.input("answer").click();
		radio.selected().set(String::new());
		{
			let form_element = Element::new(container.native_form().into());
			let _cancel_reset = form_element.add_event_listener_with_event("reset", |event| {
				event.prevent_default();
			});
			container.native_form().reset();
			gloo_timers::future::TimeoutFuture::new(0).await;
			assert_eq!(runtime.get_values().answer, "yes");
			assert_eq!(runtime.get_values().selected, "");
			assert!(container.input("answer").checked());
			assert!(!container.input("selected").checked());
		}
		runtime.set_error(radio.answer_field(), FieldError::new("radio error"));
		event_count.set(0);
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.get_values().answer, "");
		assert_eq!(runtime.get_values().selected, "on");
		assert_eq!(runtime.watch().get().answer, "");
		assert!(!runtime.form_state().is_touched.get());
		assert!(!runtime.form_state().is_dirty.get());
		assert!(!runtime.get_field_state(radio.answer_field()).is_touched);
		assert_eq!(runtime.get_field_state(radio.answer_field()).error, None);
		assert_eq!(event_count.get(), 0);
		assert_radio(&container.input("answer"), "answer", "answer", "yes", false);
		assert_radio(
			&container.input("selected"),
			"selected",
			"selected",
			"on",
			true,
		);

		container.input("answer").click();
		container
			.0
			.query_selector("button[type=reset]")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlElement>()
			.unwrap()
			.click();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.get_values().answer, "");
		assert!(!container.input("answer").checked());
	}

	#[wasm_bindgen_test]
	async fn native_radio_reset_preserves_writes_after_dispatch() {
		// Arrange: a radio and ordinary controls share the pending reset operation.
		let radio = form! {
			name: DeferredResetRadios,
			fields: {
				answer: ChoiceField<String> {
					widget: RadioInput,
					choices: [("yes", "Yes")],
				}
				name: CharField {
					initial: "initial name"
				}
				enabled: BooleanField {}
			}
		};
		let runtime = use_form(&radio).revalidate_on(RevalidateOn::Change).build();
		let container = TestContainer::mount(radio.clone().into_page());
		radio.answer().set(String::from("other"));
		runtime.set_value(radio.name_field(), String::from("edited name"));
		runtime.set_value(radio.enabled_field(), true);

		// A synchronous write after reset() wins over its pending synchronization task.
		container.native_form().reset();
		runtime.set_value(radio.answer_field(), String::from("yes"));
		runtime.set_error(radio.answer_field(), FieldError::new("new error"));
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.watch().get().answer, "yes");
		assert!(container.input("answer").checked());
		assert_eq!(runtime.watch().get().name, "edited name");
		assert_eq!(container.input("name").value(), "edited name");
		assert!(runtime.get_values().enabled);
		assert!(container.input("enabled").checked());
		assert!(runtime.get_field_state(radio.answer_field()).is_touched);
		assert_eq!(
			runtime.get_field_state(radio.answer_field()).error,
			Some(FieldError::new("new error"))
		);

		// Later non-cancelling reset listeners can also replace the pending reset.
		radio.answer().set(String::from("other"));
		{
			let form_element = Element::new(container.native_form().into());
			let listener_runtime = runtime.clone();
			let field = radio.answer_field();
			let name_field = radio.name_field();
			let _later_reset = form_element.add_event_listener_with_event("reset", move |_| {
				listener_runtime.set_value(field, String::from("yes"));
				listener_runtime.set_value(name_field, String::from("listener name"));
				listener_runtime.set_error(field, FieldError::new("listener error"));
			});
			container.native_form().reset();
			gloo_timers::future::TimeoutFuture::new(0).await;
			assert_eq!(runtime.watch().get().answer, "yes");
			assert!(container.input("answer").checked());
			assert_eq!(runtime.watch().get().name, "listener name");
			assert_eq!(container.input("name").value(), "listener name");
			assert!(runtime.get_field_state(radio.answer_field()).is_touched);
			assert_eq!(
				runtime.get_field_state(radio.answer_field()).error,
				Some(FieldError::new("listener error"))
			);
		}

		// Equal-value writes still supersede a reset and retain newer validation state.
		container.native_form().reset();
		runtime.set_value(radio.answer_field(), String::from("yes"));
		runtime.set_error(radio.answer_field(), FieldError::new("same-value error"));
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.watch().get().answer, "yes");
		assert!(container.input("answer").checked());
		assert!(runtime.get_field_state(radio.answer_field()).is_touched);
		assert_eq!(
			runtime.get_field_state(radio.answer_field()).error,
			Some(FieldError::new("same-value error"))
		);

		// Completed guards are disposed, so the next ordinary reset can finish normally.
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.watch().get().answer, "");
		assert!(!container.input("answer").checked());
		assert_eq!(runtime.watch().get().name, "initial name");
		assert_eq!(container.input("name").value(), "initial name");
		assert!(!runtime.get_values().enabled);
		assert!(!container.input("enabled").checked());
		assert!(!runtime.form_state().is_touched.get());
		assert_eq!(runtime.get_field_state(radio.answer_field()).error, None);
	}

	#[wasm_bindgen_test]
	async fn native_radio_reset_restores_mixed_fields_and_collection_paths() {
		struct LoadedFields {
			answer: String,
			name: String,
			enabled: bool,
		}
		async fn load_fields() -> Result<LoadedFields, reinhardt_pages::ServerFnError> {
			Ok(LoadedFields {
				answer: String::from("yes"),
				name: String::from("loaded name"),
				enabled: false,
			})
		}

		// Arrange: scalar and collection controls share one native reset boundary.
		let radio = form! {
			name: MixedNativeReset,
			initial_loader: load_fields,
			fields: {
				answer: ChoiceField<String> {
					widget: RadioInput,
					choices: [("yes", "Yes")],
					initial_from: "answer",
				}
				name: CharField {
					initial: "initial name",
					initial_from: "name"
				}
				enabled: BooleanField {
					initial: true,
					initial_from: "enabled"
				}
				unbound: CharField { bind: false }
				answers: FieldArray {
					fields: {
						name: CharField {}
						enabled: BooleanField {}
						answer: ChoiceField<String> {
							widget: RadioInput,
							choices: [("yes", "Yes")],
						}
						unbound: CharField { bind: false }
					}
				}
			}
		};
		let runtime = use_form(&radio).revalidate_on(RevalidateOn::Change).build();
		let mut item = radio.new_answers_item();
		item.name = String::from("mounted row");
		item.enabled = true;
		item.answer = String::from("yes");
		let key = runtime.push_item(radio.answers_collection(), item);
		let mut second_item = radio.new_answers_item();
		second_item.name = String::from("second row");
		let second_key = runtime.push_item(radio.answers_collection(), second_item);
		let name_path = radio.answers_name_path(key);
		let watch_scope = reinhardt_pages::reactive::ReactiveScope::new();
		let name_watch = watch_scope.enter(|| runtime.watch_path::<String>(name_path.clone()));
		let container = TestContainer::mount(radio.clone().into_page());
		radio.unbound().set(String::from("unbound scalar kept"));
		runtime.set_path_value(
			radio.answers_unbound_path(key),
			String::from("unbound row kept"),
		);
		runtime.reset_default_values();
		let event_count = Rc::new(Cell::new(0));
		let observed_events = Rc::clone(&event_count);
		let _subscription =
			runtime.subscribe(move |_| observed_events.set(observed_events.get() + 1));

		// Act: reset edits to a row that has no persisted loader defaults yet.
		container.input("answer").click();
		container.input("name").set_value("edited name");
		container
			.input("name")
			.dispatch_event(&web_sys::Event::new("input").unwrap())
			.unwrap();
		runtime.set_value(radio.enabled_field(), false);
		runtime.set_path_value(name_path.clone(), String::from("edited row"));
		runtime.set_path_value(radio.answers_enabled_path(key), false);
		runtime.set_path_value(radio.answers_answer_path(key), String::new());
		runtime.set_error(radio.name_field(), FieldError::new("name error"));
		runtime.set_path_error(name_path.clone(), FieldError::new("row error"));
		event_count.set(0);
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;

		// Assert: values, DOM, and existing path handles all reflect their reset values.
		assert_eq!(runtime.get_values().name, "initial name");
		assert_eq!(container.input("name").value(), "initial name");
		assert!(runtime.get_values().enabled);
		assert!(container.input("enabled").checked());
		assert_eq!(runtime.get_values().answer, "");
		assert!(!container.input("answer").checked());
		assert_eq!(name_watch.get(), "mounted row");
		assert_eq!(container.input("answers_0_name").value(), "mounted row");
		assert!(runtime.get_values().answers[0].enabled);
		assert!(container.input("answers_0_enabled").checked());
		assert_eq!(runtime.get_values().answers[0].answer, "yes");
		assert!(container.input("answers_0_answer").checked());
		assert_eq!(radio.answers().get_untracked()[0].key(), key);
		assert_eq!(runtime.get_values().unbound, "unbound scalar kept");
		assert_eq!(runtime.get_values().answers[0].unbound, "unbound row kept");
		assert!(!runtime.form_state().is_dirty.get());
		assert!(!runtime.form_state().is_touched.get());
		assert_eq!(runtime.get_path_state(name_path.clone()).error, None);
		assert_eq!(runtime.form_state().error.get(), None);
		assert_eq!(event_count.get(), 0);

		// A later loader refresh persists current collection defaults as well as scalars.
		runtime.set_path_value(name_path.clone(), String::from("loaded row"));
		runtime.set_path_value(radio.answers_enabled_path(key), false);
		runtime.set_path_value(
			radio.answers_name_path(second_key),
			String::from("other loaded row"),
		);
		radio.load_initial_values().await.unwrap();
		assert_eq!(
			runtime.move_item(radio.answers_collection(), key, 1),
			Some((0, 1))
		);
		let name_watch = watch_scope.enter(|| runtime.watch_path::<String>(name_path.clone()));
		runtime.reset_default_values();
		runtime.set_value(radio.name_field(), String::from("changed again"));
		runtime.set_value(radio.enabled_field(), true);
		runtime.set_value(radio.answer_field(), String::new());
		runtime.set_path_value(name_path.clone(), String::from("changed row again"));
		runtime.set_path_value(
			radio.answers_name_path(second_key),
			String::from("other changed row"),
		);
		runtime.set_path_value(radio.answers_enabled_path(key), true);
		event_count.set(0);
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.watch().get().name, "loaded name");
		assert_eq!(container.input("name").value(), "loaded name");
		assert_eq!(runtime.get_values().answer, "yes");
		assert!(container.input("answer").checked());
		assert!(!runtime.get_values().enabled);
		assert!(!container.input("enabled").checked());
		assert_eq!(name_watch.get(), "loaded row");
		assert_eq!(container.input("answers_1_name").value(), "loaded row");
		assert_eq!(
			container.input("answers_0_name").value(),
			"other loaded row"
		);
		assert!(!runtime.get_values().answers[1].enabled);
		assert!(!container.input("answers_1_enabled").checked());
		assert_eq!(radio.answers().get_untracked()[1].key(), key);
		assert_eq!(radio.answers().get_untracked()[0].key(), second_key);
		assert!(!runtime.form_state().is_dirty.get());
		assert!(!runtime.form_state().is_touched.get());
		assert_eq!(event_count.get(), 0);
		runtime.set_path_value(name_path, String::from("observed after reset"));
		assert_eq!(name_watch.get(), "observed after reset");
		assert_eq!(
			container.input("answers_1_name").value(),
			"observed after reset"
		);
	}

	#[wasm_bindgen_test]
	async fn radio_input_native_reset_uses_defaults_loaded_after_mount() {
		struct LoadedAnswer {
			answer: String,
		}
		async fn load_answer() -> Result<LoadedAnswer, reinhardt_pages::ServerFnError> {
			Ok(LoadedAnswer {
				answer: String::from("yes"),
			})
		}

		// Arrange: create the page before the asynchronous default arrives.
		let radio = form! {
			name: LoadedRadios,
			initial_loader: load_answer,
			fields: {
				answer: ChoiceField<String> {
					widget: RadioInput,
					choices: [("yes", "Yes")],
					initial_from: "answer",
				}
			}
		};
		let container = TestContainer::mount(radio.clone().into_page());
		assert!(!container.input("answer").checked());
		radio.load_initial_values().await.unwrap();
		assert!(container.input("answer").checked());
		radio.answer().set(String::new());
		assert!(!container.input("answer").checked());

		// Act and assert: reset reads the refreshed initial storage at event time.
		container.native_form().reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(radio.answer().get(), "yes");
		assert!(container.input("answer").checked());
	}

	#[rstest::rstest]
	#[case::mounted(false)]
	#[case::hydrated(true)]
	#[test_attr(wasm_bindgen_test)]
	#[serial_test::serial(form_radio_input_dom)]
	fn radio_focus_replacement_is_scoped_to_new_roots_and_descendants(#[case] hydrated: bool) {
		// Arrange: a preceding form owns controls with matching IDs, names, and values.
		let unrelated = TestContainer::mount(
			PageElement::new("form")
				.child(
					PageElement::new("input")
						.attr("type", "radio")
						.attr("id", "answer")
						.attr("name", "answer")
						.attr("value", "yes"),
				)
				.child(
					PageElement::new("input")
						.attr("type", "radio")
						.attr("id", "answers_0_answer")
						.attr("name", "answers[0][answer]")
						.attr("value", "yes"),
				)
				.into_page(),
		);
		let radio = form! {
			name: FocusedRadios,
			fields: {
				answer: ChoiceField<String> {
					widget: RadioInput,
					choices: [("yes", "Yes")]
				}
				answers: FieldArray {
					fields: {
						answer: ChoiceField<String> {
							widget: RadioInput,
							choices: [("yes", "Yes")]
						}
					}
				}
			}
		};
		let runtime = use_form(&radio).build();
		let key = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
		let focus_source = radio.clone();
		let page = PageElement::new("div")
			.child(Page::reactive(move || {
				let _ = focus_source.answer().get();
				focus_source.clone().into_page()
			}))
			.into_page();
		let container = if hydrated {
			let container = TestContainer::mount(Page::Empty);
			container.0.set_inner_html(&page.render_to_string());
			container.hydrate(page);
			container
		} else {
			TestContainer::mount(page)
		};
		let document = web_sys::window().unwrap().document().unwrap();

		// Act and assert: both a root radio and a radio below a collection wrapper keep focus.
		let previous_scalar = container.input("answer");
		previous_scalar.focus().unwrap();
		radio.answer().set(String::from("yes"));
		assert!(
			document
				.active_element()
				.unwrap()
				.is_same_node(Some(&container.input("answer")))
		);
		assert!(!previous_scalar.is_same_node(Some(&container.input("answer"))));
		assert!(!unrelated.input("answer").checked());
		let previous_collection = container.input("answers_0_answer");
		previous_collection.focus().unwrap();
		runtime.set_path_value(radio.answers_answer_path(key), String::from("yes"));
		let _second = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
		assert!(
			document
				.active_element()
				.unwrap()
				.is_same_node(Some(&container.input("answers_0_answer")))
		);
		assert!(!previous_collection.is_same_node(Some(&container.input("answers_0_answer"))));
		assert!(!unrelated.input("answers_0_answer").checked());
	}

	#[wasm_bindgen_test]
	fn radio_input_honors_unbound_snapshot_and_disabled_options() {
		// Arrange
		let radio = form! {
			name: UnboundRadios,
			action: "/answer",
			method: Get,
			fields: {
				snapshot: ChoiceField<String> {
					widget: RadioInput,
					initial: "on",
					bind: false,
				}
				unavailable: ChoiceField<String> {
					widget: RadioInput,
					choices: [("yes", "Unavailable") { disabled }],
				}
			}
		};
		let container = TestContainer::mount(radio.clone().into_page());

		// Act and assert: neither direction is bound, while native checked state works.
		assert_radio(
			&container.input("snapshot"),
			"snapshot",
			"snapshot",
			"on",
			true,
		);
		radio.snapshot().set("other".into());
		assert!(container.input("snapshot").checked());
		container.input("snapshot").set_checked(false);
		container.input("snapshot").click();
		assert!(container.input("snapshot").checked());
		assert_eq!(radio.snapshot().get(), "other");
		assert!(container.input("unavailable").disabled());
		container.input("unavailable").click();
		assert_radio(
			&container.input("unavailable"),
			"unavailable",
			"unavailable",
			"yes",
			false,
		);
		assert_eq!(radio.unavailable().get(), "");
	}

	#[wasm_bindgen_test]
	async fn collection_radio_input_updates_only_its_item_and_resets() {
		// Arrange
		let radio = form! {
			name: BrowserCollectionRadios,
			action: "/answer",
			method: Get,
			fields: {
				answers: FieldArray {
					fields: {
						answer: ChoiceField<String> {
							widget: RadioInput,
							choices: [("yes", "Yes")],
						}
					}
				}
			}
		};
		let runtime = use_form(&radio).build();
		let first = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
		let second = runtime.push_item(radio.answers_collection(), radio.new_answers_item());
		runtime.reset_default_values();
		let container = TestContainer::mount(radio.clone().into_page());
		assert_eq!(
			container
				.0
				.query_selector_all("input[type=radio]")
				.unwrap()
				.length(),
			2
		);
		assert_radio(
			&container.input("answers_0_answer"),
			"answers[0][answer]",
			"answers_0_answer",
			"yes",
			false,
		);
		assert_radio(
			&container.input("answers_1_answer"),
			"answers[1][answer]",
			"answers_1_answer",
			"yes",
			false,
		);
		assert_eq!(
			container
				.0
				.query_selector("label[for=answers_0_answer]")
				.unwrap()
				.unwrap()
				.text_content()
				.as_deref(),
			Some("Yes")
		);

		// Act and assert: independent item names prevent browser radio grouping.
		let first_input = container.input("answers_0_answer");
		let second_input = container.input("answers_1_answer");
		first_input.click();
		assert_eq!(runtime.get_values().answers[0].answer, "yes");
		assert_eq!(runtime.get_values().answers[1].answer, "");
		runtime.set_path_value(radio.answers_answer_path(second), String::from("yes"));
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert!(container.input("answers_0_answer").checked());
		assert!(container.input("answers_1_answer").checked());
		runtime.set_path_value(radio.answers_answer_path(first), String::new());
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert!(!container.input("answers_0_answer").checked());
		assert!(container.input("answers_1_answer").checked());
		assert!(first_input.is_same_node(Some(&container.input("answers_0_answer"))));
		assert!(second_input.is_same_node(Some(&container.input("answers_1_answer"))));

		// Explicit runtime reset rebuilds the collection with its default keys.
		runtime.reset();
		gloo_timers::future::TimeoutFuture::new(0).await;
		assert_eq!(runtime.get_values().answers.len(), 2);
		assert_eq!(runtime.get_values().answers[0].answer, "");
		assert_eq!(runtime.get_values().answers[1].answer, "");
		assert!(!container.input("answers_0_answer").checked());
		assert!(!container.input("answers_1_answer").checked());
	}
}
