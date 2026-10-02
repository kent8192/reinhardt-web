//! Catalog-generated event-specific payload wrappers.

use reinhardt_event_catalog::{EventInterface, KnownEvent};

#[cfg(wasm)]
use wasm_bindgen::JsCast;

use super::{
	EventConversionError, EventFile, EventPayload, EventTarget, EventTargetError, Modifiers,
	MouseButton, MouseButtons, Point, PointerKind,
};
use crate::platform;

/// Owned details captured from a Content Security Policy violation event.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityPolicyViolationDetails {
	/// URI blocked by the policy.
	pub blocked_uri: String,
	/// Source column number.
	pub column_number: u32,
	/// Policy disposition.
	pub disposition: String,
	/// URI of the document where the violation occurred.
	pub document_uri: String,
	/// Effective directive that was violated.
	pub effective_directive: String,
	/// Source line number.
	pub line_number: u32,
	/// Full policy text.
	pub original_policy: String,
	/// Referrer associated with the violation.
	pub referrer: String,
	/// Source sample associated with the violation.
	pub sample: String,
	/// Source file associated with the violation.
	pub source_file: String,
	/// HTTP status code associated with the protected resource.
	pub status_code: u16,
	/// Original directive that was violated.
	pub violated_directive: String,
}

/// Owned contact point captured from a touch event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TouchPoint {
	/// Stable touch identifier.
	pub identifier: i32,
	/// Viewport-relative position.
	pub client_position: Point,
	/// Screen-relative position.
	pub screen_position: Point,
	/// Document-relative position.
	pub page_position: Point,
	/// Contact radii.
	pub radius: Point,
	/// Contact rotation angle.
	pub rotation_angle: f64,
	/// Normalized contact force.
	pub force: f32,
}

impl TouchPoint {
	/// Creates an owned touch-point snapshot.
	#[must_use]
	pub const fn new(
		identifier: i32,
		client_position: Point,
		screen_position: Point,
		page_position: Point,
		radius: Point,
		rotation_angle: f64,
		force: f32,
	) -> Self {
		Self {
			identifier,
			client_position,
			screen_position,
			page_position,
			radius,
			rotation_angle,
			force,
		}
	}
}

/// Owned, cross-target WebXR input-source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XrInputSourceDescriptor {
	/// Input-side classification such as `left`, `right`, or `none`.
	pub handedness: String,
	/// Target-ray mode such as `tracked-pointer` or `gaze`.
	pub target_ray_mode: String,
	/// Ordered WebXR interaction profiles.
	pub profiles: Vec<String>,
}

#[cfg(native)]
fn native_target(
	target: &Option<reinhardt_core::types::page::NativeEventTarget>,
) -> Option<EventTarget> {
	target.as_ref().map(EventTarget::from_native)
}

#[cfg(wasm)]
fn web_target(target: Option<web_sys::EventTarget>) -> Option<EventTarget> {
	target.and_then(EventTarget::from_web_target)
}

#[cfg(wasm)]
fn reflect_string(value: &wasm_bindgen::JsValue, property: &str) -> String {
	js_sys::Reflect::get(value, &wasm_bindgen::JsValue::from_str(property))
		.ok()
		.and_then(|value| value.as_string())
		.unwrap_or_default()
}

#[cfg(wasm)]
fn touch_points(list: web_sys::TouchList) -> Vec<TouchPoint> {
	(0..list.length())
		.filter_map(|index| list.item(index))
		.map(|touch| {
			TouchPoint::new(
				touch.identifier(),
				Point::new(f64::from(touch.client_x()), f64::from(touch.client_y())),
				Point::new(f64::from(touch.screen_x()), f64::from(touch.screen_y())),
				Point::new(f64::from(touch.page_x()), f64::from(touch.page_y())),
				Point::new(f64::from(touch.radius_x()), f64::from(touch.radius_y())),
				f64::from(touch.rotation_angle()),
				touch.force(),
			)
		})
		.collect()
}

#[derive(Clone)]
struct PayloadCore {
	raw: platform::Event,
	target: Option<EventTarget>,
	current_target: Option<EventTarget>,
}

impl std::fmt::Debug for PayloadCore {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter
			.debug_struct("PayloadCore")
			.field("event_type", &platform::event_type(&self.raw))
			.field("target", &self.target)
			.field("current_target", &self.current_target)
			.finish()
	}
}

fn current_target_required<'a>(
	current_target: &'a Option<EventTarget>,
	event: &'static str,
) -> Result<&'a EventTarget, EventTargetError> {
	current_target
		.as_ref()
		.ok_or(EventTargetError::MissingCurrentTarget { event })
}

