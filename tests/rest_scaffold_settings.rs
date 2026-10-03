//! Resolve the unmodified REST scaffold settings in an isolated consumer.

use reinhardt::commands::{BaseCommand, CommandContext, StartProjectCommand};
use reinhardt::test::fixtures::temp_dir;
use rstest::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const SECRET: &str = "rest-scaffold-settings-test-secret";

#[rstest]
#[tokio::test]
async fn rest_scaffold_resolves_default_and_configured_contacts(temp_dir: TempDir) {
	// Arrange
	let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
	let generated = temp_dir.path().join("generated");
	let mut context = CommandContext::new(vec![
		"rest_settings_probe".into(),
		generated.to_str().expect("generated path is UTF-8").into(),
	]);
	context.set_option("restful".into(), "true".into());
	context.set_option("no-interactive".into(), "true".into());
	StartProjectCommand
		.execute(&context)
		.await
		.expect("REST project should be generated");

	let base_example = fs::read_to_string(generated.join("settings/base.example.toml"))
		.expect("read generated base example");
	let local_example = fs::read_to_string(generated.join("settings/local.example.toml"))
		.expect("read generated local example");
	let configured_local = local_example.replace(
		"# [core]\n# debug = true\n# secret_key = \"uncomment-this-line-and-replace-with-a-long-random-string\"",
		&format!("[core]\ndebug = true\nsecret_key = \"{SECRET}\""),
	);
	let target = temp_dir.path().join("target");

	for (scenario, base, local, configured) in [
		(
			"examples",
			base_example.as_str(),
			configured_local.clone(),
			false,
		),
		(
			"missing_contacts_table",
			"[core]\ndebug = false\n",
			configured_local.clone(),
			false,
		),
		(
			"configured_contacts",
			base_example.as_str(),
			format!(
				"{configured_local}\n[contacts]\nadmins = [{{ name = \"Admin\", email = \"admin@example.com\" }}]\nmanagers = [{{ name = \"Manager\", email = \"manager@example.com\" }}]\n"
			),
			true,
		),
	] {
		let consumer = temp_dir.path().join(scenario);
		fs::create_dir_all(consumer.join("src")).expect("create consumer source directory");
		fs::create_dir_all(consumer.join("settings")).expect("create consumer settings directory");
		fs::copy(
			generated.join("src/config/settings.rs"),
			consumer.join("src/settings.rs"),
		)
		.expect("copy unmodified generated settings module");
		fs::write(consumer.join("settings/base.toml"), base).expect("write base settings");
		fs::write(consumer.join("settings/local.toml"), local).expect("write local settings");
		fs::write(
			consumer.join("Cargo.toml"),
			format!(
				r#"[package]
name = "rest_settings_probe"
version = "0.1.0"
edition = "2024"

[dependencies]
reinhardt = {{ path = {workspace:?}, package = "reinhardt-web", default-features = false, features = ["conf"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"

[dev-dependencies]
rstest = "0.26"

[workspace]
"#
			),
		)
		.expect("write isolated consumer manifest");
		fs::write(
			consumer.join("src/lib.rs"),
			format!(
				r#"mod settings;

#[cfg(test)]
mod tests {{
    use rstest::*;

    #[rstest]
    fn contacts_resolve_for_full_settings_and_shell() {{
        let resolved = super::settings::get_settings()
            .expect("settings sources should load")
            .resolve()
            .expect("all generated fragments should resolve");
        let shell = super::settings::get_shell_settings();

        for settings in [resolved.settings(), &shell] {{
            assert_eq!(settings.core.secret_key, {SECRET:?});
            if {configured} {{
                assert_eq!(settings.contacts.admins.len(), 1);
                assert_eq!(settings.contacts.admins[0].name, "Admin");
                assert_eq!(settings.contacts.admins[0].email, "admin@example.com");
                assert_eq!(settings.contacts.managers.len(), 1);
                assert_eq!(settings.contacts.managers[0].name, "Manager");
                assert_eq!(settings.contacts.managers[0].email, "manager@example.com");
            }} else {{
                assert_eq!(settings.contacts.admins.len(), 0);
                assert_eq!(settings.contacts.managers.len(), 0);
            }}
        }}
    }}
}}
"#
			),
		)
		.expect("write consumer assertions");

		// Act
		let mut command = Command::new(env!("CARGO"));
		command
			.current_dir(temp_dir.path())
			.args(["test", "--offline", "--quiet", "--lib", "--manifest-path"])
			.arg(consumer.join("Cargo.toml"))
			.arg("--target-dir")
			.arg(&target);
		for (key, _) in std::env::vars_os() {
			if key.to_string_lossy().starts_with("REINHARDT_") {
				command.env_remove(key);
			}
		}
		let output = command.output().expect("run generated settings consumer");

		// Assert
		assert!(
			output.status.success(),
			"generated settings failed for {scenario}:\nstdout:\n{}\nstderr:\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
		);
	}
}
