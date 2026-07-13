//! Authoritative metadata for standard HTML and SVG element events.
//!
//! This dependency-free crate is the shared source of truth for event parsing,
//! runtime registration, public payload generation, hydration, and native test
//! dispatch. Consumers that generate code can expand the hidden catalog macro
//! without depending on parser or token libraries here.

#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// Browser event interface family used to carry an event's intrinsic data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EventInterface {
	/// Base DOM `Event` data.
	Generic,
	/// CSS `AnimationEvent` data.
	Animation,
	/// Clipboard event data.
	Clipboard,
	/// HTML `CommandEvent` data.
	Command,
	/// IME `CompositionEvent` data.
	Composition,
	/// Drag-and-drop `DragEvent` data.
	Drag,
	/// `FocusEvent` data.
	Focus,
	/// `InputEvent` data.
	Input,
	/// `KeyboardEvent` data.
	Keyboard,
	/// Encrypted-media `MediaEncryptedEvent` data.
	MediaEncrypted,
	/// `MouseEvent` data.
	Mouse,
	/// Picture-in-picture event data.
	PictureInPicture,
	/// `PointerEvent` data.
	Pointer,
	/// Content Security Policy violation data.
	SecurityPolicyViolation,
	/// `SubmitEvent` data.
	Submit,
	/// SVG animation `TimeEvent` data.
	Time,
	/// HTML `ToggleEvent` data.
	Toggle,
	/// `TouchEvent` data.
	Touch,
	/// CSS `TransitionEvent` data.
	Transition,
	/// `WheelEvent` data.
	Wheel,
	/// WebXR `XRInputSourceEvent` data.
	XrInputSource,
}

/// Target-dependent convenience accessors exposed by a typed event payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EventCapability {
	/// Read the current target's control value or editable text.
	Value,
	/// Read the current target's checked state.
	Checked,
	/// Read all selected values from a multi-select control.
	SelectedValues,
	/// Read file metadata from a file input.
	Files,
}

/// Default dispatch flags used when constructing native event fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventBehavior {
	/// Whether the event traverses ancestor listeners by default.
	pub bubbles: bool,
	/// Whether `prevent_default` can cancel the event by default.
	pub cancelable: bool,
	/// Whether the event crosses a shadow DOM boundary by default.
	pub composed: bool,
}

impl EventBehavior {
	/// Creates event behavior defaults from standard dispatch flags.
	#[must_use]
	pub const fn new(bubbles: bool, cancelable: bool, composed: bool) -> Self {
		Self {
			bubbles,
			cancelable,
			composed,
		}
	}
}

/// Deterministic mouse state used when constructing a native event fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventFixtureDefaults {
	/// Changed DOM mouse button number.
	pub mouse_button: i16,
	/// Pressed DOM mouse-button bitmask.
	pub mouse_buttons: u16,
}

impl EventFixtureDefaults {
	/// No mouse button changed or remains pressed.
	pub const NONE: Self = Self::new(-1, 0);
	/// The primary mouse button changed and no button remains pressed.
	pub const PRIMARY: Self = Self::new(0, 0);
	/// The primary mouse button changed and remains pressed.
	pub const PRIMARY_PRESSED: Self = Self::new(0, 1);

	const fn new(mouse_button: i16, mouse_buttons: u16) -> Self {
		Self {
			mouse_button,
			mouse_buttons,
		}
	}
}

/// Metadata for one standardized element event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventSpec {
	/// Closed event identifier shared by all catalog consumers.
	pub kind: KnownEvent,
	/// Exact case-sensitive name passed to the DOM event APIs.
	pub dom_name: &'static str,
	/// Exact public event payload type name.
	pub payload_name: &'static str,
	/// Preferred standardized browser interface.
	pub primary_interface: EventInterface,
	/// Standardized compatibility interfaces accepted by adapters.
	pub fallback_interfaces: &'static [EventInterface],
	/// Target-dependent convenience accessors available to the payload.
	pub capabilities: &'static [EventCapability],
	/// Native fixture defaults for propagation and cancellation.
	pub behavior: EventBehavior,
	/// Native fixture defaults for interface-specific mouse state.
	pub fixture_defaults: EventFixtureDefaults,
	/// Deprecation guidance for compatibility events.
	pub deprecation: Option<&'static str>,
}