macro_rules! capability_methods {
	($payload:ident, Value) => {
		impl $payload {
			/// Reads the listener target's control value or contenteditable text.
			pub fn value(&self) -> Result<String, EventTargetError> {
				current_target_required(&self.core.current_target, Self::EVENT.as_str())?
					.value_for(Self::EVENT.as_str())
			}
		}
	};
	($payload:ident, Checked) => {
		impl $payload {
			/// Reads the listener target's checkbox or radio state.
			pub fn checked(&self) -> Result<bool, EventTargetError> {
				current_target_required(&self.core.current_target, Self::EVENT.as_str())?
					.checked_for(Self::EVENT.as_str())
			}
		}
	};
	($payload:ident, SelectedValues) => {
		impl $payload {
			/// Reads every selected value from the listener select element.
			pub fn selected_values(&self) -> Result<Vec<String>, EventTargetError> {
				current_target_required(&self.core.current_target, Self::EVENT.as_str())?
					.selected_values_for(Self::EVENT.as_str())
			}
		}
	};
	($payload:ident, Files) => {
		impl $payload {
			/// Reads owned file metadata from the listener file input.
			pub fn files(&self) -> Result<Vec<EventFile>, EventTargetError> {
				current_target_required(&self.core.current_target, Self::EVENT.as_str())?
					.files_for(Self::EVENT.as_str())
			}
		}
	};
}

macro_rules! mouse_methods {
	($payload:ident) => {
		impl $payload {
			/// Returns the viewport-relative pointer position.
			#[must_use]
			pub fn client_position(&self) -> Point {
				let data = platform::mouse_snapshot(&self.core.raw);
				Point::new(data.client_x, data.client_y)
			}

			/// Returns the screen-relative pointer position.
			#[must_use]
			pub fn screen_position(&self) -> Point {
				let data = platform::mouse_snapshot(&self.core.raw);
				Point::new(data.screen_x, data.screen_y)
			}

			/// Returns the document-relative pointer position.
			#[must_use]
			pub fn page_position(&self) -> Point {
				let data = platform::mouse_snapshot(&self.core.raw);
				Point::new(data.page_x, data.page_y)
			}

			/// Returns the listener-relative pointer position.
			#[must_use]
			pub fn offset_position(&self) -> Point {
				let data = platform::mouse_snapshot(&self.core.raw);
				Point::new(data.offset_x, data.offset_y)
			}

			/// Returns the button whose state changed.
			#[must_use]
			pub fn button(&self) -> MouseButton {
				platform::mouse_snapshot(&self.core.raw).button.into()
			}

			/// Returns the buttons pressed during dispatch.
			#[must_use]
			pub fn buttons(&self) -> MouseButtons {
				MouseButtons::from_bits(platform::mouse_snapshot(&self.core.raw).buttons)
			}

			/// Returns the click count or interface detail.
			#[must_use]
			pub fn detail(&self) -> i32 {
				platform::mouse_snapshot(&self.core.raw).detail
			}

			/// Returns keyboard modifiers active during dispatch.
			#[must_use]
			pub fn modifiers(&self) -> Modifiers {
				let data = platform::mouse_snapshot(&self.core.raw);
				Modifiers {
					alt: data.alt,
					control: data.control,
					meta: data.meta,
					shift: data.shift,
				}
			}
		}
	};
}

#[cfg(native)]
fn animation_values(event: &platform::Event) -> (String, f64, String) {
	let reinhardt_core::types::page::NativeEventPayload::Animation(data) = event.payload() else {
		unreachable!("validated animation event changed interface")
	};
	(
		data.animation_name.clone(),
		data.elapsed_time,
		data.pseudo_element.clone(),
	)
}

#[cfg(wasm)]
fn animation_values(event: &platform::Event) -> (String, f64, String) {
	let event = event
		.dyn_ref::<web_sys::AnimationEvent>()
		.expect("validated animation event changed interface");
	(
		event.animation_name(),
		f64::from(event.elapsed_time()),
		event.pseudo_element(),
	)
}

#[cfg(native)]
fn clipboard_text(event: &platform::Event) -> Option<String> {
	let reinhardt_core::types::page::NativeEventPayload::Clipboard(data) = event.payload() else {
		unreachable!("validated clipboard event changed interface")
	};
	data.text.clone()
}

#[cfg(wasm)]
fn clipboard_text(event: &platform::Event) -> Option<String> {
	event
		.dyn_ref::<web_sys::ClipboardEvent>()
		.expect("validated clipboard event changed interface")
		.clipboard_data()
		.and_then(|data| data.get_data("text/plain").ok())
}

#[cfg(native)]
fn command_values(event: &platform::Event) -> (String, Option<EventTarget>) {
	let reinhardt_core::types::page::NativeEventPayload::Command(data) = event.payload() else {
		unreachable!("validated command event changed interface")
	};
	(data.command.clone(), native_target(&data.source))
}

