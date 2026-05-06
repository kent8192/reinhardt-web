//! Hot-reload server rebuild pipeline.
//!
//! Runs `cargo build --bin <bin>` and, on success, swaps the currently-running
//! server child process for a freshly spawned one. Emits the structured
//! `[hot-reload] ...` log lines the watcher contract requires.

use std::time::{Duration, Instant};

use tokio::process::{Child, Command};

/// Outcome of a single server rebuild attempt triggered by the hot-reload loop.
#[derive(Debug)]
pub enum ServerRebuildOutcome {
	/// Build succeeded and the child was respawned.
	Ok {
		/// Wall-clock time for the entire rebuild + restart.
		duration: Duration,
	},
	/// `cargo build` exited with a non-zero status.
	BuildFailed {
		/// Wall-clock time for the failed build.
		duration: Duration,
		/// Last lines of stderr from the failed build, joined by `\n`.
		// Field is read by the watcher when it forwards diagnostics to a UI
		// channel in a later task; suppress the dead-code warning until then.
		#[allow(dead_code)]
		stderr_tail: String,
	},
	/// Building or respawning the child process failed at the OS level.
	SpawnFailed {
		/// Wall-clock time before the failure surfaced.
		duration: Duration,
		/// Description of the spawn-side failure.
		message: String,
	},
}

/// Stateless pipeline runner. Held as a unit struct so callers have a
/// consistent type-based entry point (mirrors `WasmRebuildPipeline`).
pub struct ServerRebuildPipeline;

impl ServerRebuildPipeline {
	/// Run `cargo build --bin <bin_name>` and, on success, swap the child.
	///
	/// On `BuildFailed` we deliberately leave `current_child` running so the
	/// developer keeps a working server while the source has compile errors.
	pub async fn run(
		bin_name: &str,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
	) -> (ServerRebuildOutcome, Option<Child>) {
		let start = Instant::now();

		// Phase 1: invoke `cargo build --bin <bin_name>`.
		let output_result = Command::new("cargo")
			.args(["build", "--bin", bin_name])
			.output()
			.await;

		let output = match output_result {
			Ok(o) => o,
			Err(e) => {
				let duration = start.elapsed();
				let outcome = ServerRebuildOutcome::SpawnFailed {
					duration,
					message: format!("failed to invoke cargo build: {}", e),
				};
				eprintln!("{}", Self::format_log_line(&outcome));
				eprintln!("[hot-reload] watching for next change...");
				return (outcome, None);
			}
		};

		if !output.status.success() {
			let duration = start.elapsed();
			let stderr = String::from_utf8_lossy(&output.stderr);
			let tail = Self::tail_lines(&stderr, 20);
			let outcome = ServerRebuildOutcome::BuildFailed {
				duration,
				stderr_tail: tail.clone(),
			};
			eprintln!("{}", Self::format_log_line(&outcome));
			if !tail.is_empty() {
				// Indent the stderr tail by two spaces, matching the spec.
				for line in tail.lines() {
					eprintln!("  {}", line);
				}
			}
			eprintln!("[hot-reload] watching for next change...");
			return (outcome, None);
		}

		// Phase 2: kill the old child, await its exit, then respawn.
		if let Err(e) = current_child.kill().await {
			let duration = start.elapsed();
			let outcome = ServerRebuildOutcome::SpawnFailed {
				duration,
				message: format!("failed to kill running server: {}", e),
			};
			eprintln!("{}", Self::format_log_line(&outcome));
			eprintln!("[hot-reload] watching for next change...");
			return (outcome, None);
		}
		// We do not care about the exit status; just ensure the process is reaped.
		let _ = current_child.wait().await;

		match respawn() {
			Ok(new_child) => {
				let duration = start.elapsed();
				let outcome = ServerRebuildOutcome::Ok { duration };
				eprintln!("{}", Self::format_log_line(&outcome));
				(outcome, Some(new_child))
			}
			Err(e) => {
				let duration = start.elapsed();
				let outcome = ServerRebuildOutcome::SpawnFailed {
					duration,
					message: format!("failed to respawn server: {}", e),
				};
				eprintln!("{}", Self::format_log_line(&outcome));
				eprintln!("[hot-reload] watching for next change...");
				(outcome, None)
			}
		}
	}

	/// Format the single-line summary printed to stderr by the watcher.
	pub fn format_log_line(outcome: &ServerRebuildOutcome) -> String {
		match outcome {
			ServerRebuildOutcome::Ok { duration } => format!(
				"[hot-reload] Server rebuild + restart OK (took {})",
				format_duration(*duration)
			),
			ServerRebuildOutcome::BuildFailed { duration, .. } => format!(
				"[hot-reload] Server rebuild FAILED (took {}):",
				format_duration(*duration)
			),
			ServerRebuildOutcome::SpawnFailed { duration, message } => format!(
				"[hot-reload] Server respawn FAILED (took {}): {}",
				format_duration(*duration),
				message
			),
		}
	}

	/// Return the last `n` lines of `stderr` joined by `\n`.
	///
	/// When the input has fewer than `n` lines, all lines are returned.
	pub fn tail_lines(stderr: &str, n: usize) -> String {
		if n == 0 {
			return String::new();
		}
		let lines: Vec<&str> = stderr.split('\n').collect();
		let start = lines.len().saturating_sub(n);
		lines[start..].join("\n")
	}
}

/// Format a `Duration` as `"{:.1}s"` seconds.
fn format_duration(d: Duration) -> String {
	format!("{:.1}s", d.as_secs_f32())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn format_log_line_ok_includes_restart_and_duration() {
		// Arrange
		let outcome = ServerRebuildOutcome::Ok {
			duration: Duration::from_millis(2500),
		};

		// Act
		let line = ServerRebuildPipeline::format_log_line(&outcome);

		// Assert
		assert_eq!(line, "[hot-reload] Server rebuild + restart OK (took 2.5s)");
	}

	#[test]
	fn format_log_line_build_failed_starts_with_failed_prefix() {
		// Arrange
		let outcome = ServerRebuildOutcome::BuildFailed {
			duration: Duration::from_millis(800),
			stderr_tail: "error[E0308]: mismatched types".to_string(),
		};

		// Act
		let line = ServerRebuildPipeline::format_log_line(&outcome);

		// Assert
		assert_eq!(
			line, "[hot-reload] Server rebuild FAILED (took 0.8s):",
			"unexpected line: {line:?}"
		);
	}

	#[test]
	fn tail_lines_returns_last_n_lines() {
		// Arrange
		let stderr = "line1\nline2\nline3\nline4\nline5";

		// Act
		let tail = ServerRebuildPipeline::tail_lines(stderr, 3);

		// Assert
		assert_eq!(tail, "line3\nline4\nline5");
	}

	#[test]
	fn tail_lines_returns_all_when_fewer_than_n() {
		// Arrange
		let stderr = "only-line-1\nonly-line-2";

		// Act
		let tail = ServerRebuildPipeline::tail_lines(stderr, 20);

		// Assert
		assert_eq!(tail, "only-line-1\nonly-line-2");
	}
}
