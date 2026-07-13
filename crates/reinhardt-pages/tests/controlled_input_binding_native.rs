#![cfg(all(native, feature = "testing"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use reinhardt_pages::component::{ControlBinding, ControlBindingError, NumberParseErrorKind};
use reinhardt_pages::event::{
	ChangeEvent, CompositionEndEvent, CompositionStartEvent, EventPayload,
};
use reinhardt_pages::prelude::spawn_task;
use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_layout_effect;
use reinhardt_pages::testing::component::{EventError, EventFixture, render};
use reinhardt_pages::{PageElement, page};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[tokio::test]
async fn bound_input_updates_before_explicit_handler() {
	// Arrange
	let value = Signal::new("old".to_owned());
	let observed = Arc::new(Mutex::new(String::new()));
	let observed_handler = Arc::clone(&observed);
	let value_handler = value.clone();
	let screen = render(page!({
		input {
			aria_label: "Name",
			bind: value,
			@input: move |_| *observed_handler.lock().unwrap() = value_handler.get(),
		}
	}));
	let input = screen.get_by_label("Name");

	// Act
	input.input("new");
	screen.settle().await;

	// Assert
	assert_eq!(value.get(), "new");
	assert_eq!(*observed.lock().unwrap(), "new");
	assert_eq!(input.value().as_deref(), Some("new"));
}

#[rstest]
#[serial(controlled_binding_effect)]
fn binding_write_layout_effect_can_read_the_same_screen() {
	// Arrange
	let value = Signal::new("old".to_owned());
	let screen = render(page!({
		input {
			aria_label: "Name",
			bind: value,
		}
	}));
	let input = screen.get_by_label("Name");
	let observed = Arc::new(Mutex::new(Vec::new()));
	let effect_input = input.clone();
	let effect_value = value.clone();
	let effect_observed = Arc::clone(&observed);
	let _effect = use_layout_effect(
		move || {
			effect_observed
				.lock()
				.unwrap()
				.push((effect_value.get(), effect_input.value()));
			None::<fn()>
		},
		(value.clone(),),
	);

	// Act
	input.input("new");

	// Assert
	assert_eq!(
		*observed.lock().unwrap(),
		vec![
			("old".to_owned(), Some("old".to_owned())),
			("new".to_owned(), Some("new".to_owned())),
		]
	);
}

#[rstest]
#[serial(controlled_binding_effect)]
#[tokio::test]
async fn binding_write_layout_effect_spawns_on_the_screen_scheduler() {
	// Arrange
	let value = Signal::new("old".to_owned());
	let screen = render(page!({
		input {
			aria_label: "Name",
			bind: value,
		}
	}));
	let input = screen.get_by_label("Name");
	let completed = Arc::new(AtomicBool::new(false));
	let effect_value = value.clone();
	let effect_completed = Arc::clone(&completed);
	let _effect = use_layout_effect(
		move || {
			if effect_value.get() == "new" {
				let completed = Arc::clone(&effect_completed);
				spawn_task(async move {
					completed.store(true, Ordering::SeqCst);
				});
			}
			None::<fn()>
		},
		(value.clone(),),
	);

	// Act
	input.input("new");
	screen.settle().await;

	// Assert
	assert!(completed.load(Ordering::SeqCst));
}

#[rstest]
#[tokio::test]
async fn checkbox_binding_tracks_fixture_and_external_signal_state() {
	// Arrange
	let checked = Signal::new(false);
	let observed = Arc::new(Mutex::new(None));
	let observed_handler = Arc::clone(&observed);
	let screen = render(page!({
		input {
			aria_label: "Enabled",
			type: "checkbox",
			bind: checked,
			@change: move |event: ChangeEvent| {
				*observed_handler.lock().unwrap() = Some(event.checked());
			},
		}
	}));
	let checkbox = screen.get_by_label("Enabled");

	// Act
	checkbox.change_checked(true);
	checked.set(false);
	screen.settle().await;
	checkbox
		.dispatch(EventFixture::change())
		.expect("refreshed checkbox should dispatch");

	// Assert
	assert!(!checked.get());
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&Ok::<bool, reinhardt_pages::event::EventTargetError>(false))
	);
}