#[cfg(wasm)]
fn command_values(event: &platform::Event) -> (String, Option<EventTarget>) {
	let event = event
		.dyn_ref::<web_sys::CommandEvent>()
		.expect("validated command event changed interface");
	let source = event
		.source()
		.map(|element| element.unchecked_into::<web_sys::EventTarget>());
	(event.command(), web_target(source))
}

#[cfg(native)]
fn composition_data(event: &platform::Event) -> String {
	let reinhardt_core::types::page::NativeEventPayload::Composition(data) = event.payload() else {
		unreachable!("validated composition event changed interface")
	};
	data.data.clone()
}

#[cfg(wasm)]
fn composition_data(event: &platform::Event) -> String {
	event
		.dyn_ref::<web_sys::CompositionEvent>()
		.expect("validated composition event changed interface")
		.data()
		.unwrap_or_default()
}

#[cfg(native)]
fn focus_related_target(event: &platform::Event) -> Option<EventTarget> {
	let reinhardt_core::types::page::NativeEventPayload::Focus(data) = event.payload() else {
		unreachable!("validated focus event changed interface")
	};
	native_target(&data.related_target)
}

#[cfg(wasm)]
fn focus_related_target(event: &platform::Event) -> Option<EventTarget> {
	let target = event
		.dyn_ref::<web_sys::FocusEvent>()
		.expect("validated focus event changed interface")
		.related_target();
	web_target(target)
}

#[cfg(native)]
fn encrypted_init_data_type(event: &platform::Event) -> String {
	let reinhardt_core::types::page::NativeEventPayload::MediaEncrypted(data) = event.payload()
	else {
		unreachable!("validated encrypted-media event changed interface")
	};
	data.init_data_type.clone()
}

#[cfg(wasm)]
fn encrypted_init_data_type(event: &platform::Event) -> String {
	event
		.dyn_ref::<web_sys::MediaEncryptedEvent>()
		.expect("validated encrypted-media event changed interface")
		.init_data_type()
}

#[cfg(native)]
fn encrypted_init_data(event: &platform::Event) -> Vec<u8> {
	let reinhardt_core::types::page::NativeEventPayload::MediaEncrypted(data) = event.payload()
	else {
		unreachable!("validated encrypted-media event changed interface")
	};
	data.init_data.clone()
}

#[cfg(wasm)]
fn encrypted_init_data(event: &platform::Event) -> Vec<u8> {
	event
		.dyn_ref::<web_sys::MediaEncryptedEvent>()
		.expect("validated encrypted-media event changed interface")
		.init_data()
		.ok()
		.flatten()
		.map(|buffer| js_sys::Uint8Array::new(&buffer).to_vec())
		.unwrap_or_default()
}

#[cfg(native)]
fn picture_in_picture_size(event: &platform::Event) -> (u32, u32) {
	let reinhardt_core::types::page::NativeEventPayload::PictureInPicture(data) = event.payload()
	else {
		unreachable!("validated picture-in-picture event changed interface")
	};
	(data.width, data.height)
}

#[cfg(wasm)]
fn picture_in_picture_size(event: &platform::Event) -> (u32, u32) {
	let window = js_sys::Reflect::get(
		event.as_ref(),
		&wasm_bindgen::JsValue::from_str("pictureInPictureWindow"),
	)
	.unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
	let width = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("width"))
		.ok()
		.and_then(|value| value.as_f64())
		.unwrap_or_default() as u32;
	let height = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("height"))
		.ok()
		.and_then(|value| value.as_f64())
		.unwrap_or_default() as u32;
	(width, height)
}

#[cfg(native)]
fn security_policy_violation_details(event: &platform::Event) -> SecurityPolicyViolationDetails {
	let reinhardt_core::types::page::NativeEventPayload::SecurityPolicyViolation(data) =
		event.payload()
	else {
		unreachable!("validated security-policy event changed interface")
	};
	SecurityPolicyViolationDetails {
		blocked_uri: data.blocked_uri.clone(),
		column_number: data.column_number,
		disposition: data.disposition.clone(),
		document_uri: data.document_uri.clone(),
		effective_directive: data.effective_directive.clone(),
		line_number: data.line_number,
		original_policy: data.original_policy.clone(),
		referrer: data.referrer.clone(),
		sample: data.sample.clone(),
		source_file: data.source_file.clone(),
		status_code: data.status_code,
		violated_directive: data.violated_directive.clone(),
	}
}

