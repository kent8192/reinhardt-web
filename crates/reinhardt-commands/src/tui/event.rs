use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;

/// Event handler
pub struct EventHandler;

impl EventHandler {
	/// Create a new EventHandler
	pub fn new() -> Self {
		Self
	}

	/// Wait for an event and return KeyCode
	/// Returns None after timeout
	pub fn next_key(&self) -> std::io::Result<Option<KeyCode>> {
		// Polling interval (100ms)
		if event::poll(Duration::from_millis(100))?
			&& let Event::Key(KeyEvent {
				code,
				kind: KeyEventKind::Press,
				..
			}) = event::read()?
		{
			return Ok(Some(code));
		}
		Ok(None)
	}
}

impl Default for EventHandler {
	fn default() -> Self {
		Self::new()
	}
}