/// Expands the authoritative deprecation metadata for one event.
#[doc(hidden)]
#[macro_export]
macro_rules! __reinhardt_event_deprecation {
	(KeyPress => $callback:ident ! { $($tokens:tt)* }) => {
		$callback! {
			"keypress is deprecated; use keydown or keyup instead";
			$($tokens)*
		}
	};
	($kind:ident => $callback:ident ! { $($tokens:tt)* }) => {
		$callback! {
			;
			$($tokens)*
		}
	};
}

/// Expands the authoritative catalog into a consumer-provided callback macro.
///
/// The callback receives consumer-neutral identifiers and literals in this
/// order: known-event variant, DOM name, payload identifier, interface
/// identifier, fallback interface identifiers, capability identifiers, then
/// the `bubbles`, `cancelable`, and `composed` defaults, followed by the
/// associated [`EventFixtureDefaults`] constant.
#[doc(hidden)]
#[macro_export]
macro_rules! __reinhardt_event_catalog {
	($callback:ident) => {
		$callback! {
			Abort, "abort", AbortEvent, Generic, [], [], false, false, false, NONE;
			AuxClick, "auxclick", AuxClickEvent, Pointer, [], [], true, true, true, NONE;
			BeforeInput, "beforeinput", BeforeInputEvent, Input, [], [Value], true, true, true, NONE;
			BeforeMatch, "beforematch", BeforeMatchEvent, Generic, [], [], true, false, true, NONE;
			BeforeToggle, "beforetoggle", BeforeToggleEvent, Toggle, [], [], false, true, false, NONE;
			Blur, "blur", BlurEvent, Focus, [], [], false, false, true, NONE;
			Cancel, "cancel", CancelEvent, Generic, [], [], false, true, false, NONE;
			CanPlay, "canplay", CanPlayEvent, Generic, [], [], false, false, false, NONE;
			CanPlayThrough, "canplaythrough", CanPlayThroughEvent, Generic, [], [], false, false, false, NONE;
			Change, "change", ChangeEvent, Generic, [], [Value, Checked, SelectedValues, Files], true, false, false, NONE;
			Click, "click", ClickEvent, Pointer, [Mouse], [], true, true, true, PRIMARY;
			Close, "close", CloseEvent, Generic, [], [], false, false, false, NONE;
			Command, "command", CommandEvent, Command, [], [], true, true, true, NONE;
			ContextLost, "contextlost", ContextLostEvent, Generic, [], [], false, true, false, NONE;
			ContextMenu, "contextmenu", ContextMenuEvent, Pointer, [], [], true, true, true, NONE;
			ContextRestored, "contextrestored", ContextRestoredEvent, Generic, [], [], false, false, false, NONE;
			Copy, "copy", CopyEvent, Clipboard, [], [], true, true, true, NONE;
			CueChange, "cuechange", CueChangeEvent, Generic, [], [], false, false, false, NONE;
			Cut, "cut", CutEvent, Clipboard, [], [], true, true, true, NONE;
			DblClick, "dblclick", DblClickEvent, Mouse, [], [], true, true, true, NONE;
			Drag, "drag", DragEvent, Drag, [], [], true, true, true, NONE;
			DragEnd, "dragend", DragEndEvent, Drag, [], [], true, false, true, NONE;
			DragEnter, "dragenter", DragEnterEvent, Drag, [], [], true, true, true, NONE;
			DragLeave, "dragleave", DragLeaveEvent, Drag, [], [], true, false, true, NONE;
			DragOver, "dragover", DragOverEvent, Drag, [], [], true, true, true, NONE;
			DragStart, "dragstart", DragStartEvent, Drag, [], [], true, true, true, NONE;
			Drop, "drop", DropEvent, Drag, [], [], true, true, true, NONE;
			DurationChange, "durationchange", DurationChangeEvent, Generic, [], [], false, false, false, NONE;
			Emptied, "emptied", EmptiedEvent, Generic, [], [], false, false, false, NONE;
			Ended, "ended", EndedEvent, Generic, [], [], false, false, false, NONE;
			Error, "error", ErrorEvent, Generic, [], [], false, false, false, NONE;
			Focus, "focus", FocusEvent, Focus, [], [], false, false, true, NONE;
			FormData, "formdata", FormDataEvent, Generic, [], [], true, false, false, NONE;
			Input, "input", InputEvent, Input, [Generic], [Value, Checked, SelectedValues, Files], true, false, true, NONE;
			Invalid, "invalid", InvalidEvent, Generic, [], [], false, true, false, NONE;
			KeyDown, "keydown", KeyDownEvent, Keyboard, [], [], true, true, true, NONE;
			KeyPress, "keypress", KeyPressEvent, Keyboard, [], [], true, true, true, NONE;
			KeyUp, "keyup", KeyUpEvent, Keyboard, [], [], true, true, true, NONE;
			Load, "load", LoadEvent, Generic, [], [], false, false, false, NONE;
			LoadedData, "loadeddata", LoadedDataEvent, Generic, [], [], false, false, false, NONE;
			LoadedMetadata, "loadedmetadata", LoadedMetadataEvent, Generic, [], [], false, false, false, NONE;
			LoadStart, "loadstart", LoadStartEvent, Generic, [], [], false, false, false, NONE;
			MouseDown, "mousedown", MouseDownEvent, Mouse, [], [], true, true, true, PRIMARY_PRESSED;
			MouseEnter, "mouseenter", MouseEnterEvent, Mouse, [], [], false, false, false, NONE;
			MouseLeave, "mouseleave", MouseLeaveEvent, Mouse, [], [], false, false, false, NONE;
			MouseMove, "mousemove", MouseMoveEvent, Mouse, [], [], true, true, true, NONE;
			MouseOut, "mouseout", MouseOutEvent, Mouse, [], [], true, true, true, NONE;
			MouseOver, "mouseover", MouseOverEvent, Mouse, [], [], true, true, true, NONE;
			MouseUp, "mouseup", MouseUpEvent, Mouse, [], [], true, true, true, PRIMARY;
			Paste, "paste", PasteEvent, Clipboard, [], [], true, true, true, NONE;
			Pause, "pause", PauseEvent, Generic, [], [], false, false, false, NONE;
			Play, "play", PlayEvent, Generic, [], [], false, false, false, NONE;
			Playing, "playing", PlayingEvent, Generic, [], [], false, false, false, NONE;
			Progress, "progress", ProgressEvent, Generic, [], [], false, false, false, NONE;
			RateChange, "ratechange", RateChangeEvent, Generic, [], [], false, false, false, NONE;
			Reset, "reset", ResetEvent, Generic, [], [], true, true, false, NONE;
			Resize, "resize", ResizeEvent, Generic, [], [], false, false, false, NONE;
			Scroll, "scroll", ScrollEvent, Generic, [], [], false, false, false, NONE;
			ScrollEnd, "scrollend", ScrollEndEvent, Generic, [], [], false, false, false, NONE;
			SecurityPolicyViolation, "securitypolicyviolation", SecurityPolicyViolationEvent, SecurityPolicyViolation, [], [], true, false, true, NONE;
			Seeked, "seeked", SeekedEvent, Generic, [], [], false, false, false, NONE;
			Seeking, "seeking", SeekingEvent, Generic, [], [], false, false, false, NONE;
			Select, "select", SelectEvent, Generic, [], [Value], true, false, false, NONE;
			SlotChange, "slotchange", SlotChangeEvent, Generic, [], [], true, false, false, NONE;
			Stalled, "stalled", StalledEvent, Generic, [], [], false, false, false, NONE;
			Submit, "submit", SubmitEvent, Submit, [], [], true, true, false, NONE;
			Suspend, "suspend", SuspendEvent, Generic, [], [], false, false, false, NONE;
			TimeUpdate, "timeupdate", TimeUpdateEvent, Generic, [], [], false, false, false, NONE;
			Toggle, "toggle", ToggleEvent, Toggle, [], [], false, false, false, NONE;
			VolumeChange, "volumechange", VolumeChangeEvent, Generic, [], [], false, false, false, NONE;
			Waiting, "waiting", WaitingEvent, Generic, [], [], false, false, false, NONE;
			Wheel, "wheel", WheelEvent, Wheel, [], [], true, true, true, NONE;
			CompositionStart, "compositionstart", CompositionStartEvent, Composition, [], [], true, true, true, NONE;
			CompositionUpdate, "compositionupdate", CompositionUpdateEvent, Composition, [], [], true, false, true, NONE;
			CompositionEnd, "compositionend", CompositionEndEvent, Composition, [], [], true, false, true, NONE;
			FocusIn, "focusin", FocusInEvent, Focus, [], [], true, false, true, NONE;
			FocusOut, "focusout", FocusOutEvent, Focus, [], [], true, false, true, NONE;
			PointerDown, "pointerdown", PointerDownEvent, Pointer, [], [], true, true, true, PRIMARY_PRESSED;
			PointerUp, "pointerup", PointerUpEvent, Pointer, [], [], true, true, true, PRIMARY;
			PointerMove, "pointermove", PointerMoveEvent, Pointer, [], [], true, true, true, NONE;
			PointerOver, "pointerover", PointerOverEvent, Pointer, [], [], true, true, true, NONE;
			PointerEnter, "pointerenter", PointerEnterEvent, Pointer, [], [], false, false, false, NONE;
			PointerOut, "pointerout", PointerOutEvent, Pointer, [], [], true, true, true, NONE;
			PointerLeave, "pointerleave", PointerLeaveEvent, Pointer, [], [], false, false, false, NONE;
			PointerCancel, "pointercancel", PointerCancelEvent, Pointer, [], [], true, false, true, NONE;
			GotPointerCapture, "gotpointercapture", GotPointerCaptureEvent, Pointer, [], [], true, false, true, NONE;
			LostPointerCapture, "lostpointercapture", LostPointerCaptureEvent, Pointer, [], [], true, false, true, NONE;
			PointerRawUpdate, "pointerrawupdate", PointerRawUpdateEvent, Pointer, [], [], true, false, true, NONE;
			TouchStart, "touchstart", TouchStartEvent, Touch, [], [], true, true, true, NONE;
			TouchEnd, "touchend", TouchEndEvent, Touch, [], [], true, false, true, NONE;
			TouchMove, "touchmove", TouchMoveEvent, Touch, [], [], true, true, true, NONE;
			TouchCancel, "touchcancel", TouchCancelEvent, Touch, [], [], true, false, true, NONE;
			AnimationStart, "animationstart", AnimationStartEvent, Animation, [], [], true, false, false, NONE;
			AnimationEnd, "animationend", AnimationEndEvent, Animation, [], [], true, false, false, NONE;
			AnimationIteration, "animationiteration", AnimationIterationEvent, Animation, [], [], true, false, false, NONE;
			AnimationCancel, "animationcancel", AnimationCancelEvent, Animation, [], [], true, false, false, NONE;
			TransitionRun, "transitionrun", TransitionRunEvent, Transition, [], [], true, false, false, NONE;
			TransitionStart, "transitionstart", TransitionStartEvent, Transition, [], [], true, false, false, NONE;
			TransitionEnd, "transitionend", TransitionEndEvent, Transition, [], [], true, false, false, NONE;
			TransitionCancel, "transitioncancel", TransitionCancelEvent, Transition, [], [], true, false, false, NONE;
			FullscreenChange, "fullscreenchange", FullscreenChangeEvent, Generic, [], [], true, false, true, NONE;
			FullscreenError, "fullscreenerror", FullscreenErrorEvent, Generic, [], [], true, false, true, NONE;
			SelectionChange, "selectionchange", SelectionChangeEvent, Generic, [], [], true, false, true, NONE;
			SelectStart, "selectstart", SelectStartEvent, Generic, [], [], true, true, true, NONE;
			ScrollSnapChange, "scrollsnapchange", ScrollSnapChangeEvent, Generic, [], [], true, false, true, NONE;
			ScrollSnapChanging, "scrollsnapchanging", ScrollSnapChangingEvent, Generic, [], [], true, false, true, NONE;
			ContentVisibilityAutoStateChange, "contentvisibilityautostatechange", ContentVisibilityAutoStateChangeEvent, Generic, [], [], true, false, false, NONE;
			Encrypted, "encrypted", EncryptedEvent, MediaEncrypted, [], [], false, false, false, NONE;
			WaitingForKey, "waitingforkey", WaitingForKeyEvent, Generic, [], [], false, false, false, NONE;
			EnterPictureInPicture, "enterpictureinpicture", EnterPictureInPictureEvent, PictureInPicture, [], [], false, false, false, NONE;
			LeavePictureInPicture, "leavepictureinpicture", LeavePictureInPictureEvent, PictureInPicture, [], [], false, false, false, NONE;
			BeforeXrSelect, "beforexrselect", BeforeXrSelectEvent, XrInputSource, [], [], true, true, true, NONE;
			BeginEvent, "beginEvent", BeginEvent, Time, [], [], false, false, false, NONE;
			EndEvent, "endEvent", EndEvent, Time, [], [], false, false, false, NONE;
			RepeatEvent, "repeatEvent", RepeatEvent, Time, [], [], false, false, false, NONE;
		}
	};
}