#[cfg(wasm)]
fn security_policy_violation_details(event: &platform::Event) -> SecurityPolicyViolationDetails {
	let event = event
		.dyn_ref::<web_sys::SecurityPolicyViolationEvent>()
		.expect("validated security-policy event changed interface");
	SecurityPolicyViolationDetails {
		blocked_uri: event.blocked_uri(),
		column_number: u32::try_from(event.column_number()).unwrap_or_default(),
		disposition: reflect_string(event.as_ref(), "disposition"),
		document_uri: event.document_uri(),
		effective_directive: event.effective_directive(),
		line_number: u32::try_from(event.line_number()).unwrap_or_default(),
		original_policy: event.original_policy(),
		referrer: event.referrer(),
		sample: event.sample(),
		source_file: event.source_file(),
		status_code: event.status_code(),
		violated_directive: event.violated_directive(),
	}
}

#[cfg(native)]
fn submit_event_target(event: &platform::Event) -> Option<EventTarget> {
	let reinhardt_core::types::page::NativeEventPayload::Submit(data) = event.payload() else {
		unreachable!("validated submit event changed interface")
	};
	native_target(&data.submitter)
}

#[cfg(wasm)]
fn submit_event_target(event: &platform::Event) -> Option<EventTarget> {
	let submitter = event
		.dyn_ref::<web_sys::SubmitEvent>()
		.expect("validated submit event changed interface")
		.submitter()
		.map(|element| element.unchecked_into::<web_sys::EventTarget>());
	web_target(submitter)
}

#[cfg(native)]
fn time_detail(event: &platform::Event) -> i32 {
	let reinhardt_core::types::page::NativeEventPayload::Time(data) = event.payload() else {
		unreachable!("validated time event changed interface")
	};
	data.detail
}

#[cfg(wasm)]
fn time_detail(event: &platform::Event) -> i32 {
	event
		.dyn_ref::<web_sys::TimeEvent>()
		.expect("validated time event changed interface")
		.detail()
}

#[cfg(native)]
fn toggle_states(event: &platform::Event) -> (String, String) {
	let reinhardt_core::types::page::NativeEventPayload::Toggle(data) = event.payload() else {
		unreachable!("validated toggle event changed interface")
	};
	(data.old_state.clone(), data.new_state.clone())
}

#[cfg(wasm)]
fn toggle_states(event: &platform::Event) -> (String, String) {
	let event = event
		.dyn_ref::<web_sys::ToggleEvent>()
		.expect("validated toggle event changed interface");
	(event.old_state(), event.new_state())
}

fn modifiers_from_state(alt: bool, control: bool, meta: bool, shift: bool) -> Modifiers {
	Modifiers {
		alt,
		control,
		meta,
		shift,
	}
}

#[cfg(native)]
fn native_touch_point(data: &reinhardt_core::types::page::TouchPointData) -> TouchPoint {
	TouchPoint::new(
		data.identifier,
		Point::new(data.client_x, data.client_y),
		Point::new(data.screen_x, data.screen_y),
		Point::new(data.page_x, data.page_y),
		Point::new(data.radius_x, data.radius_y),
		data.rotation_angle,
		data.force,
	)
}

#[derive(Clone, Copy)]
enum TouchCollection {
	All,
	Target,
	Changed,
}

#[cfg(native)]
fn touch_collection(event: &platform::Event, collection: TouchCollection) -> Vec<TouchPoint> {
	let reinhardt_core::types::page::NativeEventPayload::Touch(data) = event.payload() else {
		unreachable!("validated touch event changed interface")
	};
	match collection {
		TouchCollection::All => data.touches.iter().map(native_touch_point).collect(),
		TouchCollection::Target => data.target_touches.iter().map(native_touch_point).collect(),
		TouchCollection::Changed => data
			.changed_touches
			.iter()
			.map(native_touch_point)
			.collect(),
	}
}

#[cfg(wasm)]
fn touch_collection(event: &platform::Event, collection: TouchCollection) -> Vec<TouchPoint> {
	let event = event
		.dyn_ref::<web_sys::TouchEvent>()
		.expect("validated touch event changed interface");
	match collection {
		TouchCollection::All => touch_points(event.touches()),
		TouchCollection::Target => touch_points(event.target_touches()),
		TouchCollection::Changed => touch_points(event.changed_touches()),
	}
}

#[cfg(native)]
fn touch_modifiers(event: &platform::Event) -> Modifiers {
	let reinhardt_core::types::page::NativeEventPayload::Touch(data) = event.payload() else {
		unreachable!("validated touch event changed interface")
	};
	modifiers_from_state(
		data.modifiers.alt,
		data.modifiers.control,
		data.modifiers.meta,
		data.modifiers.shift,
	)
}

#[cfg(wasm)]
fn touch_modifiers(event: &platform::Event) -> Modifiers {
	let event = event
		.dyn_ref::<web_sys::TouchEvent>()
		.expect("validated touch event changed interface");
	modifiers_from_state(
		event.alt_key(),
		event.ctrl_key(),
		event.meta_key(),
		event.shift_key(),
	)
}

