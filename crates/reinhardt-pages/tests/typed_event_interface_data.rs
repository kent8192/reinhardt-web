#![cfg(feature = "testing")]

use reinhardt_pages::event::{
	AnimationStartEvent, BeforeXrSelectEvent, BeginEvent, CommandEvent, CompositionStartEvent,
	CopyEvent, DragStartEvent, EncryptedEvent, EnterPictureInPictureEvent, EventPayload,
	FocusInEvent, KnownEvent, Modifiers, Point, SecurityPolicyViolationDetails,
	SecurityPolicyViolationEvent, SubmitEvent, ToggleEvent, TouchPoint, TouchStartEvent,
	TransitionStartEvent, WheelEvent, XrInputSourceDescriptor,
};
use reinhardt_pages::testing::component::EventFixture;

#[test]
fn animation_clipboard_command_composition_and_focus_data_round_trip() {
	let animation = AnimationStartEvent::try_from_raw(
		EventFixture::new(KnownEvent::AnimationStart)
			.animation("fade-in", 1.25, "::before")
			.build()
			.expect("animation fixture must build"),
	)
	.expect("animation payload must convert");
	assert_eq!(animation.animation_name(), "fade-in");
	assert_eq!(animation.elapsed_time(), 1.25);
	assert_eq!(animation.pseudo_element(), "::before");

	let clipboard = CopyEvent::try_from_raw(
		EventFixture::new(KnownEvent::Copy)
			.clipboard_text("copied text")
			.build()
			.expect("clipboard fixture must build"),
	)
	.expect("clipboard payload must convert");
	assert_eq!(clipboard.text().as_deref(), Some("copied text"));

	let command = CommandEvent::try_from_raw(
		EventFixture::new(KnownEvent::Command)
			.command("show-modal")
			.command_source("button")
			.build()
			.expect("command fixture must build"),
	)
	.expect("command payload must convert");
	assert_eq!(command.command(), "show-modal");
	assert_eq!(
		command.source().expect("command source").tag_name(),
		"button"
	);

	let composition = CompositionStartEvent::try_from_raw(
		EventFixture::new(KnownEvent::CompositionStart)
			.composition_data("あ")
			.build()
			.expect("composition fixture must build"),
	)
	.expect("composition payload must convert");
	assert_eq!(composition.data(), "あ");

	let focus = FocusInEvent::try_from_raw(
		EventFixture::new(KnownEvent::FocusIn)
			.related_target("input")
			.build()
			.expect("focus fixture must build"),
	)
	.expect("focus payload must convert");
	assert_eq!(
		focus.related_target().expect("related target").tag_name(),
		"input"
	);
}

#[test]
fn encrypted_picture_in_picture_security_submit_time_and_toggle_data_round_trip() {
	let encrypted = EncryptedEvent::try_from_raw(
		EventFixture::new(KnownEvent::Encrypted)
			.encrypted_data("cenc", [1_u8, 2, 3])
			.build()
			.expect("encrypted fixture must build"),
	)
	.expect("encrypted payload must convert");
	assert_eq!(encrypted.init_data_type(), "cenc");
	assert_eq!(encrypted.init_data(), vec![1, 2, 3]);

	let picture_in_picture = EnterPictureInPictureEvent::try_from_raw(
		EventFixture::new(KnownEvent::EnterPictureInPicture)
			.picture_in_picture_size(640, 360)
			.build()
			.expect("picture-in-picture fixture must build"),
	)
	.expect("picture-in-picture payload must convert");
	assert_eq!(picture_in_picture.width(), 640);
	assert_eq!(picture_in_picture.height(), 360);

	let details = SecurityPolicyViolationDetails {
		blocked_uri: "https://blocked.example/script.js".to_owned(),
		column_number: 7,
		disposition: "enforce".to_owned(),
		document_uri: "https://example.test/".to_owned(),
		effective_directive: "script-src".to_owned(),
		line_number: 11,
		original_policy: "script-src 'self'".to_owned(),
		referrer: "https://referrer.example/".to_owned(),
		sample: "alert(1)".to_owned(),
		source_file: "app.js".to_owned(),
		status_code: 200,
		violated_directive: "script-src".to_owned(),
	};
	let security = SecurityPolicyViolationEvent::try_from_raw(
		EventFixture::new(KnownEvent::SecurityPolicyViolation)
			.security_policy_violation(details.clone())
			.build()
			.expect("security fixture must build"),
	)
	.expect("security payload must convert");
	assert_eq!(security.details(), details);

	let submit = SubmitEvent::try_from_raw(
		EventFixture::new(KnownEvent::Submit)
			.submitter("button")
			.build()
			.expect("submit fixture must build"),
	)
	.expect("submit payload must convert");
	assert_eq!(
		submit.submitter().expect("submitter target").tag_name(),
		"button"
	);

	let time = BeginEvent::try_from_raw(
		EventFixture::new(KnownEvent::BeginEvent)
			.time_detail(4)
			.build()
			.expect("time fixture must build"),
	)
	.expect("time payload must convert");
	assert_eq!(time.detail(), 4);

	let toggle = ToggleEvent::try_from_raw(
		EventFixture::new(KnownEvent::Toggle)
			.toggle_states("closed", "open")
			.build()
			.expect("toggle fixture must build"),
	)
	.expect("toggle payload must convert");
	assert_eq!(toggle.old_state(), "closed");
	assert_eq!(toggle.new_state(), "open");
}

