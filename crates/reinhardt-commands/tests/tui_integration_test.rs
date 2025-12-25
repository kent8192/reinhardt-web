//! Integration tests for TUI module
//!
//! Tests basic functionality of the TUI module components.

#[cfg(feature = "server")]
use reinhardt_commands::tui::{AppState, LogBuffer, LogEntry, LogLevel, LogSource, Pane};

#[cfg(feature = "server")]
#[test]
fn test_app_state_initialization() {
	let state = AppState::new();
	assert_eq!(state.active_pane, Pane::Backend);
	assert_eq!(state.filter_level, LogLevel::Trace);
	assert!(!state.should_quit);
	assert_eq!(state.backend_logs.len(), 0);
	assert_eq!(state.frontend_logs.len(), 0);
}

#[cfg(feature = "server")]
#[test]
fn test_log_entry_routing() {
	let mut state = AppState::new();

	// Add backend log
	state.add_log(LogEntry::new(
		LogSource::Backend,
		"Backend message".to_string(),
	));

	// Add frontend log
	state.add_log(LogEntry::new(
		LogSource::Frontend,
		"Frontend message".to_string(),
	));

	// Verify logs are routed to correct buffers
	assert_eq!(state.backend_logs.len(), 1);
	assert_eq!(state.frontend_logs.len(), 1);
}

#[cfg(feature = "server")]
#[test]
fn test_pane_switching() {
	let mut state = AppState::new();
	assert_eq!(state.active_pane, Pane::Backend);

	// Simulate Tab key press
	use crossterm::event::KeyCode;
	state.handle_key(KeyCode::Tab);
	assert_eq!(state.active_pane, Pane::Frontend);

	state.handle_key(KeyCode::Tab);
	assert_eq!(state.active_pane, Pane::Backend);
}

#[cfg(feature = "server")]
#[test]
fn test_quit_handling() {
	let mut state = AppState::new();
	assert!(!state.should_quit);

	// Simulate 'q' key press
	use crossterm::event::KeyCode;
	state.handle_key(KeyCode::Char('q'));
	assert!(state.should_quit);
}

#[cfg(feature = "server")]
#[test]
fn test_log_buffer_overflow() {
	let mut buffer = LogBuffer::new(5);

	// Add 10 logs (should keep only last 5)
	for i in 0..10 {
		buffer.push(LogEntry::new(LogSource::Backend, format!("Message {}", i)));
	}

	assert_eq!(buffer.len(), 5);

	// First message should be "Message 5" (0-4 were removed)
	let first = buffer.entries().next().unwrap();
	assert!(first.message.contains("Message 5"));
}

#[cfg(feature = "server")]
#[test]
fn test_log_buffer_clear() {
	let mut buffer = LogBuffer::new(100);

	buffer.push(LogEntry::new(
		LogSource::Backend,
		"Test message".to_string(),
	));
	assert_eq!(buffer.len(), 1);

	buffer.clear();
	assert_eq!(buffer.len(), 0);
	assert_eq!(buffer.scroll_offset(), 0);
}

#[cfg(feature = "server")]
#[test]
fn test_clear_active_pane() {
	let mut state = AppState::new();

	// Add logs to both panes
	state.add_log(LogEntry::new(LogSource::Backend, "Backend log".to_string()));
	state.add_log(LogEntry::new(
		LogSource::Frontend,
		"Frontend log".to_string(),
	));

	assert_eq!(state.backend_logs.len(), 1);
	assert_eq!(state.frontend_logs.len(), 1);

	// Clear backend (active pane)
	use crossterm::event::KeyCode;
	state.handle_key(KeyCode::Char('c'));

	assert_eq!(state.backend_logs.len(), 0);
	assert_eq!(state.frontend_logs.len(), 1); // Frontend unchanged

	// Switch to frontend and clear
	state.handle_key(KeyCode::Tab);
	state.handle_key(KeyCode::Char('c'));

	assert_eq!(state.frontend_logs.len(), 0);
}