#[cfg(native)]
fn transition_values(event: &platform::Event) -> (String, f64, String) {
	let reinhardt_core::types::page::NativeEventPayload::Transition(data) = event.payload() else {
		unreachable!("validated transition event changed interface")
	};
	(
		data.property_name.clone(),
		data.elapsed_time,
		data.pseudo_element.clone(),
	)
}

#[cfg(wasm)]
fn transition_values(event: &platform::Event) -> (String, f64, String) {
	let event = event
		.dyn_ref::<web_sys::TransitionEvent>()
		.expect("validated transition event changed interface");
	(
		event.property_name(),
		f64::from(event.elapsed_time()),
		event.pseudo_element(),
	)
}

#[cfg(native)]
fn wheel_values(event: &platform::Event) -> (f64, f64, f64, u32) {
	let reinhardt_core::types::page::NativeEventPayload::Wheel(data) = event.payload() else {
		unreachable!("validated wheel event changed interface")
	};
	(data.delta_x, data.delta_y, data.delta_z, data.delta_mode)
}

#[cfg(wasm)]
fn wheel_values(event: &platform::Event) -> (f64, f64, f64, u32) {
	let event = event
		.dyn_ref::<web_sys::WheelEvent>()
		.expect("validated wheel event changed interface");
	(
		event.delta_x(),
		event.delta_y(),
		event.delta_z(),
		event.delta_mode(),
	)
}

#[cfg(native)]
fn xr_input_source(event: &platform::Event) -> XrInputSourceDescriptor {
	let reinhardt_core::types::page::NativeEventPayload::XrInputSource(data) = event.payload()
	else {
		unreachable!("validated XR input-source event changed interface")
	};
	XrInputSourceDescriptor {
		handedness: data.handedness.clone(),
		target_ray_mode: data.target_ray_mode.clone(),
		profiles: data.profiles.clone(),
	}
}

#[cfg(wasm)]
fn xr_input_source(event: &platform::Event) -> XrInputSourceDescriptor {
	let source = js_sys::Reflect::get(
		event.as_ref(),
		&wasm_bindgen::JsValue::from_str("inputSource"),
	)
	.unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
	let profiles = js_sys::Reflect::get(&source, &wasm_bindgen::JsValue::from_str("profiles"))
		.ok()
		.map(|value| js_sys::Array::from(&value))
		.map(|profiles| {
			profiles
				.iter()
				.filter_map(|value| value.as_string())
				.collect()
		})
		.unwrap_or_default();
	XrInputSourceDescriptor {
		handedness: reflect_string(&source, "handedness"),
		target_ray_mode: reflect_string(&source, "targetRayMode"),
		profiles,
	}
}

