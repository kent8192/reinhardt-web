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
pub enum WasmRebuildOutcome {
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
pub struct WasmRebuildPipeline;

impl WasmRebuildPipeline {
	/// Run the WASM builder once and capture the outcome with timing.
	///
	/// The underlying builder is synchronous, so we offload it onto a blocking
	/// task to avoid stalling the tokio runtime that drives the watcher.
	pub async fn run(ctx: &CommandContext) -> WasmRebuildOutcome {
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
	/// the caller can suppress logging entirely. The summary line never
	/// embeds the error string — `WasmBuildError` variants such as
	/// `CargoBuildFailed(String)` carry multi-line cargo stderr, which would
	/// break the "single greppable summary line" contract callers rely on.
	/// Use [`WasmRebuildPipeline::detail_lines`] to retrieve the error
	/// detail when rendering a `Failed` outcome.
	pub fn format_log_line(outcome: &WasmRebuildOutcome) -> Option<String> {
		match outcome {
			WasmRebuildOutcome::Ok { duration } => Some(format!(
				"[hot-reload] WASM rebuild OK (took {})",
				format_duration(*duration)
			)),
			WasmRebuildOutcome::Failed { duration, .. } => Some(format!(
				"[hot-reload] WASM rebuild FAILED (took {}):",
				format_duration(*duration)
			)),
			WasmRebuildOutcome::Skipped => None,
		}
	}

	/// Return the per-line detail to print after the summary.
	///
	/// For `Failed`, returns the error rendered via `Display`, split into
	/// individual lines so the caller can indent each one. For `Ok` and
	/// `Skipped`, returns an empty `Vec`.
	pub fn detail_lines(outcome: &WasmRebuildOutcome) -> Vec<String> {
		match outcome {
			WasmRebuildOutcome::Failed { error, .. } => format!("{}", error)
				.split('\n')
				.map(|s| s.to_string())
				.collect(),
			WasmRebuildOutcome::Ok { .. } | WasmRebuildOutcome::Skipped => Vec::new(),
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
	use rstest::rstest;

	#[rstest]
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

	#[rstest]
	fn format_log_line_failed_is_single_line_summary_only() {
		// Arrange: a multi-line cargo stderr embedded in the error must
		// not leak into the summary line.
		let outcome = WasmRebuildOutcome::Failed {
			duration: Duration::from_millis(2300),
			error: WasmBuildError::CargoBuildFailed(
				"error[E0308]: mismatched types\n  --> src/lib.rs:3:5\nline three".to_string(),
			),
		};

		// Act
		let line = WasmRebuildPipeline::format_log_line(&outcome)
			.expect("Failed outcome must produce a log line");

		// Assert
		assert_eq!(
			line, "[hot-reload] WASM rebuild FAILED (took 2.3s):",
			"summary must be a single greppable line with no embedded error"
		);
		assert!(!line.contains('\n'), "summary must not contain newlines");
	}

	#[rstest]
	fn format_log_line_skipped_returns_none() {
		// Arrange
		let outcome = WasmRebuildOutcome::Skipped;

		// Act
		let line = WasmRebuildPipeline::format_log_line(&outcome);

		// Assert
		assert!(line.is_none(), "Skipped must not produce a log line");
	}

	#[rstest]
	fn detail_lines_failed_splits_multiline_error_into_lines() {
		// Arrange
		let outcome = WasmRebuildOutcome::Failed {
			duration: Duration::from_millis(800),
			error: WasmBuildError::CargoBuildFailed("first\nsecond\nthird".to_string()),
		};

		// Act
		let detail = WasmRebuildPipeline::detail_lines(&outcome);

		// Assert: one entry per source line, none containing newlines.
		assert_eq!(detail.len(), 3, "expected 3 detail lines, got {detail:?}");
		for line in &detail {
			assert!(!line.contains('\n'), "detail line must not contain '\\n'");
		}
	}

	#[rstest]
	fn detail_lines_ok_and_skipped_are_empty() {
		// Arrange
		let ok = WasmRebuildOutcome::Ok {
			duration: Duration::from_millis(100),
		};
		let skipped = WasmRebuildOutcome::Skipped;

		// Act
		let ok_detail = WasmRebuildPipeline::detail_lines(&ok);
		let skipped_detail = WasmRebuildPipeline::detail_lines(&skipped);

		// Assert
		assert!(ok_detail.is_empty(), "Ok must have no detail lines");
		assert!(
			skipped_detail.is_empty(),
			"Skipped must have no detail lines"
		);
	}
}