macro_rules! define_catalog {
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
		macro_rules! deprecation_value {
			($note:literal;) => {
				Some($note)
			};
			(;) => {
				None
			};
		}

		/// Closed set of standardized HTML and SVG element events.
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
		#[repr(usize)]
		pub enum KnownEvent {
			$($kind,)*
		}

		impl KnownEvent {
			/// Returns the exact case-sensitive DOM event name.
			#[must_use]
			pub const fn as_str(self) -> &'static str {
				self.spec().dom_name
			}

			/// Returns this event's authoritative catalog metadata.
			#[must_use]
			pub const fn spec(self) -> &'static EventSpec {
				&EVENT_SPECS[self as usize]
			}
		}

		/// Authoritative metadata snapshot for all standardized element events.
		pub const EVENT_SPECS: &[EventSpec] = &[
			$(
				EventSpec {
					kind: KnownEvent::$kind,
					dom_name: $dom_name,
					payload_name: stringify!($payload),
					primary_interface: EventInterface::$interface,
					fallback_interfaces: &[$(EventInterface::$fallback),*],
					capabilities: &[$(EventCapability::$capability),*],
					behavior: EventBehavior::new($bubbles, $cancelable, $composed),
					fixture_defaults: EventFixtureDefaults::$fixture_defaults,
					deprecation: crate::__reinhardt_event_deprecation! {
						$kind => deprecation_value! {}
					},
				},
			)*
		];
	};
}

