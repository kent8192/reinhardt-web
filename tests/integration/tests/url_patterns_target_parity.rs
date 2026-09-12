//! Real consumer checks for shared native HTTP and browser URL declarations.
//!
//! Consumer manifests resolve independently of the workspace lockfile and patches.
//! Refresh missing registry entries when their first offline resolution fails.

use reinhardt_test::fixtures::temp_dir;
use rstest::{fixture, rstest};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use tempfile::{NamedTempFile, TempDir};

const WASM: &str = "wasm32-unknown-unknown";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(1200);

struct ConsumerWorkspace {
	root: TempDir,
	repository: PathBuf,
}

#[fixture]
fn consumer_workspace(temp_dir: TempDir) -> ConsumerWorkspace {
	ConsumerWorkspace {
		root: temp_dir,
		repository: Path::new(env!("CARGO_MANIFEST_DIR"))
			.ancestors()
			.nth(2)
			.expect("integration crate is inside tests/")
			.to_path_buf(),
	}
}

#[derive(Clone, Copy, Debug)]
struct Scenario {
	target: Option<&'static str>,
	server: bool,
	client_router: bool,
}

impl Scenario {
	fn features(self, registration: Option<&str>) -> String {
		let mut features = Vec::new();
		if self.server {
			features.push("server");
		}
		if self.client_router {
			features.push("client-router");
		}
		if let Some(registration) = registration {
			features.push(registration);
		}
		features.join(",")
	}
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
	fn drop(&mut self) {
		// Reap even on timeout, panic, or an I/O error in the test harness.
		let _ = self.0.kill();
		let _ = self.0.wait();
	}
}

struct RunOutput {
	status: ExitStatus,
	stdout: String,
	stderr: String,
}

impl RunOutput {
	fn messages(&self) -> impl Iterator<Item = Value> + '_ {
		self.stdout
			.lines()
			.filter_map(|line| serde_json::from_str(line).ok())
	}

	fn diagnostic_text(&self) -> String {
		self.messages()
			.filter(|message| message["reason"] == "compiler-message")
			.filter_map(|message| message["message"]["rendered"].as_str().map(String::from))
			.collect::<Vec<_>>()
			.join("\n")
	}

	fn assert_success(&self, label: &str) {
		let output = self
			.stdout
			.lines()
			.filter(|line| serde_json::from_str::<Value>(line).is_err())
			.collect::<Vec<_>>()
			.join("\n");
		assert_eq!(
			self.status.code(),
			Some(0),
			"{label}\n{}\n{}\n{output}",
			self.diagnostic_text(),
			self.stderr,
		);
	}

	fn primary_errors(&self) -> Vec<(String, u64, u64)> {
		self.messages()
			.filter(|message| message["reason"] == "compiler-message")
			.filter_map(|message| {
				let diagnostic = &message["message"];
				if diagnostic["level"] != "error" {
					return None;
				}
				let span = diagnostic["spans"].as_array()?.iter().find(|span| {
					span["is_primary"] == true
						&& span["file_name"]
							.as_str()
							.is_some_and(|path| path.ends_with("src/lib.rs"))
				})?;
				Some((
					diagnostic["message"].as_str()?.into(),
					span["line_start"].as_u64()?,
					span["column_start"].as_u64()?,
				))
			})
			.collect()
	}

	fn assert_rust_error(&self, code: &str) {
		assert_eq!(self.status.code(), Some(101), "{}", self.stderr);
		let codes: Vec<_> = self
			.messages()
			.filter(|message| message["reason"] == "compiler-message")
			.filter_map(|message| {
				let diagnostic = &message["message"];
				let in_consumer = diagnostic["spans"].as_array()?.iter().any(|span| {
					span["is_primary"] == true
						&& span["file_name"]
							.as_str()
							.is_some_and(|path| path.ends_with("src/lib.rs"))
				});
				in_consumer
					.then(|| diagnostic["code"]["code"].as_str().map(String::from))
					.flatten()
			})
			.collect();
		assert!(
			codes.iter().any(|actual| actual == code),
			"expected {code}: {}\n{}",
			self.diagnostic_text(),
			self.stderr
		);
	}
}

