//! Owned values shared by typed event payloads on native and WASM targets.

/// Keyboard modifier flags active when an event was dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
	/// Whether the Alt key was active.
	pub alt: bool,
	/// Whether the Control key was active.
	pub control: bool,
	/// Whether the Meta key was active.
	pub meta: bool,
	/// Whether the Shift key was active.
	pub shift: bool,
}

/// A two-dimensional event coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
	/// Horizontal coordinate.
	pub x: f64,
	/// Vertical coordinate.
	pub y: f64,
}

impl Point {
	/// Creates a coordinate pair.
	#[must_use]
	pub const fn new(x: f64, y: f64) -> Self {
		Self { x, y }
	}
}

/// Mouse button whose state changed for an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MouseButton {
	/// No button changed.
	#[default]
	None,
	/// Primary button, normally the left button.
	Primary,
	/// Auxiliary button, normally the middle button.
	Auxiliary,
	/// Secondary button, normally the right button.
	Secondary,
	/// Fourth button, normally the browser back button.
	Fourth,
	/// Fifth button, normally the browser forward button.
	Fifth,
	/// A browser-specific button number.
	Other(i16),
}

impl From<i16> for MouseButton {
	fn from(button: i16) -> Self {
		match button {
			-1 => Self::None,
			0 => Self::Primary,
			1 => Self::Auxiliary,
			2 => Self::Secondary,
			3 => Self::Fourth,
			4 => Self::Fifth,
			other => Self::Other(other),
		}
	}
}

/// Bitmask of mouse buttons pressed while an event was dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MouseButtons(u16);

impl MouseButtons {
	/// No buttons are pressed.
	pub const NONE: Self = Self(0);
	/// Primary button bit.
	pub const PRIMARY: Self = Self(1);
	/// Secondary button bit.
	pub const SECONDARY: Self = Self(2);
	/// Auxiliary button bit.
	pub const AUXILIARY: Self = Self(4);
	/// Fourth button bit.
	pub const FOURTH: Self = Self(8);
	/// Fifth button bit.
	pub const FIFTH: Self = Self(16);

	/// Creates a button mask from DOM `MouseEvent.buttons` bits.
	#[must_use]
	pub const fn from_bits(bits: u16) -> Self {
		Self(bits)
	}

	/// Returns the raw DOM button bits.
	#[must_use]
	pub const fn bits(self) -> u16 {
		self.0
	}

	/// Returns whether all bits in `buttons` are present.
	#[must_use]
	pub const fn contains(self, buttons: Self) -> bool {
		self.0 & buttons.0 == buttons.0
	}
}

/// Pointer device classification.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PointerKind {
	/// Mouse or mouse-compatible pointer.
	#[default]
	Mouse,
	/// Pen or stylus pointer.
	Pen,
	/// Touch contact pointer.
	Touch,
	/// Browser-specific pointer classification.
	Other(String),
}

impl From<String> for PointerKind {
	fn from(kind: String) -> Self {
		match kind.as_str() {
			"" | "mouse" => Self::Mouse,
			"pen" => Self::Pen,
			"touch" => Self::Touch,
			_ => Self::Other(kind),
		}
	}
}

/// Owned metadata for a file selected by an event target.
#[derive(Clone)]
pub struct EventFile {
	name: String,
	media_type: String,
	size: u64,
	last_modified: i64,
	#[cfg(wasm)]
	raw: web_sys::File,
}

impl EventFile {
	/// Returns the file name without a path.
	#[must_use]
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Returns the reported media type.
	#[must_use]
	pub fn media_type(&self) -> &str {
		&self.media_type
	}

	/// Returns the file size in bytes.
	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}

	/// Returns the last-modified timestamp in milliseconds since the Unix epoch.
	#[must_use]
	pub const fn last_modified(&self) -> i64 {
		self.last_modified
	}

	/// Returns the source browser file.
	#[cfg(wasm)]
	#[must_use]
	pub const fn raw(&self) -> &web_sys::File {
		&self.raw
	}

	#[cfg(native)]
	pub(crate) fn from_native(file: &reinhardt_core::types::page::NativeEventFile) -> Self {
		Self {
			name: file.name.clone(),
			media_type: file.media_type.clone(),
			size: file.size,
			last_modified: file.last_modified,
		}
	}

	#[cfg(wasm)]
	pub(crate) fn from_web_file(file: web_sys::File) -> Self {
		Self {
			name: file.name(),
			media_type: file.type_(),
			size: file.size() as u64,
			last_modified: file.last_modified() as i64,
			raw: file,
		}
	}
}

impl std::fmt::Debug for EventFile {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter
			.debug_struct("EventFile")
			.field("name", &self.name)
			.field("media_type", &self.media_type)
			.field("size", &self.size)
			.field("last_modified", &self.last_modified)
			.finish()
	}
}

impl PartialEq for EventFile {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
			&& self.media_type == other.media_type
			&& self.size == other.size
			&& self.last_modified == other.last_modified
	}
}

impl Eq for EventFile {}
