#![cfg(all(native, feature = "testing"))]

use std::sync::{Arc, Mutex};

use reinhardt_pages::event::{
	AbortEvent, AnimationEndEvent, CompositionUpdateEvent, CopyEvent, EncryptedEvent,
	EventInterface, EventPayload, TransitionEndEvent,
};
use reinhardt_pages::page;
use reinhardt_pages::testing::component::{EventFixture, render};
use rstest::rstest;

#[rstest]
fn page_clipboard_handler_receives_fixture_text() {
	// Arrange
	let observed = Arc::new(Mutex::new(None));
	let screen = render(page!(|observed: Arc<Mutex<Option<Option<String>>>>| {
		div {
			@copy: move |event: CopyEvent| {
				*observed.lock().unwrap() = Some(event.text());
			},
			"Clipboard target"
		}
	})(Arc::clone(&observed)));

	// Act
	screen
		.get_by_text("Clipboard target")
		.dispatch(EventFixture::new(CopyEvent::EVENT).clipboard_text("copied text"))
		.expect("clipboard fixture should dispatch");

	// Assert
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&Some("copied text".to_string()))
	);
}

#[rstest]
fn page_composition_handler_receives_fixture_data() {
	// Arrange
	let observed = Arc::new(Mutex::new(None));
	let screen = render(page!(|observed: Arc<Mutex<Option<String>>>| {
		div {
			@compositionupdate: move |event: CompositionUpdateEvent| {
				*observed.lock().unwrap() = Some(event.data());
			},
			"Composition target"
		}
	})(Arc::clone(&observed)));

	// Act
	screen
		.get_by_text("Composition target")
		.dispatch(EventFixture::new(CompositionUpdateEvent::EVENT).composition_data("かな"))
		.expect("composition fixture should dispatch");

	// Assert
	assert_eq!(observed.lock().unwrap().as_deref(), Some("かな"));
}

#[rstest]
fn page_animation_handler_receives_fixture_data() {
	// Arrange
	let observed = Arc::new(Mutex::new(None));
	let screen = render(
		page!(|observed: Arc<Mutex<Option<(String, f64, String)>>>| {
			div {
				@animationend: move |event: AnimationEndEvent| {
					*observed.lock().unwrap() = Some((
						event.animation_name(),
						event.elapsed_time(),
						event.pseudo_element(),
					));
				},
				"Animation target"
			}
		})(Arc::clone(&observed)),
	);

	// Act
	screen
		.get_by_text("Animation target")
		.dispatch(
			EventFixture::new(AnimationEndEvent::EVENT).animation("fade-out", 1.75, "::after"),
		)
		.expect("animation fixture should dispatch");

	// Assert
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&("fade-out".to_string(), 1.75, "::after".to_string()))
	);
}

#[rstest]
fn page_transition_handler_receives_fixture_data() {
	// Arrange
	let observed = Arc::new(Mutex::new(None));
	let screen = render(
		page!(|observed: Arc<Mutex<Option<(String, f64, String)>>>| {
			div {
				@transitionend: move |event: TransitionEndEvent| {
					*observed.lock().unwrap() = Some((
						event.property_name(),
						event.elapsed_time(),
						event.pseudo_element(),
					));
				},
				"Transition target"
			}
		})(Arc::clone(&observed)),
	);

	// Act
	screen
		.get_by_text("Transition target")
		.dispatch(
			EventFixture::new(TransitionEndEvent::EVENT).transition("opacity", 0.75, "::before"),
		)
		.expect("transition fixture should dispatch");

	// Assert
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&("opacity".to_string(), 0.75, "::before".to_string()))
	);
}

#[rstest]
fn page_encrypted_media_handler_receives_fixture_data() {
	// Arrange
	let observed = Arc::new(Mutex::new(None));
	let screen = render(page!(|observed: Arc<Mutex<Option<(String, Vec<u8>)>>>| {
		video {
			@encrypted: move |event: EncryptedEvent| {
				*observed.lock().unwrap() = Some((event.init_data_type(), event.init_data()));
			},
			"Media target"
		}
	})(Arc::clone(&observed)));

	// Act
	screen
		.get_by_text("Media target")
		.dispatch(
			EventFixture::new(EncryptedEvent::EVENT).encrypted_data("cenc", [0x01_u8, 0x02, 0xfe]),
		)
		.expect("encrypted-media fixture should dispatch");

	// Assert
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&("cenc".to_string(), vec![0x01, 0x02, 0xfe]))
	);
}

#[rstest]
fn page_generic_handler_receives_fixture_interface() {
	// Arrange
	let observed = Arc::new(Mutex::new(None));
	let screen = render(
		page!(|observed: Arc<Mutex<Option<(String, EventInterface)>>>| {
			div {
				@abort: move |event: AbortEvent| {
					*observed.lock().unwrap() = Some((
						event.event_type().to_string(),
						event.raw().payload().interface(),
					));
				},
				"Generic target"
			}
		})(Arc::clone(&observed)),
	);

	// Act
	screen
		.get_by_text("Generic target")
		.dispatch(EventFixture::new(AbortEvent::EVENT))
		.expect("generic fixture should dispatch");

	// Assert
	assert_eq!(
		observed.lock().unwrap().as_ref(),
		Some(&("abort".to_string(), EventInterface::Generic))
	);
}