macro_rules! interface_methods {
	($payload:ident, Input) => {
		impl $payload {
			/// Returns inserted characters when the input operation carried data.
			#[must_use]
			pub fn data(&self) -> Option<String> {
				platform::input_snapshot(&self.core.raw).data
			}

			/// Returns the standardized input operation type.
			#[must_use]
			pub fn input_type(&self) -> Option<String> {
				platform::input_snapshot(&self.core.raw).input_type
			}

			/// Returns whether this input occurred during IME composition.
			#[must_use]
			pub fn is_composing(&self) -> bool {
				platform::input_snapshot(&self.core.raw).is_composing
			}
		}
	};
	($payload:ident, Keyboard) => {
		impl $payload {
			/// Returns the logical key value.
			#[must_use]
			pub fn key(&self) -> String {
				platform::keyboard_snapshot(&self.core.raw).key
			}

			/// Returns the physical key code.
			#[must_use]
			pub fn code(&self) -> String {
				platform::keyboard_snapshot(&self.core.raw).code
			}

			/// Returns the keyboard location value.
			#[must_use]
			pub fn location(&self) -> u32 {
				platform::keyboard_snapshot(&self.core.raw).location
			}

			/// Returns whether this event is an automatic repeat.
			#[must_use]
			pub fn repeat(&self) -> bool {
				platform::keyboard_snapshot(&self.core.raw).repeat
			}

			/// Returns whether this event occurred during IME composition.
			#[must_use]
			pub fn is_composing(&self) -> bool {
				platform::keyboard_snapshot(&self.core.raw).is_composing
			}

			/// Returns keyboard modifiers active during dispatch.
			#[must_use]
			pub fn modifiers(&self) -> Modifiers {
				let data = platform::keyboard_snapshot(&self.core.raw);
				Modifiers {
					alt: data.alt,
					control: data.control,
					meta: data.meta,
					shift: data.shift,
				}
			}
		}
	};
	($payload:ident, Mouse) => {
		mouse_methods!($payload);
	};
	($payload:ident, Pointer) => {
		mouse_methods!($payload);
		impl $payload {
			/// Returns the stable pointer identifier.
			#[must_use]
			pub fn pointer_id(&self) -> i32 {
				platform::pointer_snapshot(&self.core.raw).pointer_id
			}

			/// Returns the pointer device classification.
			#[must_use]
			pub fn pointer_type(&self) -> PointerKind {
				platform::pointer_snapshot(&self.core.raw)
					.pointer_kind
					.into()
			}

			/// Returns normalized device pressure.
			#[must_use]
			pub fn pressure(&self) -> f32 {
				platform::pointer_snapshot(&self.core.raw).pressure
			}

			/// Returns the contact width.
			#[must_use]
			pub fn width(&self) -> f64 {
				platform::pointer_snapshot(&self.core.raw).width
			}

			/// Returns the contact height.
			#[must_use]
			pub fn height(&self) -> f64 {
				platform::pointer_snapshot(&self.core.raw).height
			}

			/// Returns barrel pressure for compatible pen devices.
			#[must_use]
			pub fn tangential_pressure(&self) -> f32 {
				platform::pointer_snapshot(&self.core.raw).tangential_pressure
			}

			/// Returns pen tilt on the x-axis.
			#[must_use]
			pub fn tilt_x(&self) -> i32 {
				platform::pointer_snapshot(&self.core.raw).tilt_x
			}

			/// Returns pen tilt on the y-axis.
			#[must_use]
			pub fn tilt_y(&self) -> i32 {
				platform::pointer_snapshot(&self.core.raw).tilt_y
			}

			/// Returns pen rotation in degrees.
			#[must_use]
			pub fn twist(&self) -> i32 {
				platform::pointer_snapshot(&self.core.raw).twist
			}

			/// Returns whether this is the primary pointer of its kind.
			#[must_use]
			pub fn is_primary(&self) -> bool {
				platform::pointer_snapshot(&self.core.raw).is_primary
			}
		}
	};
	($payload:ident, Drag) => {
		mouse_methods!($payload);
		impl $payload {
			/// Returns the captured plain-text drag data, when available.
			#[must_use]
			pub fn data(&self) -> Option<String> {
				#[cfg(native)]
				{
					let reinhardt_core::types::page::NativeEventPayload::Drag(data) =
						self.core.raw.payload()
					else {
						unreachable!("validated drag event changed interface")
					};
					data.data.clone()
				}
				#[cfg(wasm)]
				{
					self.core
						.raw
						.dyn_ref::<web_sys::DragEvent>()
						.expect("validated drag event changed interface")
						.data_transfer()
						.and_then(|data| data.get_data("text/plain").ok())
				}
			}
		}
	};
	($payload:ident, Wheel) => {
		mouse_methods!($payload);
		impl $payload {
			/// Returns the horizontal scroll delta.
			#[must_use]
			pub fn delta_x(&self) -> f64 {
				wheel_values(&self.core.raw).0
			}

			/// Returns the vertical scroll delta.
			#[must_use]
			pub fn delta_y(&self) -> f64 {
				wheel_values(&self.core.raw).1
			}

			/// Returns the depth-axis scroll delta.
			#[must_use]
			pub fn delta_z(&self) -> f64 {
				wheel_values(&self.core.raw).2
			}

			/// Returns the unit used by the delta values.
			#[must_use]
			pub fn delta_mode(&self) -> u32 {
				wheel_values(&self.core.raw).3
			}
		}
	};
	($payload:ident, Generic) => {};
	($payload:ident, Animation) => {
		impl $payload {
			/// Returns the animation name.
			#[must_use]
			pub fn animation_name(&self) -> String {
				animation_values(&self.core.raw).0
			}

			/// Returns elapsed animation time in seconds.
			#[must_use]
			pub fn elapsed_time(&self) -> f64 {
				animation_values(&self.core.raw).1
			}

			/// Returns the pseudo-element name, when applicable.
			#[must_use]
			pub fn pseudo_element(&self) -> String {
				animation_values(&self.core.raw).2
			}
		}
	};
	($payload:ident, Clipboard) => {
		impl $payload {
			/// Returns the captured plain-text clipboard data, when available.
			#[must_use]
			pub fn text(&self) -> Option<String> {
				clipboard_text(&self.core.raw)
			}
		}
	};
	($payload:ident, Command) => {
		impl $payload {
			/// Returns the command name.
			#[must_use]
			pub fn command(&self) -> String {
				command_values(&self.core.raw).0
			}

			/// Returns the element that invoked the command, when available.
			#[must_use]
			pub fn source(&self) -> Option<EventTarget> {
				command_values(&self.core.raw).1
			}
		}
	};
	($payload:ident, Composition) => {
		impl $payload {
			/// Returns characters produced by the input method.
			#[must_use]
			pub fn data(&self) -> String {
				composition_data(&self.core.raw)
			}
		}
	};
	($payload:ident, Focus) => {
		impl $payload {
			/// Returns the focus target related to the transition, when available.
			#[must_use]
			pub fn related_target(&self) -> Option<EventTarget> {
				focus_related_target(&self.core.raw)
			}
		}
	};
	($payload:ident, MediaEncrypted) => {
		impl $payload {
			/// Returns the initialization-data format.
			#[must_use]
			pub fn init_data_type(&self) -> String {
				encrypted_init_data_type(&self.core.raw)
			}

			/// Returns owned initialization bytes.
			#[must_use]
			pub fn init_data(&self) -> Vec<u8> {
				encrypted_init_data(&self.core.raw)
			}
		}
	};
	($payload:ident, PictureInPicture) => {
		impl $payload {
			/// Returns the picture-in-picture window width in CSS pixels.
			#[must_use]
			pub fn width(&self) -> u32 {
				picture_in_picture_size(&self.core.raw).0
			}

			/// Returns the picture-in-picture window height in CSS pixels.
			#[must_use]
			pub fn height(&self) -> u32 {
				picture_in_picture_size(&self.core.raw).1
			}
		}
	};
	($payload:ident, SecurityPolicyViolation) => {
		impl $payload {
			/// Returns the complete owned policy-violation details.
			#[must_use]
			pub fn details(&self) -> SecurityPolicyViolationDetails {
				security_policy_violation_details(&self.core.raw)
			}
		}
	};
	($payload:ident, Submit) => {
		impl $payload {
			/// Returns the element that initiated submission, when available.
			#[must_use]
			pub fn submitter(&self) -> Option<EventTarget> {
				submit_event_target(&self.core.raw)
			}
		}
	};
	($payload:ident, Time) => {
		impl $payload {
			/// Returns the repeat or timing detail supplied by the event.
			#[must_use]
			pub fn detail(&self) -> i32 {
				time_detail(&self.core.raw)
			}
		}
	};
	($payload:ident, Toggle) => {
		impl $payload {
			/// Returns the state before the toggle.
			#[must_use]
			pub fn old_state(&self) -> String {
				toggle_states(&self.core.raw).0
			}

			/// Returns the state after the toggle.
			#[must_use]
			pub fn new_state(&self) -> String {
				toggle_states(&self.core.raw).1
			}
		}
	};
	($payload:ident, Touch) => {
		impl $payload {
			/// Returns all current touch points.
			#[must_use]
			pub fn touches(&self) -> Vec<TouchPoint> {
				touch_collection(&self.core.raw, TouchCollection::All)
			}

			/// Returns current touch points that began on the target.
			#[must_use]
			pub fn target_touches(&self) -> Vec<TouchPoint> {
				touch_collection(&self.core.raw, TouchCollection::Target)
			}

			/// Returns touch points changed by this event.
			#[must_use]
			pub fn changed_touches(&self) -> Vec<TouchPoint> {
				touch_collection(&self.core.raw, TouchCollection::Changed)
			}

			/// Returns keyboard modifiers active during dispatch.
			#[must_use]
			pub fn modifiers(&self) -> Modifiers {
				touch_modifiers(&self.core.raw)
			}
		}
	};
	($payload:ident, Transition) => {
		impl $payload {
			/// Returns the transitioned CSS property name.
			#[must_use]
			pub fn property_name(&self) -> String {
				transition_values(&self.core.raw).0
			}

			/// Returns elapsed transition time in seconds.
			#[must_use]
			pub fn elapsed_time(&self) -> f64 {
				transition_values(&self.core.raw).1
			}

			/// Returns the pseudo-element name, when applicable.
			#[must_use]
			pub fn pseudo_element(&self) -> String {
				transition_values(&self.core.raw).2
			}
		}
	};
	($payload:ident, XrInputSource) => {
		impl $payload {
			/// Returns owned WebXR input-source metadata.
			#[must_use]
			pub fn input_source(&self) -> XrInputSourceDescriptor {
				xr_input_source(&self.core.raw)
			}
		}
	};
}

