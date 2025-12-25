//! TUI (Terminal User Interface) module for runall command
//!
//! Provides a split-pane terminal interface for viewing backend and frontend logs separately.

pub mod app;
pub mod event;
pub mod log_buffer;
pub mod log_writer;
pub mod metrics;
pub mod ui;

// Re-exports for public API
pub use app::{AppState, Pane, TuiApp};
pub use event::EventHandler;
pub use log_buffer::{LogBuffer, LogEntry, LogLevel, LogSource};
pub use log_writer::{LogFileWriter, spawn_log_writer_task};
pub use metrics::{
	MetricsCollector, PidUpdate, ProcessStatus, ServerMetrics, spawn_metrics_collector_task,
};