impl ConsumerWorkspace {
	fn source(&self) -> PathBuf {
		self.repository
			.join("tests/integration/tests/fixtures/url_patterns_target_parity")
	}

	fn create_consumer(&self, directory: &str, facade: &str, source: Option<&str>) -> PathBuf {
		let root = self.root.path().join(directory);
		fs::create_dir_all(root.join("src")).unwrap();
		fs::create_dir_all(root.join("native-only/src")).unwrap();
		for entry in fs::read_dir(self.source().join("src")).unwrap() {
			let entry = entry.unwrap();
			let text = fs::read_to_string(entry.path()).unwrap();
			fs::write(
				root.join("src").join(entry.file_name()),
				text.replace("reinhardt::", &format!("{facade}::")),
			)
			.unwrap();
		}
		if let Some(source) = source {
			fs::write(
				root.join("src/lib.rs"),
				source.replace("reinhardt::", &format!("{facade}::")),
			)
			.unwrap();
		}
		for relative in [
			"build.rs",
			"native-only/Cargo.toml",
			"native-only/src/lib.rs",
		] {
			fs::copy(self.source().join(relative), root.join(relative)).unwrap();
		}
		// JSON string escaping is also valid for these TOML string paths.
		let framework_path = serde_json::to_string(&self.repository).unwrap();
		let manifest = format!(
			r#"[package]
name = "url-patterns-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[profile.dev]
debug = 0

[profile.test]
debug = 0

[features]
server = []
client-router = ["{facade}/client-router"]
routes-first = ["client-router"]
url-patterns-first = ["client-router"]

[dependencies]
{facade} = {{ package = "reinhardt-web", path = {framework_path}, default-features = false, features = ["core", "routing"] }}

[target.'cfg(not(all(target_family = "wasm", target_os = "unknown")))'.dependencies]
{facade} = {{ package = "reinhardt-web", path = {framework_path}, default-features = false, features = ["di"] }}
native-only-fixture = {{ path = "native-only" }}
async-trait = "0.1"

[target.'cfg(not(all(target_family = "wasm", target_os = "unknown")))'.dev-dependencies]
rstest = "0.26.1"
tokio = {{ version = "1", features = ["macros", "rt"] }}
serial_test = "3"

[target.'cfg(all(target_family = "wasm", target_os = "unknown"))'.dev-dependencies]
wasm-bindgen-test = "0.3"
"#
		);
		fs::write(root.join("Cargo.toml"), manifest).unwrap();
		root
	}