macro_rules! define_event_payload_impls {
	(
		$kind:ident,
		$dom_name:literal,
		$payload:ident,
		$interface:ident,
		[$($fallback:ident),* $(,)?],
		[$($capability:ident),* $(,)?]
	) => {
			impl EventPayload for $payload {
				const EVENT: KnownEvent = KnownEvent::$kind;

				fn try_from_raw(event: platform::Event) -> Result<Self, EventConversionError> {
					if !platform::event_name_matches(&event, Self::EVENT) {
						return Err(EventConversionError::UnexpectedName {
							expected: $dom_name,
							actual: platform::event_type(&event),
						});
					}
					platform::event_interface(
						&event,
						EventInterface::$interface,
						&[$(EventInterface::$fallback),*],
					)
					.map_err(|actual| EventConversionError::UnexpectedInterface {
						event: $dom_name,
						primary: EventInterface::$interface,
						fallbacks: &[$(EventInterface::$fallback),*],
						actual,
					})?;

					let target = platform::target(&event);
					let current_target = platform::current_target(&event);
					Ok(Self {
						core: PayloadCore {
							raw: event,
							target,
							current_target,
						},
					})
				}
			}

			impl $payload {
				/// Returns the unmodified cross-target raw event.
				#[must_use]
				pub const fn raw(&self) -> &platform::Event {
					&self.core.raw
				}

				/// Returns the exact standardized DOM event name.
				#[must_use]
				pub const fn event_type(&self) -> &'static str {
					$dom_name
				}

				/// Prevents the default action when the event is cancelable.
				pub fn prevent_default(&self) {
					self.core.raw.prevent_default();
				}

				/// Stops dispatch before the next ancestor listener.
				pub fn stop_propagation(&self) {
					self.core.raw.stop_propagation();
				}

				/// Stops later listeners on this target and ancestor traversal.
				pub fn stop_immediate_propagation(&self) {
					self.core.raw.stop_immediate_propagation();
				}

				/// Returns whether the default action has been prevented.
				#[must_use]
				pub fn default_prevented(&self) -> bool {
					self.core.raw.default_prevented()
				}

				/// Returns the originating event target snapshot.
				#[must_use]
				pub fn target(&self) -> Option<EventTarget> {
					self.core.target.clone()
				}

				/// Returns the listener target snapshot captured during conversion.
				#[must_use]
				pub fn current_target(&self) -> Option<EventTarget> {
					self.core.current_target.clone()
				}

				/// Returns whether the event bubbles.
				#[must_use]
				pub fn bubbles(&self) -> bool {
					platform::bubbles(&self.core.raw)
				}

				/// Returns whether the event can be canceled.
				#[must_use]
				pub fn cancelable(&self) -> bool {
					platform::cancelable(&self.core.raw)
				}

				/// Returns whether the event crosses shadow boundaries.
				#[must_use]
				pub fn composed(&self) -> bool {
					platform::composed(&self.core.raw)
				}

				/// Returns the event timestamp in milliseconds.
				#[must_use]
				pub fn time_stamp(&self) -> f64 {
					platform::time_stamp(&self.core.raw)
				}

				/// Returns whether the user agent created the event.
				#[must_use]
				pub fn is_trusted(&self) -> bool {
					platform::is_trusted(&self.core.raw)
				}
			}

			interface_methods!($payload, $interface);
			$(capability_methods!($payload, $capability);)*
	};
}