#[test]
fn touch_transition_wheel_drag_and_xr_data_round_trip() {
	let touch = TouchPoint::new(
		9,
		Point::new(10.0, 11.0),
		Point::new(12.0, 13.0),
		Point::new(14.0, 15.0),
		Point::new(2.0, 3.0),
		45.0,
		0.75,
	);
	let modifiers = Modifiers {
		alt: true,
		control: false,
		meta: true,
		shift: false,
	};
	let touch_event = TouchStartEvent::try_from_raw(
		EventFixture::new(KnownEvent::TouchStart)
			.touches([touch.clone()])
			.target_touches([touch.clone()])
			.changed_touches([touch.clone()])
			.modifiers(modifiers)
			.build()
			.expect("touch fixture must build"),
	)
	.expect("touch payload must convert");
	assert_eq!(touch_event.touches(), vec![touch.clone()]);
	assert_eq!(touch_event.target_touches(), vec![touch.clone()]);
	assert_eq!(touch_event.changed_touches(), vec![touch]);
	assert_eq!(touch_event.modifiers(), modifiers);

	let transition = TransitionStartEvent::try_from_raw(
		EventFixture::new(KnownEvent::TransitionStart)
			.transition("opacity", 0.5, "::after")
			.build()
			.expect("transition fixture must build"),
	)
	.expect("transition payload must convert");
	assert_eq!(transition.property_name(), "opacity");
	assert_eq!(transition.elapsed_time(), 0.5);
	assert_eq!(transition.pseudo_element(), "::after");

	let wheel = WheelEvent::try_from_raw(
		EventFixture::new(KnownEvent::Wheel)
			.wheel_delta(1.0, 2.0, 3.0, 1)
			.build()
			.expect("wheel fixture must build"),
	)
	.expect("wheel payload must convert");
	assert_eq!(wheel.delta_x(), 1.0);
	assert_eq!(wheel.delta_y(), 2.0);
	assert_eq!(wheel.delta_z(), 3.0);
	assert_eq!(wheel.delta_mode(), 1);

	let drag = DragStartEvent::try_from_raw(
		EventFixture::new(KnownEvent::DragStart)
			.drag_data("text/plain payload")
			.build()
			.expect("drag fixture must build"),
	)
	.expect("drag payload must convert");
	assert_eq!(drag.data().as_deref(), Some("text/plain payload"));

	let descriptor = XrInputSourceDescriptor {
		handedness: "left".to_owned(),
		target_ray_mode: "tracked-pointer".to_owned(),
		profiles: vec!["generic-trigger".to_owned()],
	};
	let xr = BeforeXrSelectEvent::try_from_raw(
		EventFixture::new(KnownEvent::BeforeXrSelect)
			.xr_input_source(descriptor.clone())
			.build()
			.expect("XR fixture must build"),
	)
	.expect("XR payload must convert");
	assert_eq!(xr.input_source(), descriptor);
}
