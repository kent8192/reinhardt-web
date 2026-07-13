//! Hot-reload server rebuild pipeline.
//!
//! Runs `cargo build --bin <bin>` and, on success, swaps the currently-running
//! server child process for a freshly spawned one. Emits the structured
//! `[hot-reload] ...` log lines the watcher contract requires.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout};

const READINESS_TIMEOUT: Duration = Duration::from_secs(5);
const READINESS_INTERVAL: Duration = Duration::from_millis(50);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

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
		Self::run_for_package(bin_name, None, current_child, respawn).await
	}

	/// Run `cargo build --package <package> --bin <bin_name>` when a package is selected.
	///
	/// Passing `None` preserves the workspace-default cargo invocation used by
	/// callers that did not select a package.
	pub async fn run_for_package(
		bin_name: &str,
		package: Option<&str>,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
	) -> (ServerRebuildOutcome, Option<Child>) {
		Self::run_for_package_with_features(bin_name, package, &[], false, current_child, respawn)
			.await
	}

	/// Run a package-selected native rebuild with the selected Cargo features.
	pub async fn run_for_package_with_features(
		bin_name: &str,
		package: Option<&str>,
		features: &[String],
		all_features: bool,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
	) -> (ServerRebuildOutcome, Option<Child>) {
		Self::run_inner(
			bin_name,
			package,
			features,
			all_features,
			current_child,
			respawn,
			None,
		)
		.await
	}

	/// Run `cargo build --bin <bin_name>`, swap the child, then wait until
	/// the advertised server address accepts TCP connections.
	///
	/// If the new child starts but never becomes reachable, the failure is
	/// reported as `SpawnFailed` while the child is still returned so the
	/// watcher can keep owning and eventually replace or kill it.
	pub async fn run_with_readiness(
		bin_name: &str,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
		address: &str,
	) -> (ServerRebuildOutcome, Option<Child>) {
		Self::run_with_readiness_for_package(bin_name, None, current_child, respawn, address).await
	}

	/// Run a package-selected native rebuild and wait for the respawned server.
	pub async fn run_with_readiness_for_package(
		bin_name: &str,
		package: Option<&str>,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
		address: &str,
	) -> (ServerRebuildOutcome, Option<Child>) {
		Self::run_with_readiness_for_package_with_features(
			bin_name,
			package,
			&[],
			false,
			current_child,
			respawn,
			address,
		)
		.await
	}

	/// Run a feature-selected native rebuild and wait for the respawned server.
	pub async fn run_with_readiness_for_package_with_features(
		bin_name: &str,
		package: Option<&str>,
		features: &[String],
		all_features: bool,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
		address: &str,
	) -> (ServerRebuildOutcome, Option<Child>) {
		let readiness = ServerReadinessProbe::new(address);
		Self::run_inner(
			bin_name,
			package,
			features,
			all_features,
			current_child,
			respawn,
			Some(readiness),
		)
		.await
	}

	async fn run_inner(
		bin_name: &str,
		package: Option<&str>,
		features: &[String],
		all_features: bool,
		current_child: &mut Child,
		respawn: impl FnOnce() -> std::io::Result<Child>,
		readiness: Option<ServerReadinessProbe>,
	) -> (ServerRebuildOutcome, Option<Child>) {
		let start = Instant::now();

		// Phase 1: invoke `cargo build [--package <package>] [feature selection] --bin <bin_name>`.
		let output_result = Command::new("cargo")
			.args(Self::cargo_build_arguments(
				bin_name,
				package,
				features,
				all_features,
			))
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
				if let Some(readiness) = readiness
					&& let Err(e) = readiness.wait_until_ready().await
				{
					let duration = start.elapsed();
					let outcome = ServerRebuildOutcome::SpawnFailed {
						duration,
						message: format!("server did not become reachable: {}", e),
					};
					eprintln!("{}", Self::format_log_line(&outcome));
					eprintln!("[hot-reload] watching for next change...");
					return (outcome, Some(new_child));
				}

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

	fn cargo_build_arguments(
		bin_name: &str,
		package: Option<&str>,
		features: &[String],
		all_features: bool,
	) -> Vec<String> {
		let mut arguments = vec!["build".to_string()];
		if let Some(package) = package {
			arguments.extend(["--package".to_string(), package.to_string()]);
		}
		if all_features {
			arguments.push("--all-features".to_string());
		} else if !features.is_empty() {
			arguments.extend(["--features".to_string(), features.join(",")]);
		}
		arguments.extend(["--bin".to_string(), bin_name.to_string()]);
		arguments
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

struct ServerReadinessProbe {
	address: String,
	timeout: Duration,
	interval: Duration,
	connect_timeout: Duration,
}

impl ServerReadinessProbe {
	fn new(address: &str) -> Self {
		Self {
			address: address.to_string(),
			timeout: READINESS_TIMEOUT,
			interval: READINESS_INTERVAL,
			connect_timeout: CONNECT_TIMEOUT,
		}
	}

	async fn wait_until_ready(&self) -> std::io::Result<()> {
		let addrs = Self::probe_addrs(&self.address)?;
		let deadline = Instant::now() + self.timeout;
		let mut last_error = String::from("no connection attempt made");

		loop {
			for addr in &addrs {
				match timeout(self.connect_timeout, tokio::net::TcpStream::connect(addr)).await {
					Ok(Ok(_stream)) => return Ok(()),
					Ok(Err(e)) => {
						last_error = format!("connect to {} failed: {}", addr, e);
					}
					Err(_) => {
						last_error = format!("connect to {} timed out", addr);
					}
				}
			}

			if Instant::now() >= deadline {
				return Err(std::io::Error::new(
					std::io::ErrorKind::TimedOut,
					format!(
						"{} did not accept connections within {}; {}",
						self.address,
						format_duration(self.timeout),
						last_error
					),
				));
			}

			sleep(self.interval).await;
		}
	}

	fn probe_addrs(address: &str) -> std::io::Result<Vec<SocketAddr>> {
		let addrs: Vec<SocketAddr> = address
			.to_socket_addrs()?
			.map(|addr| {
				if addr.ip().is_unspecified() {
					match addr {
						SocketAddr::V4(addr) => {
							SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), addr.port())
						}
						SocketAddr::V6(addr) => {
							SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), addr.port())
						}
					}
				} else {
					addr
				}
			})
			.collect();

		if addrs.is_empty() {
			return Err(std::io::Error::new(
				std::io::ErrorKind::InvalidInput,
				format!("server address {address:?} did not resolve to any socket addresses"),
			));
		}

		Ok(addrs)
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

	#[test]
	fn cargo_build_arguments_include_selected_package() {
		// Act
		let arguments =
			ServerRebuildPipeline::cargo_build_arguments("manage", Some("web-app"), &[], false);

		// Assert
		assert_eq!(
			arguments,
			vec![
				"build".to_string(),
				"--package".to_string(),
				"web-app".to_string(),
				"--bin".to_string(),
				"manage".to_string(),
			]
		);
	}

	#[test]
	fn cargo_build_arguments_preserve_workspace_default_without_package() {
		// Act
		let arguments = ServerRebuildPipeline::cargo_build_arguments("manage", None, &[], false);

		// Assert
		assert_eq!(
			arguments,
			vec![
				"build".to_string(),
				"--bin".to_string(),
				"manage".to_string()
			]
		);
	}

	#[test]
	fn cargo_build_arguments_include_selected_features() {
		// Arrange
		let features = vec!["theme".to_string(), "tracing".to_string()];

		// Act
		let arguments = ServerRebuildPipeline::cargo_build_arguments(
			"manage",
			Some("web-app"),
			&features,
			false,
		);

		// Assert
		assert_eq!(
			arguments,
			vec![
				"build".to_string(),
				"--package".to_string(),
				"web-app".to_string(),
				"--features".to_string(),
				"theme,tracing".to_string(),
				"--bin".to_string(),
				"manage".to_string(),
			]
		);
	}

	#[test]
	fn cargo_build_arguments_include_all_features() {
		// Act
		let arguments = ServerRebuildPipeline::cargo_build_arguments("manage", None, &[], true);

		// Assert
		assert_eq!(
			arguments,
			vec![
				"build".to_string(),
				"--all-features".to_string(),
				"--bin".to_string(),
				"manage".to_string(),
			]
		);
	}

	#[test]
	fn readiness_probe_rewrites_unspecified_ipv4_to_loopback() {
		// Act
		let addrs = ServerReadinessProbe::probe_addrs("0.0.0.0:8000").unwrap();

		// Assert
		assert_eq!(addrs, vec!["127.0.0.1:8000".parse().unwrap()]);
	}

	#[test]
	fn readiness_probe_rewrites_unspecified_ipv6_to_loopback() {
		// Act
		let addrs = ServerReadinessProbe::probe_addrs("[::]:8000").unwrap();

		// Assert
		assert_eq!(addrs, vec!["[::1]:8000".parse().unwrap()]);
	}
}