#[rstest]
#[tokio::test]
async fn radio_binding_writes_only_the_checked_choice_and_refreshes_comparison() {
	// Arrange
	let selected = Signal::new("draft".to_owned());
	let screen = render(page!({
		input {
			aria_label: "Draft",
			type: "radio",
			value: "draft",
			bind: selected,
		}
		input {
			aria_label: "Published",
			type: "radio",
			value: "published",
			bind: selected,
		}
	}));
	let published = screen.get_by_label("Published");

	// Act
	published.change_checked(true);
	let selected_after_checked = selected.get();
	selected.set("draft".to_owned());
	screen.settle().await;
	published
		.dispatch(EventFixture::change())
		.expect("unchecked radio binding should ignore the change");

	// Assert
	assert_eq!(selected_after_checked, "published");
	assert_eq!(selected.get(), "draft");
}

#[rstest]
#[tokio::test]
async fn invalid_number_raw_survives_settle_until_the_value_signal_changes() {
	// Arrange
	let value = Signal::new(7_i32);
	let error = Signal::new(None);
	let screen = render(page!({
		input {
			aria_label: "Quantity",
			type: "number",
			bind: number(value, error),
		}
	}));
	let input = screen.get_by_label("Quantity");

	// Act
	input.input("-");
	screen.settle().await;
	let retained_raw = input.value();
	value.set(12);
	screen.settle().await;

	// Assert
	assert_eq!(value.get(), 12);
	assert_eq!(retained_raw.as_deref(), Some("-"));
	assert_eq!(input.value().as_deref(), Some("12"));
	let parse_error = error.get().expect("invalid number should set an error");
	assert_eq!(parse_error.raw(), "-");
	assert_eq!(parse_error.kind(), NumberParseErrorKind::Incomplete);
}

#[rstest]
#[tokio::test]
async fn select_one_binding_tracks_selected_value_in_both_directions() {
	// Arrange
	let selected = Signal::new("rust".to_owned());
	let observed = Arc::new(Mutex::new(None));
	let observed_handler = Arc::clone(&observed);
	let screen = render(page!({
		select {
			aria_label: "Language",
			bind: selected,
			@change: move |event: ChangeEvent| {
				*observed_handler.lock().unwrap() = Some(event.selected_values());
			},
			option {
				value: "rust",
				"Rust"
			}
			option {
				value: "wasm",
				"WebAssembly"
			}
		}
	}));
	let select = screen.get_by_label("Language");

	// Act
	select
		.dispatch(EventFixture::change().selected_values(["wasm"]))
		.expect("select-one fixture should dispatch");
	let wasm_dom = screen.pretty();
	selected.set("rust".to_owned());
	screen.settle().await;
	select
		.dispatch(EventFixture::change())
		.expect("refreshed select-one should dispatch");

	// Assert
	assert_eq!(selected.get(), "rust");
	assert_eq!(select.value().as_deref(), Some("rust"));
	assert_eq!(
		wasm_dom,
		concat!(
			"<select aria-label=\"Language\">\n",
			"  <option value=\"rust\">\n",
			"    Rust\n",
			"  </option>\n",
			"  <option value=\"wasm\" selected=\"selected\">\n",
			"    WebAssembly\n",
			"  </option>\n",
			"</select>\n",
		)
	);
	assert_eq!(
		screen.pretty(),
		concat!(
			"<select aria-label=\"Language\">\n",
			"  <option value=\"rust\" selected=\"selected\">\n",
			"    Rust\n",
			"  </option>\n",
			"  <option value=\"wasm\">\n",
			"    WebAssembly\n",
			"  </option>\n",
			"</select>\n",
		)
	);
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&Ok::<Vec<String>, reinhardt_pages::event::EventTargetError>(vec!["rust".to_owned()]))
	);
}

#[rstest]
fn select_one_empty_selection_commits_the_browser_empty_value() {
	// Arrange
	let selected = Signal::new("rust".to_owned());
	let screen = render(page!({
		select {
			aria_label: "Language",
			bind: selected,
			option {
				value: "rust",
				"Rust"
			}
		}
	}));
	let select = screen.get_by_label("Language");

	// Act
	select
		.dispatch(EventFixture::change().selected_values(Vec::<String>::new()))
		.expect("empty select-one fixture should dispatch");

	// Assert
	assert_eq!(selected.get(), "");
	assert_eq!(select.value().as_deref(), Some(""));
}

