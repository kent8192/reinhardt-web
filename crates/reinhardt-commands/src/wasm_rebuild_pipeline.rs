//! Hot-reload WASM rebuild pipeline.
//!
//! Wraps the existing synchronous `Runserver::build_pages_wasm` builder in a
//! timing + structured-logging shell so the debounced watcher can dispatch
//! rebuild requests without managing duration measurement or output formatting
//! itself.

use std::time::{Duration, Instant};

use crate::CommandContext;
use crate::wasm_builder::WasmBuildError;

/// Outcome of a single WASM rebuild attempt triggered by the hot-reload loop.
#[derive(Debug)]
pub(crate) enum WasmRebuildOutcome {
	/// The builder ran successfully.
	Ok {
		/// Wall-clock time the builder took.
		duration: Duration,
	},
	/// The builder returned an error.
	Failed {
		/// Wall-clock time the builder took before failing.
		duration: Duration,
		/// The originating builder error.
		error: WasmBuildError,
	},
	/// The pipeline did not run (e.g. the project does not declare a cdylib
	/// target). No log line is emitted in this case. Reserved for callers
	/// that want to suppress the pipeline at the source level; the watcher
	/// currently only constructs `Ok`/`Failed`.
	#[allow(dead_code)]
	Skipped,
}

/// Stateless pipeline runner. Held as a unit struct so callers can use a
/// consistent type-based entry point (mirroring `ServerRebuildPipeline`).
pub(crate) struct WasmRebuildPipeline;

impl WasmRebuildPipeline {
	/// Run the WASM builder once and capture the outcome with timing.
	///
	/// The underlying builder is synchronous, so we offload it onto a blocking
	/// task to avoid stalling the tokio runtime that drives the watcher.
	pub(crate) async fn run(ctx: &CommandContext) -> WasmRebuildOutcome {
		let start = Instant::now();
		let ctx_clone = ctx.clone();
		let join_result = tokio::task::spawn_blocking(move || {
			crate::builtin::RunServerCommand::build_pages_wasm(&ctx_clone, true)
		})
		.await;

		let duration = start.elapsed();
		match join_result {
			Ok(Ok(())) => WasmRebuildOutcome::Ok { duration },
			Ok(Err(error)) => WasmRebuildOutcome::Failed { duration, error },
			Err(join_err) => WasmRebuildOutcome::Failed {
				duration,
				error: WasmBuildError::Other(format!("WASM rebuild task panicked: {}", join_err)),
			},
		}
	}

	/// Format the single-line summary printed to stderr by the watcher.
	///
	/// Returns `None` when the rebuild was skipped (no cdylib target, etc.) so
	/// the caller can suppress logging entirely. The first line of the output
	/// is the only thing asserted by tests; multi-line decoration (error
	/// detail, "watching for next change..." footer) is appended by the
	/// caller.
	pub(crate) fn format_log_line(outcome: &WasmRebuildOutcome) -> Option<String> {
		match outcome {
			WasmRebuildOutcome::Ok { duration } => Some(format!(
				"[hot-reload] WASM rebuild OK (took {})",
				format_duration(*duration)
			)),
			WasmRebuildOutcome::Failed { duration, error } => Some(format!(
				"[hot-reload] WASM rebuild FAILED (took {}): {}",
				format_duration(*duration),
				error
			)),
			WasmRebuildOutcome::Skipped => None,
		}
	}
}

/// Format a `Duration` as `"{:.1}s"` seconds.
///
/// Centralised so the Server pipeline can reuse the exact same shape later.
fn format_duration(d: Duration) -> String {
	format!("{:.1}s", d.as_secs_f32())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn format_log_line_ok_formats_seconds_with_one_decimal() {
		// Arrange
		let outcome = WasmRebuildOutcome::Ok {
			duration: Duration::from_millis(1234),
		};

		// Act
		let line = WasmRebuildPipeline::format_log_line(&outcome);

		// Assert
		assert_eq!(
			line.as_deref(),
			Some("[hot-reload] WASM rebuild OK (took 1.2s)"),
		);
	}

	#[test]
	fn format_log_line_failed_starts_with_failed_prefix() {
		// Arrange
		let outcome = WasmRebuildOutcome::Failed {
			duration: Duration::from_millis(2300),
			error: WasmBuildError::Other("boom".to_string()),
		};

		// Act
		let line = WasmRebuildPipeline::format_log_line(&outcome)
			.expect("Failed outcome must produce a log line");

		// Assert
		assert!(
			line.starts_with("[hot-reload] WASM rebuild FAILED (took 2.3s):"),
			"unexpected line: {line:?}"
		);
	}

	#[test]
	fn format_log_line_skipped_returns_none() {
		// Arrange
		let outcome = WasmRebuildOutcome::Skipped;

		// Act
		let line = WasmRebuildPipeline::format_log_line(&outcome);

		// Assert
		assert!(line.is_none(), "Skipped must not produce a log line");
	}
}