macro_rules! define_event_payload {
	(
		;
		$kind:ident,
		$dom_name:literal,
		$payload:ident,
		$interface:ident,
		[$($fallback:ident),* $(,)?],
		[$($capability:ident),* $(,)?]
	) => {
		#[doc = concat!("Typed payload for the standard `", $dom_name, "` event.")]
		#[derive(Clone, Debug)]
		pub struct $payload {
			core: PayloadCore,
		}

		define_event_payload_impls!(
			$kind,
			$dom_name,
			$payload,
			$interface,
			[$($fallback),*],
			[$($capability),*]
		);
	};
	(
		$deprecation:literal;
		$kind:ident,
		$dom_name:literal,
		$payload:ident,
		$interface:ident,
		[$($fallback:ident),* $(,)?],
		[$($capability:ident),* $(,)?]
	) => {
		#[doc = concat!(
			"Typed payload for the standard `",
			$dom_name,
			"` event. This compatibility payload is deprecated; use `keydown` or `keyup` instead."
		)]
		#[deprecated(note = $deprecation)]
		#[derive(Clone, Debug)]
		pub struct $payload {
			core: PayloadCore,
		}

		// The catalog spelling names this module while it isolates deprecated payload use.
		#[allow(deprecated, non_snake_case)]
		mod $kind {
			use super::*;

			define_event_payload_impls!(
				$kind,
				$dom_name,
				$payload,
				$interface,
				[$($fallback),*],
				[$($capability),*]
			);
		}
	};
}

macro_rules! define_event_payloads {
	(
		$(
			$kind:ident,
			$dom_name:literal,
			$payload:ident,
			$interface:ident,
			[$($fallback:ident),* $(,)?],
			[$($capability:ident),* $(,)?],
			$bubbles:literal,
			$cancelable:literal,
			$composed:literal,
			$fixture_defaults:ident;
		)*
	) => {
		$(
			reinhardt_event_catalog::__reinhardt_event_deprecation! {
				$kind => define_event_payload! {
					$kind,
					$dom_name,
					$payload,
					$interface,
					[$($fallback),*],
					[$($capability),*]
				}
			}
		)*
	};
}

reinhardt_event_catalog::__reinhardt_event_catalog!(define_event_payloads);
