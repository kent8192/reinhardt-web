use super::log_buffer::{LogBuffer, LogEntry, LogLevel, LogSource};
use super::metrics::ServerMetrics;
use crossterm::event::KeyCode;
use std::io;
use tokio::sync::mpsc;

/// Active pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
	Backend,
	Frontend,
}

/// TUI application state (testable logic part)
pub struct AppState {
	pub backend_logs: LogBuffer,
	pub frontend_logs: LogBuffer,
	pub active_pane: Pane,
	pub filter_level: LogLevel,
	pub should_quit: bool,
	pub metrics: ServerMetrics,
}

impl AppState {
	/// Create a new AppState
	pub fn new() -> Self {
		const MAX_LOG_LINES: usize = 10000;
		Self {
			backend_logs: LogBuffer::new(MAX_LOG_LINES),
			frontend_logs: LogBuffer::new(MAX_LOG_LINES),
			active_pane: Pane::Backend,
			filter_level: LogLevel::Trace, // Show all
			should_quit: false,
			metrics: ServerMetrics::default(),
		}
	}

	/// Update metrics
	pub fn update_metrics(&mut self, metrics: ServerMetrics) {
		self.metrics = metrics;
	}

	/// Handle key input
	pub fn handle_key(&mut self, key: KeyCode) {
		match key {
			KeyCode::Char('q') => {
				self.should_quit = true;
			}
			KeyCode::Tab => {
				// Switch active pane between backend and frontend
				self.active_pane = match self.active_pane {
					Pane::Backend => Pane::Frontend,
					Pane::Frontend => Pane::Backend,
				};
			}
			KeyCode::Char('f') => {
				// Cycle through log filter levels (ALL → WARN+ → ERROR → ALL)
				self.filter_level = self.filter_level.cycle_filter();
			}
			KeyCode::Char('c') => {
				// Clear active pane logs
				match self.active_pane {
					Pane::Backend => self.backend_logs.clear(),
					Pane::Frontend => self.frontend_logs.clear(),
				}
			}
			KeyCode::Up => {
				// Scroll up through log entries
				match self.active_pane {
					Pane::Backend => self.backend_logs.scroll_up(1),
					Pane::Frontend => self.frontend_logs.scroll_up(1),
				}
			}
			KeyCode::Down => {
				// Scroll down through log entries
				match self.active_pane {
					Pane::Backend => self.backend_logs.scroll_down(1),
					Pane::Frontend => self.frontend_logs.scroll_down(1),
				}
			}
			KeyCode::PageUp => {
				// Scroll up by page (10 lines)
				match self.active_pane {
					Pane::Backend => self.backend_logs.scroll_up(10),
					Pane::Frontend => self.frontend_logs.scroll_up(10),
				}
			}
			KeyCode::PageDown => {
				// Scroll down by page (10 lines)
				match self.active_pane {
					Pane::Backend => self.backend_logs.scroll_down(10),
					Pane::Frontend => self.frontend_logs.scroll_down(10),
				}
			}
			KeyCode::Home => {
				// Scroll to top
				match self.active_pane {
					Pane::Backend => self.backend_logs.scroll_to_top(),
					Pane::Frontend => self.frontend_logs.scroll_to_top(),
				}
			}
			KeyCode::End => {
				// Scroll to bottom
				match self.active_pane {
					Pane::Backend => self.backend_logs.scroll_to_bottom(),
					Pane::Frontend => self.frontend_logs.scroll_to_bottom(),
				}
			}
			_ => {}
		}
	}

	/// Add a log entry
	pub fn add_log(&mut self, entry: LogEntry) {
		match entry.source {
			LogSource::Backend => self.backend_logs.push(entry),
			LogSource::Frontend => self.frontend_logs.push(entry),
		}
	}
}

impl Default for AppState {
	fn default() -> Self {
		Self::new()
	}
}

/// TUI application (includes UI part)
pub struct TuiApp {
	state: AppState,
	log_receiver: mpsc::UnboundedReceiver<LogEntry>,
	metrics_receiver: mpsc::UnboundedReceiver<ServerMetrics>,
}

impl TuiApp {
	/// Create a new TuiApp
	pub fn new(
		log_receiver: mpsc::UnboundedReceiver<LogEntry>,
		metrics_receiver: mpsc::UnboundedReceiver<ServerMetrics>,
	) -> Self {
		Self {
			state: AppState::new(),
			log_receiver,
			metrics_receiver,
		}
	}

	/// Try to initialize TUI
	pub fn try_init() -> io::Result<()> {
		// Check terminal initialization
		crossterm::terminal::enable_raw_mode()?;
		crossterm::terminal::disable_raw_mode()?;
		Ok(())
	}

	/// Get application state (for testing)
	pub fn state(&self) -> &AppState {
		&self.state
	}

	/// Get mutable application state (for testing)
	pub fn state_mut(&mut self) -> &mut AppState {
		&mut self.state
	}

	/// Process received log entries
	pub fn process_logs(&mut self) {
		while let Ok(entry) = self.log_receiver.try_recv() {
			self.state.add_log(entry);
		}
	}

	/// Process received metrics
	pub fn process_metrics(&mut self) {
		while let Ok(metrics) = self.metrics_receiver.try_recv() {
			self.state.update_metrics(metrics);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_app_state_creation() {
		let state = AppState::new();
		assert_eq!(state.active_pane, Pane::Backend);
		assert_eq!(state.filter_level, LogLevel::Trace);
		assert!(!state.should_quit);
	}

	#[test]
	fn test_quit_key() {
		let mut state = AppState::new();
		assert!(!state.should_quit);

		state.handle_key(KeyCode::Char('q'));
		assert!(state.should_quit);
	}

	#[test]
	fn test_pane_switch() {
		let mut state = AppState::new();
		assert_eq!(state.active_pane, Pane::Backend);

		state.handle_key(KeyCode::Tab);
		assert_eq!(state.active_pane, Pane::Frontend);

		state.handle_key(KeyCode::Tab);
		assert_eq!(state.active_pane, Pane::Backend);
	}

	#[test]
	fn test_clear_logs() {
		let mut state = AppState::new();
		state.add_log(LogEntry::new(LogSource::Backend, "Test".to_string()));
		assert_eq!(state.backend_logs.len(), 1);

		state.handle_key(KeyCode::Char('c'));
		assert_eq!(state.backend_logs.len(), 0);
	}

	#[test]
	fn test_add_log() {
		let mut state = AppState::new();

		state.add_log(LogEntry::new(LogSource::Backend, "Backend log".to_string()));
		state.add_log(LogEntry::new(
			LogSource::Frontend,
			"Frontend log".to_string(),
		));

		assert_eq!(state.backend_logs.len(), 1);
		assert_eq!(state.frontend_logs.len(), 1);
	}
}