crate::__reinhardt_event_catalog!(define_catalog);

/// Error returned when a name is not a standardized catalog event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEventName {
	name: String,
}

impl UnknownEventName {
	/// Returns the unknown event name.
	#[must_use]
	pub fn as_str(&self) -> &str {
		&self.name
	}
}

impl fmt::Display for UnknownEventName {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "unknown standard element event `{}`", self.name)
	}
}

impl std::error::Error for UnknownEventName {}

impl FromStr for KnownEvent {
	type Err = UnknownEventName;

	fn from_str(name: &str) -> Result<Self, Self::Err> {
		event_spec(name)
			.map(|spec| spec.kind)
			.ok_or_else(|| UnknownEventName {
				name: name.to_owned(),
			})
	}
}

impl AsRef<str> for KnownEvent {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl fmt::Display for KnownEvent {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.as_str())
	}
}

/// Runtime event name that preserves both known and explicit custom events.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventName {
	/// Standardized HTML or SVG element event.
	Known(KnownEvent),
	/// Explicit raw custom DOM event.
	Custom(Cow<'static, str>),
}

impl EventName {
	/// Returns the exact DOM event name.
	#[must_use]
	pub fn as_str(&self) -> &str {
		match self {
			Self::Known(event) => event.as_str(),
			Self::Custom(name) => name,
		}
	}