#[rstest]
#[tokio::test]
async fn select_many_binding_tracks_all_selected_values_in_both_directions() {
	// Arrange
	let selected = Signal::new(vec!["rust".to_owned()]);
	let observed = Arc::new(Mutex::new(None));
	let observed_handler = Arc::clone(&observed);
	let screen = render(page!({
		select {
			aria_label: "Targets",
			multiple: true,
			bind: selected,
			@change: move |event: ChangeEvent| {
				*observed_handler.lock().unwrap() = Some(event.selected_values());
			},
			option {
				value: "rust",
				"Rust"
			}
			option {
				value: "wasm",
				"WebAssembly"
			}
		}
	}));
	let select = screen.get_by_label("Targets");

	// Act
	select
		.dispatch(EventFixture::change().selected_values(["rust", "wasm"]))
		.expect("select-many fixture should dispatch");
	let both_selected_dom = screen.pretty();
	selected.set(vec!["wasm".to_owned()]);
	screen.settle().await;
	select
		.dispatch(EventFixture::change())
		.expect("refreshed select-many should dispatch");

	// Assert
	assert_eq!(selected.get(), vec!["wasm".to_owned()]);
	assert_eq!(select.value().as_deref(), Some("wasm"));
	assert_eq!(
		both_selected_dom,
		concat!(
			"<select aria-label=\"Targets\" multiple=\"multiple\">\n",
			"  <option value=\"rust\" selected=\"selected\">\n",
			"    Rust\n",
			"  </option>\n",
			"  <option value=\"wasm\" selected=\"selected\">\n",
			"    WebAssembly\n",
			"  </option>\n",
			"</select>\n",
		)
	);
	assert_eq!(
		screen.pretty(),
		concat!(
			"<select aria-label=\"Targets\" multiple=\"multiple\">\n",
			"  <option value=\"rust\">\n",
			"    Rust\n",
			"  </option>\n",
			"  <option value=\"wasm\" selected=\"selected\">\n",
			"    WebAssembly\n",
			"  </option>\n",
			"</select>\n",
		)
	);
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&Ok::<Vec<String>, reinhardt_pages::event::EventTargetError>(vec!["wasm".to_owned()]))
	);
}

#[rstest]
#[tokio::test]
async fn composition_defers_writes_and_deduplicates_the_final_input() {
	// Arrange
	let value = Signal::new("old".to_owned());
	let observed = Arc::new(Mutex::new(Vec::new()));
	let observed_input = Arc::clone(&observed);
	let input_value = value.clone();
	let observed_end = Arc::clone(&observed);
	let end_value = value.clone();
	let screen = render(page!({
		input {
			aria_label: "Name",
			bind: value,
			@input: move |_| observed_input.lock().unwrap().push(input_value.get()),
			@compositionend: move |_| {
				observed_end.lock().unwrap().push(end_value.get());
				end_value.set("after-end".to_owned());
			},
		}
	}));
	let input = screen.get_by_label("Name");

	// Act
	input
		.dispatch(EventFixture::new(CompositionStartEvent::EVENT))
		.expect("composition start should dispatch");
	input
		.dispatch(EventFixture::input().value("k").is_composing(true))
		.expect("first composing input should dispatch");
	input
		.dispatch(EventFixture::input().value("かな").is_composing(true))
		.expect("second composing input should dispatch");
	input
		.dispatch(EventFixture::new(CompositionEndEvent::EVENT).value("かな"))
		.expect("composition end should dispatch");
	input
		.dispatch(EventFixture::input().value("かな"))
		.expect("duplicate final input should dispatch");

	// Assert
	assert_eq!(
		*observed.lock().unwrap(),
		vec![
			"old".to_owned(),
			"old".to_owned(),
			"かな".to_owned(),
			"after-end".to_owned(),
		]
	);
	assert_eq!(value.get(), "after-end");
}

#[rstest]
fn binding_failures_are_structured_event_errors() {
	// Arrange
	let selected = Signal::new(Vec::<String>::new());
	let screen = render(
		PageElement::new("input")
			.attr("aria-label", "Invalid")
			.control_binding(ControlBinding::select_many(selected)),
	);
	let input = screen.get_by_label("Invalid");

	// Act
	let error = input
		.dispatch(EventFixture::change().value("unexpected"))
		.expect_err("invalid binding target should fail");

	// Assert
	assert_eq!(
		error,
		EventError::ControlBinding(ControlBindingError::UnsupportedElement {
			control: reinhardt_pages::component::ControlKind::SelectMany,
			actual_tag: "input".to_owned(),
		})
	);
}