	fn command(
		&self,
		root: &Path,
		operation: &str,
		scenario: Scenario,
		registration: Option<&str>,
	) -> Command {
		let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
		command
			.current_dir(root)
			.arg(operation)
			.arg("--manifest-path")
			.arg(root.join("Cargo.toml"))
			.env("CARGO_NET_OFFLINE", "true")
			.env("CARGO_TARGET_DIR", self.root.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", self.root.path().join("build"))
			.env("CARGO_INCREMENTAL", "0")
			.env("CARGO_PROFILE_DEV_DEBUG", "0")
			.env("CARGO_PROFILE_TEST_DEBUG", "0")
			.env_remove("CARGO_ENCODED_RUSTFLAGS")
			.env_remove("RUSTFLAGS");
		if let Some(target) = scenario.target {
			command.arg("--target").arg(target);
		}
		let features = scenario.features(registration);
		if !features.is_empty() {
			command.arg("--features").arg(features);
		}
		command
	}

	fn run(&self, command: &mut Command) -> RunOutput {
		let output = self.run_once(command);
		let resolution_failed = !output.status.success()
			&& (output.stderr.contains("no matching package named")
				|| output.stderr.contains("failed to download")
				|| output.stderr.contains("attempting to make an HTTP request")
				|| output.stderr.contains("candidate versions found which didn't match"));
		if resolution_failed {
			self.run_once(command.env("CARGO_NET_OFFLINE", "false"))
		} else {
			output
		}
	}

	fn run_once(&self, command: &mut Command) -> RunOutput {
		let stdout = NamedTempFile::new_in(self.root.path()).unwrap();
		let stderr = NamedTempFile::new_in(self.root.path()).unwrap();
		command
			.stdin(Stdio::null())
			.stdout(stdout.as_file().try_clone().unwrap())
			.stderr(stderr.as_file().try_clone().unwrap());
		let started = Instant::now();
		eprintln!("Running {command:?}");
		let mut child = ChildGuard(command.spawn().expect("spawn consumer command"));
		let status = loop {
			if let Some(status) = child.0.try_wait().expect("poll consumer command") {
				break status;
			}
			assert!(
				started.elapsed() < COMMAND_TIMEOUT,
				"consumer command exceeded {COMMAND_TIMEOUT:?}:\n{}",
				fs::read_to_string(stderr.path()).unwrap()
			);
			std::thread::sleep(Duration::from_millis(100));
		};
		eprintln!(
			"Consumer command exited {status} after {:.1}s",
			started.elapsed().as_secs_f32()
		);
		let output = RunOutput {
			status,
			stdout: fs::read_to_string(stdout.path()).unwrap(),
			stderr: fs::read_to_string(stderr.path()).unwrap(),
		};
		for line in output
			.stdout
			.lines()
			.filter(|line| line.starts_with("test result:"))
		{
			eprintln!("{line}");
		}
		output
	}

	fn check(&self, root: &Path, scenario: Scenario, registration: Option<&str>) -> RunOutput {
		self.run(
			self.command(root, "check", scenario, registration)
				.args(["--lib", "--message-format=json"]),
		)
	}
}

#[rstest]
fn url_patterns_target_matrix(consumer_workspace: ConsumerWorkspace) {
	// Arrange
	let workspace = consumer_workspace;
	let scenarios = [
		Scenario {
			target: None,
			server: true,
			client_router: false,
		},
		Scenario {
			target: None,
			server: true,
			client_router: true,
		},
		Scenario {
			target: None,
			server: false,
			client_router: false,
		},
		Scenario {
			target: None,
			server: false,
			client_router: true,
		},
		Scenario {
			target: Some(WASM),
			server: false,
			client_router: true,
		},
		Scenario {
			target: Some(WASM),
			server: true,
			client_router: true,
		},
	];
	for facade in ["reinhardt", "framework"] {
		let root = workspace.create_consumer(facade, facade, None);
		for scenario in scenarios {
			// Act
			let output = workspace.check(&root, scenario, None);

			// Assert
			output.assert_success(&format!("{facade}: {scenario:?}"));
			let native_artifact = output.messages().any(|message| {
				message["reason"] == "compiler-artifact"
					&& message["target"]["name"] == "native_only_fixture"
			});
			assert_eq!(
				native_artifact,
				scenario.target.is_none(),
				"native dependency artifact: {facade} {scenario:?}"
			);
			let tree = workspace.run(
				workspace
					.command(&root, "tree", scenario, None)
					.args(["--edges", "normal", "--prefix", "none"]),
			);
			tree.assert_success("consumer target dependency tree");
			assert_eq!(
				tree.stdout
					.lines()
					.any(|line| line.split_whitespace().next() == Some("native-only-fixture")),
				scenario.target.is_none()
			);
			if scenario.target.is_none() {
				workspace
					.run(
						workspace
							.command(&root, "test", scenario, None)
							.arg("--lib"),
					)
					.assert_success("native consumer tests");
			}
		}
		for order in ["routes-first", "url-patterns-first"] {
			for scenario in scenarios
				.into_iter()
				.filter(|scenario| scenario.client_router)
			{
				if scenario.target.is_some() {
					workspace
						.check(&root, scenario, Some(order))
						.assert_success("browser registration attribute order");
				} else {
					workspace
						.run(
							workspace
								.command(&root, "test", scenario, Some(order))
								.arg("--lib"),
						)
						.assert_success("linked native registration attribute order");
				}
			}
		}
	}

	// Negative controls distinguish name erasure from an inert runtime closure.
	let original = fs::read_to_string(workspace.source().join("src/lib.rs")).unwrap();
	let without_attribute = original
		.replace("#[url_patterns]\n", "")
		.replace("use reinhardt::url_patterns;\n", "");
	let control =
		workspace.create_consumer("without-attribute", "reinhardt", Some(&without_attribute));
	workspace
		.check(&control, scenarios[0], None)
		.assert_success("unannotated native server control");
	for scenario in [scenarios[2], scenarios[4]] {
		workspace
			.check(&control, scenario, None)
			.assert_rust_error("E0433");
	}
	let invalid_handler = original.replace(
		".endpoint(crate::native_handlers::health)",
		".endpoint(0u8)",
	);
	let invalid = workspace.create_consumer("invalid-handler", "reinhardt", Some(&invalid_handler));
	workspace
		.check(&invalid, scenarios[0], None)
		.assert_rust_error("E0277");

	for invalid_case in [
		"non_tail_body",
		"unsupported_method",
		"nested_server_builder",
	] {
		let path = workspace
			.repository
			.join("crates/reinhardt-core/macros/tests/ui/url_patterns/fail")
			.join(format!("{invalid_case}.rs"));
		let source = fs::read_to_string(path)
			.unwrap()
			.replace("reinhardt_macros::", "reinhardt::");
		let root = workspace.create_consumer(invalid_case, "reinhardt", Some(&source));
		let baseline = workspace.check(&root, scenarios[0], None);
		assert_eq!(baseline.status.code(), Some(101));
		let expected = baseline.primary_errors();
		assert_ne!(
			expected.len(),
			0,
			"macro must diagnose the malformed consumer"
		);
		for scenario in [scenarios[2], scenarios[4], scenarios[5]] {
			let output = workspace.check(&root, scenario, None);
			assert_eq!(output.status.code(), Some(101));
			assert_eq!(
				output.primary_errors(),
				expected,
				"target-independent diagnostic: {invalid_case} {scenario:?}"
			);
		}
	}
}

#[rstest]
#[ignore = "requires Chrome, chromedriver, and wasm-pack"]
fn url_patterns_browser_runtime(consumer_workspace: ConsumerWorkspace) {
	// Arrange
	let workspace = consumer_workspace;
	for facade in ["reinhardt", "framework"] {
		let root = workspace.create_consumer(facade, facade, None);
		for server in [false, true] {
			for registration in [None, Some("routes-first"), Some("url-patterns-first")] {
				let scenario = Scenario {
					target: Some(WASM),
					server,
					client_router: true,
				};
				let mut command = Command::new("wasm-pack");
				command
					.current_dir(&root)
					.args(["test", "--headless", "--chrome", "--features"])
					.arg(scenario.features(registration))
					.env("CARGO_TARGET_DIR", workspace.root.path().join("target"))
					.env("CARGO_BUILD_BUILD_DIR", workspace.root.path().join("build"))
					.env("CARGO_INCREMENTAL", "0")
					.env("CARGO_PROFILE_DEV_DEBUG", "0")
					.env("CARGO_PROFILE_TEST_DEBUG", "0")
					.env_remove("CARGO_ENCODED_RUSTFLAGS")
					.env_remove("RUSTFLAGS");

				// Act
				let output = workspace.run(&mut command);

				// Assert
				output.assert_success(&format!(
					"browser runtime: {facade} {scenario:?} {registration:?}"
				));
			}
		}
	}
}