	/// Returns the standardized event kind, if this is a known event.
	#[must_use]
	pub const fn known(&self) -> Option<KnownEvent> {
		match self {
			Self::Known(event) => Some(*event),
			Self::Custom(_) => None,
		}
	}
}

impl From<KnownEvent> for EventName {
	fn from(event: KnownEvent) -> Self {
		Self::Known(event)
	}
}

impl AsRef<str> for EventName {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl fmt::Display for EventName {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.as_str())
	}
}

/// Looks up an exact case-sensitive DOM event name.
#[must_use]
pub fn event_spec(name: &str) -> Option<&'static EventSpec> {
	EVENT_SPECS.iter().find(|spec| spec.dom_name == name)
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;
	use std::collections::HashSet;
	use std::str::FromStr;

	use super::{
		EVENT_SPECS, EventBehavior, EventCapability, EventInterface, EventName, KnownEvent,
		event_spec,
	};

	macro_rules! collect_catalog_entries {
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
			const EXPORTED_CATALOG: &[(&str, &str)] = &[
				$(($dom_name, stringify!($payload)),)*
			];
		};
	}

	crate::__reinhardt_event_catalog!(collect_catalog_entries);

	#[test]
	fn dom_names_are_unique() {
		let unique = EVENT_SPECS
			.iter()
			.map(|spec| spec.dom_name)
			.collect::<HashSet<_>>();

		assert_eq!(EVENT_SPECS.len(), 115);
		assert_eq!(unique.len(), EVENT_SPECS.len());
	}

	#[test]
	fn payload_names_are_unique() {
		let unique = EVENT_SPECS
			.iter()
			.map(|spec| spec.payload_name)
			.collect::<HashSet<_>>();

		assert_eq!(unique.len(), EVENT_SPECS.len());
	}

	#[test]
	fn keypress_deprecation_is_authoritative_catalog_metadata() {
		assert_eq!(
			KnownEvent::KeyPress.spec().deprecation,
			Some("keypress is deprecated; use keydown or keyup instead")
		);
		assert_eq!(KnownEvent::KeyDown.spec().deprecation, None);
	}

	#[test]
	fn fixture_mouse_defaults_are_authoritative_catalog_metadata() {
		assert_eq!(KnownEvent::Click.spec().fixture_defaults.mouse_button, 0);
		assert_eq!(KnownEvent::Click.spec().fixture_defaults.mouse_buttons, 0);
		assert_eq!(
			KnownEvent::MouseDown.spec().fixture_defaults.mouse_button,
			0
		);
		assert_eq!(
			KnownEvent::MouseDown.spec().fixture_defaults.mouse_buttons,
			1
		);
		assert_eq!(
			KnownEvent::PointerMove.spec().fixture_defaults.mouse_button,
			-1
		);
		assert_eq!(
			KnownEvent::PointerMove
				.spec()
				.fixture_defaults
				.mouse_buttons,
			0
		);
		let primary_button_events = EVENT_SPECS
			.iter()
			.filter(|spec| spec.fixture_defaults.mouse_button == 0)
			.map(|spec| spec.kind)
			.collect::<Vec<_>>();
		let primary_pressed_events = EVENT_SPECS
			.iter()
			.filter(|spec| spec.fixture_defaults.mouse_buttons == 1)
			.map(|spec| spec.kind)
			.collect::<Vec<_>>();

		assert_eq!(
			primary_button_events,
			[
				KnownEvent::Click,
				KnownEvent::MouseDown,
				KnownEvent::MouseUp,
				KnownEvent::PointerDown,
				KnownEvent::PointerUp,
			]
		);
		assert_eq!(
			primary_pressed_events,
			[KnownEvent::MouseDown, KnownEvent::PointerDown]
		);
	}

	#[test]
	fn exported_catalog_expansion_matches_the_public_snapshot() {
		let public_snapshot = EVENT_SPECS
			.iter()
			.map(|spec| (spec.dom_name, spec.payload_name))
			.collect::<Vec<_>>();

		assert_eq!(EXPORTED_CATALOG, public_snapshot);
	}

	#[test]
	fn lookup_returns_the_catalog_entry() {
		let click = event_spec("click").expect("click must be cataloged");

		assert_eq!(click.kind, KnownEvent::Click);
		assert_eq!(click.payload_name, "ClickEvent");
		assert_eq!(click.primary_interface, EventInterface::Pointer);
		assert_eq!(click.fallback_interfaces, &[EventInterface::Mouse]);
		assert_eq!(event_spec("not-a-standard-event"), None);
	}

	#[test]
	fn svg_timing_names_are_case_sensitive() {
		assert_eq!(
			event_spec("beginEvent").map(|spec| spec.kind),
			Some(KnownEvent::BeginEvent)
		);
		assert_eq!(
			event_spec("endEvent").map(|spec| spec.kind),
			Some(KnownEvent::EndEvent)
		);
		assert_eq!(
			event_spec("repeatEvent").map(|spec| spec.kind),
			Some(KnownEvent::RepeatEvent)
		);
		assert_eq!(event_spec("beginevent"), None);
		assert_eq!(event_spec("endevent"), None);
		assert_eq!(event_spec("repeatevent"), None);
	}

	#[test]
	fn known_events_round_trip_the_full_catalog() {
		for spec in EVENT_SPECS {
			assert_eq!(spec.kind.as_str(), spec.dom_name);
			assert_eq!(KnownEvent::from_str(spec.dom_name), Ok(spec.kind));
			assert_eq!(spec.kind.spec(), spec);
		}
	}

	#[test]
	fn custom_event_names_preserve_borrowed_and_owned_names() {
		let borrowed = EventName::Custom(Cow::Borrowed("item-selected"));
		let owned = EventName::Custom(Cow::Owned(String::from("editor:commit")));
		let known = EventName::from(KnownEvent::Input);

		assert_eq!(borrowed.as_str(), "item-selected");
		assert_eq!(owned.as_str(), "editor:commit");
		assert_eq!(known.as_str(), "input");
		assert_eq!(known.known(), Some(KnownEvent::Input));
		assert_eq!(borrowed.known(), None);
	}

	#[test]
	fn interfaces_and_capabilities_are_valid() {
		for spec in EVENT_SPECS {
			let unique_fallbacks = spec.fallback_interfaces.iter().collect::<HashSet<_>>();
			let unique_capabilities = spec.capabilities.iter().collect::<HashSet<_>>();
			assert_eq!(unique_fallbacks.len(), spec.fallback_interfaces.len());
			assert_eq!(unique_capabilities.len(), spec.capabilities.len());
			assert!(!spec.fallback_interfaces.contains(&spec.primary_interface));
			if !spec.capabilities.is_empty() {
				assert!(matches!(
					spec.primary_interface,
					EventInterface::Generic | EventInterface::Input
				));
			}
			assert!(spec.capabilities.iter().all(|capability| matches!(
				capability,
				EventCapability::Value
					| EventCapability::Checked
					| EventCapability::SelectedValues
					| EventCapability::Files
			)));
		}

		let input = event_spec("input").expect("input must be cataloged");
		assert_eq!(input.primary_interface, EventInterface::Input);
		assert_eq!(input.fallback_interfaces, &[EventInterface::Generic]);
		assert_eq!(
			input.capabilities,
			&[
				EventCapability::Value,
				EventCapability::Checked,
				EventCapability::SelectedValues,
				EventCapability::Files,
			]
		);
	}

	#[test]
	fn behavior_defaults_cover_bubbling_cancellation_and_composition() {
		assert_eq!(
			event_spec("click").map(|spec| spec.behavior),
			Some(EventBehavior::new(true, true, true))
		);
		assert_eq!(
			event_spec("focus").map(|spec| spec.behavior),
			Some(EventBehavior::new(false, false, true))
		);
		assert_eq!(
			event_spec("scroll").map(|spec| spec.behavior),
			Some(EventBehavior::new(false, false, false))
		);
		assert_eq!(
			event_spec("submit").map(|spec| spec.behavior),
			Some(EventBehavior::new(true, true, false))
		);
	}

	#[test]
	fn non_cancelable_composition_and_raw_pointer_updates_match_the_standards() {
		let cancelable = ["compositionupdate", "compositionend", "pointerrawupdate"].map(|name| {
			event_spec(name)
				.expect("event must be cataloged")
				.behavior
				.cancelable
		});

		assert_eq!(cancelable, [false, false, false]);
	}

	#[test]
	fn required_specialized_interfaces_are_cataloged() {
		assert_eq!(
			event_spec("encrypted").map(|spec| spec.primary_interface),
			Some(EventInterface::MediaEncrypted)
		);
		assert_eq!(
			event_spec("waitingforkey").map(|spec| spec.primary_interface),
			Some(EventInterface::Generic)
		);
		assert_eq!(
			event_spec("enterpictureinpicture").map(|spec| spec.primary_interface),
			Some(EventInterface::PictureInPicture)
		);
		assert_eq!(
			event_spec("leavepictureinpicture").map(|spec| spec.primary_interface),
			Some(EventInterface::PictureInPicture)
		);
	}
}
