//! Captured consumer launcher context for native management invocations (P0).

use std::env;
use std::path::PathBuf;

/// The Cargo profile used for the replay check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoProfile {
	/// The normal development profile.
	Dev,
	/// Cargo's optimized release profile.
	Release,
	/// A named profile declared by the consumer project.
	Named(String),
}

impl CargoProfile {
	/// Convert Cargo's `PROFILE` value into the replay profile.
	pub fn from_name(name: impl AsRef<str>) -> Self {
		match name.as_ref() {
			"debug" | "dev" => Self::Dev,
			"release" => Self::Release,
			name => Self::Named(name.to_owned()),
		}
	}
}

/// A reason why Cargo configuration cannot be replayed safely.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoReplayUnsupported {
	/// A required launcher value was not emitted.
	MissingContext,
	/// The launcher recorded an unsupported Cargo configuration.
	UnsupportedConfiguration,
}

/// Cargo configuration replay captured by the consumer build script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoConfigReplay {
	/// Every effective build override can be applied exactly.
	Exact {
		/// Effective rustflags supplied by Cargo, when present.
		encoded_rustflags: Option<String>,
		/// Effective Rust compiler wrapper, when present.
		rustc_wrapper: Option<String>,
		/// Effective workspace Rust compiler wrapper, when present.
		rustc_workspace_wrapper: Option<String>,
		/// Effective target linker, when present.
		rustc_linker: Option<String>,
	},
	/// The build used a configuration this command must not guess.
	Unsupported {
		/// Stable reason for the refusal.
		reason: CargoReplayUnsupported,
	},
}

const UNSUPPORTED_CARGO_FLAGS: &[&str] = &[
	"--config",
	"--ignore-rust-version",
	"--locked",
	"--offline",
	"--frozen",
	"--lockfile-path",
];

#[cfg(unix)]
fn cargo_process_command_line() -> Option<String> {
	let pid = std::process::id().to_string();
	let parent = std::process::Command::new("ps")
		.args(["-o", "ppid=", "-p", &pid])
		.output()
		.ok()?;
	if !parent.status.success() {
		return None;
	}
	let parent = String::from_utf8(parent.stdout).ok()?;
	let command = std::process::Command::new("ps")
		.args(["-o", "command=", "-p", parent.trim()])
		.output()
		.ok()?;
	if !command.status.success() {
		return None;
	}
	String::from_utf8(command.stdout).ok()
}

#[cfg(windows)]
fn cargo_process_command_line() -> Option<String> {
	let script = format!(
		"$process = Get-CimInstance Win32_Process -Filter 'ProcessId = {}'; if ($null -ne $process) {{ (Get-CimInstance Win32_Process -Filter \"ProcessId = $($process.ParentProcessId)\").CommandLine }}",
		std::process::id()
	);
	let output = std::process::Command::new("powershell.exe")
		.args(["-NoProfile", "-NonInteractive", "-Command", &script])
		.output()
		.ok()?;
	if !output.status.success() {
		return None;
	}
	String::from_utf8(output.stdout).ok()
}

#[cfg(not(any(unix, windows)))]
fn cargo_process_command_line() -> Option<String> {
	None
}

fn cargo_invocation_has_unsupported_flag() -> Option<bool> {
	let command = cargo_process_command_line()?;
	let program = command.split_whitespace().next()?.trim_matches('"');
	if !(program.ends_with("cargo") || program.ends_with("cargo.exe")) {
		return None;
	}
	Some(command.split_whitespace().any(|argument| {
		UNSUPPORTED_CARGO_FLAGS.iter().any(|flag| {
			argument == *flag
				|| argument
					.strip_prefix(flag)
					.is_some_and(|suffix| suffix.starts_with('='))
		})
	}))
}

/// Compile-time Cargo context supplied by a generated management launcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoCheckContext {
	/// Every feature enabled for the management binary.
	pub enabled_features: Vec<String>,
	/// Target triple used by the consumer build.
	pub target: Option<String>,
	/// Effective Cargo profile.
	pub profile: CargoProfile,
	/// Consumer manifest path.
	pub manifest_path: PathBuf,
	/// Consumer package, when the manifest contains multiple packages.
	pub package: Option<String>,
	/// Management binary name.
	pub binary: Option<String>,
	/// Effective Cargo configuration replay.
	pub config_replay: CargoConfigReplay,
}

impl CargoCheckContext {
	/// Read launcher-provided `REINHARDT_*` values without guessing project data.
	pub fn from_launcher(
		manifest_path: impl Into<PathBuf>,
		package: Option<String>,
		binary: Option<String>,
	) -> Self {
		let enabled_features_value = env::var("REINHARDT_ENABLED_FEATURES").ok();
		let target_explicit = env::var("REINHARDT_TARGET_EXPLICIT")
			.ok()
			.or_else(|| env::var("REINHARDT_TARGET").ok().map(|_| "true".to_owned()));
		let target = (target_explicit.as_deref() == Some("true"))
			.then(|| env::var("REINHARDT_TARGET").ok())
			.flatten();
		let profile = env::var("REINHARDT_PROFILE").ok();
		let replay = match (
			env::var("REINHARDT_CARGO_REPLAY").ok(),
			cargo_invocation_has_unsupported_flag(),
		) {
			(Some(_), Some(true)) => Some("unsupported".to_owned()),
			(Some(_), Some(false)) => Some("exact".to_owned()),
			(replay, None) => replay,
			(None, Some(_)) => None,
		};
		let mut enabled_features: Vec<_> = enabled_features_value
			.clone()
			.unwrap_or_default()
			.split(',')
			.filter(|feature| !feature.is_empty())
			.map(str::to_owned)
			.collect();
		enabled_features.sort();
		enabled_features.dedup();
		let missing_context = enabled_features_value.is_none()
			|| !matches!(target_explicit.as_deref(), Some("true" | "false"))
			|| (target_explicit.as_deref() == Some("true") && target.is_none())
			|| profile.is_none()
			|| replay.is_none();
		let config_replay = match replay.as_deref() {
			Some("unsupported") => CargoConfigReplay::Unsupported {
				reason: CargoReplayUnsupported::UnsupportedConfiguration,
			},
			Some("exact") if !missing_context => CargoConfigReplay::Exact {
				encoded_rustflags: env::var("REINHARDT_ENCODED_RUSTFLAGS").ok(),
				rustc_wrapper: env::var("REINHARDT_RUSTC_WRAPPER").ok(),
				rustc_workspace_wrapper: env::var("REINHARDT_RUSTC_WORKSPACE_WRAPPER").ok(),
				rustc_linker: env::var("REINHARDT_RUSTC_LINKER").ok(),
			},
			_ => CargoConfigReplay::Unsupported {
				reason: CargoReplayUnsupported::MissingContext,
			},
		};
		Self {
			enabled_features,
			target,
			profile: CargoProfile::from_name(profile.unwrap_or_default()),
			manifest_path: manifest_path.into(),
			package,
			binary,
			config_replay,
		}
	}
}
