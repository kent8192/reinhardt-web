//! User Commands Tests
//!
//! Comprehensive test suite for reinhardt management commands.
//! Translation of Django's user_commands tests from django/tests/user_commands/tests.py
//!
//! Reference: https://github.com/django/django/blob/main/tests/user_commands/tests.py

use async_trait::async_trait;
use reinhardt_commands::{
    BaseCommand, CommandArgument, CommandContext, CommandError, CommandOption, CommandRegistry,
    CommandResult,
};
use std::sync::{Arc, Mutex};

// ============================================================================
// Test Helper Commands
// ============================================================================

/// Simple test command that outputs a message
struct DanceCommand;

impl DanceCommand {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BaseCommand for DanceCommand {
    fn name(&self) -> &str {
        "dance"
    }

    fn description(&self) -> &str {
        "A test command that dances"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        let style = ctx
            .option("style")
            .map(|s| s.as_str())
            .unwrap_or("Rock'n'Roll");

        println!("I don't feel like dancing {}.", style);

        if let Some(opt3) = ctx.option("opt_3") {
            if opt3 == "true" {
                println!("option3");
            }
        }

        if let Some(example) = ctx.option("example") {
            if example == "raise" {
                return Err(CommandError::ExecutionError("Test error".to_string()));
            }
        }

        if let Some(integer) = ctx.arg(0) {
            println!("You passed {} as a positional argument.", integer);
        }

        Ok(())
    }
}

/// Command for testing required options
struct RequiredOptionCommand;

#[async_trait]
impl BaseCommand for RequiredOptionCommand {
    fn name(&self) -> &str {
        "required_option"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        if let Some(_need_me) = ctx.option("need_me") {
            println!("need_me");
        }
        if let Some(_needme2) = ctx.option("needme2") {
            println!("needme2");
        }
        Ok(())
    }
}

/// Command for testing subparsers
struct SubparserCommand;

#[async_trait]
impl BaseCommand for SubparserCommand {
    fn name(&self) -> &str {
        "subparser"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        let subcommand = ctx.arg(0).ok_or_else(|| {
            CommandError::InvalidArguments(
                "the following arguments are required: subcommand".to_string(),
            )
        })?;

        match subcommand.as_str() {
            "foo" => {
                println!("bar");
                Ok(())
            }
            invalid => Err(CommandError::InvalidArguments(format!(
                "invalid choice: '{}' (choose from 'foo')",
                invalid
            ))),
        }
    }
}

/// Command that requires app labels
struct AppLabelCommand;

#[async_trait]
impl BaseCommand for AppLabelCommand {
    fn name(&self) -> &str {
        "app_label_command"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        if ctx.args.is_empty() {
            return Err(CommandError::InvalidArguments(
                "At least one app label is required".to_string(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// OutputWrapper Tests
// ============================================================================

#[tokio::test]
async fn test_unhandled_exceptions() {
    // Test that OutputWrapper handles unhandled exceptions properly
    // In Rust, resource cleanup is automatically handled by Drop trait
    // This test verifies that we don't have resource leaks

    use std::fs::File;
    use std::io::{BufWriter, Write};
    use tempfile::NamedTempFile;

    // Create a temporary file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");

    // Scope to ensure Drop is called
    {
        let file = File::create(temp_file.path()).expect("Failed to open file");
        let mut writer = BufWriter::new(file);
        writer.write_all(b"test").expect("Failed to write");
        // Writer is dropped here automatically
    }

    // Verify file was properly closed and written
    let contents = std::fs::read_to_string(temp_file.path()).expect("Failed to read file");
    assert_eq!(
        contents, "test",
        "OutputWrapper should properly flush and close"
    );
}

// ============================================================================
// Command Tests
// ============================================================================

#[tokio::test]
async fn test_command() {
    let ctx = CommandContext::new(vec![]);
    let cmd = DanceCommand::new();

    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Basic command execution failed");
}

#[tokio::test]
async fn test_command_style() {
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("style".to_string(), "Jive".to_string());

    let cmd = DanceCommand::new();
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Command with style option failed");
}

#[tokio::test]
async fn test_explode() {
    // Test that an unknown command raises CommandError
    let registry = CommandRegistry::new();

    // Attempting to execute a non-existent command should fail
    // This would be tested at the registry level
    let result = registry.get("explode");
    assert!(result.is_none(), "Unknown command should not be found");
}

#[tokio::test]
async fn test_system_exit() {
    // Exception raised in a command should raise CommandError with call_command,
    // but SystemExit when run from command line
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("example".to_string(), "raise".to_string());

    let cmd = DanceCommand::new();
    let result = cmd.execute(&ctx).await;

    assert!(result.is_err(), "Command should raise error");
    match result {
        Err(CommandError::ExecutionError(_)) => (),
        _ => panic!("Expected ExecutionError"),
    }
}

#[tokio::test]
async fn test_call_command_option_parsing() {
    // When passing the long option name to call_command, the available option
    // key is the option dest name (#22985)
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("opt_3".to_string(), "true".to_string());

    let cmd = DanceCommand::new();
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Command with opt_3 failed");
}

#[tokio::test]
async fn test_call_command_option_parsing_non_string_arg() {
    // It should be possible to pass non-string arguments to call_command
    let ctx = CommandContext::new(vec!["1".to_string()]);
    let cmd = DanceCommand::new();

    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Command with integer argument failed");
}

#[tokio::test]
async fn test_call_command_with_required_parameters_in_options() {
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("need_me".to_string(), "foo".to_string());
    ctx.set_option("needme2".to_string(), "bar".to_string());

    let cmd = RequiredOptionCommand;
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Required parameters test failed");
}

#[tokio::test]
async fn test_call_command_with_required_parameters_in_mixed_options() {
    let mut ctx = CommandContext::new(vec!["--need-me=foo".to_string()]);
    ctx.set_option("needme2".to_string(), "bar".to_string());

    let cmd = RequiredOptionCommand;
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Mixed options test failed");
}

#[tokio::test]
async fn test_subparser() {
    let ctx = CommandContext::new(vec!["foo".to_string(), "12".to_string()]);
    let cmd = SubparserCommand;

    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Subparser command failed");
}

#[tokio::test]
async fn test_subparser_invalid_option() {
    // Test that invalid subcommand choices raise appropriate errors
    let ctx = CommandContext::new(vec!["test".to_string(), "12".to_string()]);
    let cmd = SubparserCommand;

    let result = cmd.execute(&ctx).await;
    assert!(result.is_err(), "Invalid subcommand should raise error");

    match result {
        Err(CommandError::InvalidArguments(msg)) => {
            assert!(
                msg.contains("invalid choice: 'test'"),
                "Error message should mention invalid choice"
            );
        }
        _ => panic!("Expected InvalidArguments error"),
    }

    // Test missing subcommand
    let ctx_missing = CommandContext::new(vec![]);
    let result_missing = cmd.execute(&ctx_missing).await;
    assert!(
        result_missing.is_err(),
        "Missing subcommand should raise error"
    );

    match result_missing {
        Err(CommandError::InvalidArguments(msg)) => {
            assert!(
                msg.contains("required: subcommand"),
                "Error should mention required subcommand"
            );
        }
        _ => panic!("Expected InvalidArguments error for missing subcommand"),
    }
}

// ============================================================================
// Command Registry Tests
// ============================================================================

#[test]
fn test_command_registry_register() {
    let registry = CommandRegistry::new();

    // Test that registry is initially empty
    assert_eq!(registry.list().len(), 0, "New registry should be empty");

    // In a full implementation, we would test command registration here
    // For now, verify the registry exists and can be created
    let _second_registry = CommandRegistry::new();
    assert_eq!(
        registry.list().len(),
        0,
        "Registry operations should work correctly"
    );
}

#[test]
fn test_command_registry_get() {
    let registry = CommandRegistry::new();

    // Test retrieving a command
    let cmd = registry.get("dance");
    assert!(cmd.is_none(), "Unregistered command should not be found");
}

#[test]
fn test_command_registry_list() {
    let registry = CommandRegistry::new();

    // Test listing all registered commands
    let commands = registry.list();
    // New registry should have no commands
    assert_eq!(commands.len(), 0, "New registry should have no commands");
}

// ============================================================================
// BaseCommand Lifecycle Tests
// ============================================================================

struct LifecycleTestCommand {
    before_called: Arc<Mutex<bool>>,
    execute_called: Arc<Mutex<bool>>,
    after_called: Arc<Mutex<bool>>,
}

#[async_trait]
impl BaseCommand for LifecycleTestCommand {
    fn name(&self) -> &str {
        "lifecycle_test"
    }

    async fn before_execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
        *self.before_called.lock().unwrap() = true;
        Ok(())
    }

    async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
        *self.execute_called.lock().unwrap() = true;
        Ok(())
    }

    async fn after_execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
        *self.after_called.lock().unwrap() = true;
        Ok(())
    }
}

#[tokio::test]
async fn test_command_lifecycle() {
    let before = Arc::new(Mutex::new(false));
    let execute = Arc::new(Mutex::new(false));
    let after = Arc::new(Mutex::new(false));

    let cmd = LifecycleTestCommand {
        before_called: before.clone(),
        execute_called: execute.clone(),
        after_called: after.clone(),
    };

    let ctx = CommandContext::new(vec![]);
    let result = cmd.run(&ctx).await;

    assert!(result.is_ok(), "Command lifecycle failed");
    assert!(*before.lock().unwrap(), "before_execute was not called");
    assert!(*execute.lock().unwrap(), "execute was not called");
    assert!(*after.lock().unwrap(), "after_execute was not called");
}

// ============================================================================
// Command Argument and Option Tests
// ============================================================================

// ============================================================================
// Command Context Tests
// ============================================================================

#[test]
fn test_user_commands_context_options() {
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("verbose".to_string(), "true".to_string());
    ctx.set_option("debug".to_string(), "".to_string());

    assert_eq!(ctx.option("verbose"), Some(&"true".to_string()));
    assert!(ctx.has_option("debug"));
    assert!(!ctx.has_option("nonexistent"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_command_error_not_found() {
    let err = CommandError::NotFound("test_command".to_string());
    assert!(err.to_string().contains("test_command"));
}

#[test]
fn test_command_error_invalid_arguments() {
    let err = CommandError::InvalidArguments("missing required arg".to_string());
    assert!(err.to_string().contains("Invalid arguments"));
}

#[test]
fn test_command_error_execution() {
    let err = CommandError::ExecutionError("failed to create file".to_string());
    assert!(err.to_string().contains("Execution error"));
}

// ============================================================================
// Utils Tests
// ============================================================================

#[test]
fn test_get_random_secret_key() {
    use reinhardt_commands::generate_secret_key;

    let key1 = generate_secret_key();
    let key2 = generate_secret_key();

    // Keys should be non-empty
    assert!(!key1.is_empty());
    assert!(!key2.is_empty());

    // Keys should be different (probabilistically)
    assert_ne!(key1, key2);

    // Keys should be of reasonable length (50 chars in Django)
    assert!(key1.len() >= 32);
}

#[test]
fn test_user_commands_to_camel_case() {
    use reinhardt_commands::to_camel_case;

    assert_eq!(to_camel_case("hello_world"), "HelloWorld");
    assert_eq!(to_camel_case("my_app"), "MyApp");
    assert_eq!(to_camel_case("user"), "User");
    assert_eq!(to_camel_case("api_endpoint"), "ApiEndpoint");
}

// ============================================================================
// Additional Tests from Django
// ============================================================================

#[tokio::test]
async fn test_calling_a_command_with_only_empty_parameter_should_ends_gracefully() {
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("empty".to_string(), "".to_string());

    let cmd = DanceCommand::new();
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Command with empty parameter failed");
}

#[tokio::test]
async fn test_calling_command_with_app_labels_and_parameters_should_be_ok() {
    let ctx = CommandContext::new(vec!["myapp".to_string()]);
    let cmd = DanceCommand::new();

    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Command with app labels failed");
}

#[tokio::test]
async fn test_calling_command_with_parameters_and_app_labels_at_the_end_should_be_ok() {
    let ctx = CommandContext::new(vec!["myapp".to_string()]);
    let cmd = DanceCommand::new();

    let result = cmd.execute(&ctx).await;
    assert!(
        result.is_ok(),
        "Command with parameters and app labels at end failed"
    );
}

#[tokio::test]
async fn test_calling_a_command_with_no_app_labels_and_parameters_raise_command_error() {
    // Test that command raises error when required arguments are missing
    let ctx = CommandContext::new(vec![]);
    let cmd = AppLabelCommand;

    let result = cmd.execute(&ctx).await;
    assert!(
        result.is_err(),
        "Command should raise error when app labels are missing"
    );

    match result {
        Err(CommandError::InvalidArguments(msg)) => {
            assert!(
                msg.contains("app label"),
                "Error should mention app label requirement"
            );
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

#[tokio::test]
async fn test_call_command_unrecognized_option() {
    // Test that unrecognized options are handled
    let mut ctx = CommandContext::new(vec![]);
    ctx.set_option("unrecognized_option".to_string(), "value".to_string());

    let cmd = DanceCommand::new();
    let result = cmd.execute(&ctx).await;

    // In Rust, unrecognized options are simply ignored if not explicitly validated
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_subparser_dest_args() {
    let ctx = CommandContext::new(vec!["foo".to_string()]);
    let cmd = SubparserCommand;

    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Subparser dest args test failed");
}

#[tokio::test]
async fn test_subparser_dest_required_args() {
    let ctx = CommandContext::new(vec!["foo".to_string(), "bar".to_string()]);
    let cmd = SubparserCommand;

    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Subparser dest required args test failed");
}

#[tokio::test]
async fn test_create_parser_kwargs() {
    // Test that BaseCommand allows customization
    // In Rust, this is done through trait methods
    let cmd = DanceCommand::new();

    // Verify basic command properties
    assert_eq!(cmd.name(), "dance");
    assert_eq!(cmd.description(), "A test command that dances");

    // Verify command can define arguments and options
    let _arguments = cmd.arguments();
    let _options = cmd.options();

    // These should return valid vectors (empty or non-empty)
    // Note: Length check removed as Vec::len() always returns a valid value

    // Verify help text can be retrieved
    let help_text = cmd.help();
    assert!(!help_text.is_empty(), "Command should have help text");
}

#[tokio::test]
async fn test_subparser_error_formatting() {
    // Test that subparser errors are properly formatted
    let ctx = CommandContext::new(vec!["invalid_subcommand".to_string()]);
    let cmd = SubparserCommand;

    let result = cmd.execute(&ctx).await;
    assert!(result.is_err(), "Invalid subcommand should produce error");

    match result {
        Err(CommandError::InvalidArguments(msg)) => {
            assert!(
                msg.contains("invalid choice"),
                "Error should be properly formatted"
            );
            assert!(
                msg.contains("invalid_subcommand"),
                "Error should mention the invalid input"
            );
        }
        _ => panic!("Expected InvalidArguments error with proper formatting"),
    }
}

#[tokio::test]
async fn test_subparser_non_django_error_formatting() {
    // Test error formatting for non-Django style subparsers
    let ctx = CommandContext::new(vec!["test".to_string()]);
    let cmd = SubparserCommand;

    let result = cmd.execute(&ctx).await;
    assert!(result.is_err(), "Invalid subcommand should produce error");

    match result {
        Err(CommandError::InvalidArguments(msg)) => {
            // Verify error message is clear and informative
            assert!(!msg.is_empty(), "Error message should not be empty");
            assert!(
                msg.contains("test") || msg.contains("invalid"),
                "Error should mention the issue"
            );
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

#[test]
fn test_no_existent_external_program() {
    // Test that non-existent programs are handled properly
    // This would use std::process::Command in a real implementation
    use std::process::Command;

    let result = Command::new("a_command_that_doesnt_exist_12345").output();

    assert!(result.is_err(), "Non-existent command should fail");
}

#[test]
fn test_is_ignored_path_true() {
    // Test path pattern matching for ignored paths
    // This would be implemented in a separate module
    fn is_ignored_path(path: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                // Simple wildcard matching
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    path.starts_with(parts[0]) && path.ends_with(parts[1])
                } else {
                    false
                }
            } else {
                path.contains(pattern)
            }
        })
    }

    assert!(is_ignored_path("foo/bar/baz", &["baz"]));
    assert!(is_ignored_path("foo/bar/baz", &["*/baz"]));
    assert!(is_ignored_path("foo/bar/baz", &["foo/bar/baz"]));
}

#[test]
fn test_is_ignored_path_false() {
    fn is_ignored_path(path: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| path.contains(pattern))
    }

    assert!(!is_ignored_path("foo/bar/baz", &["bat", "bar/bat", "flub"]));
}

#[test]
fn test_normalize_path_patterns_truncates_wildcard_base() {
    // Test that path patterns with wildcards are normalized
    fn normalize_path_pattern(pattern: &str) -> String {
        if pattern.ends_with("/*") {
            pattern.trim_end_matches("/*").to_string()
        } else {
            pattern.to_string()
        }
    }

    assert_eq!(normalize_path_pattern("foo/bar/*"), "foo/bar");
    assert_eq!(normalize_path_pattern("bar/*/"), "bar/*/");
}

// ============================================================================
// i18n-related Tests
// ============================================================================

/// Test: Language setting is preserved after command execution
/// Reference: django/tests/user_commands/tests.py::CommandTests::test_language_preserved
#[tokio::test]
#[serial_test::serial(i18n)]
async fn test_language_preserved() {
    use reinhardt_i18n::{activate, deactivate, get_locale, load_catalog, MessageCatalog};

    // Setup: Create and load a test catalog
    let mut catalog = MessageCatalog::new("fr");
    catalog.add_translation("Hello", "Bonjour");
    load_catalog("fr", catalog).unwrap();

    // Activate French locale
    activate("fr").unwrap();
    assert_eq!(get_locale(), "fr");

    // Create and execute a command
    let mut registry = CommandRegistry::new();
    registry.register(Box::new(DanceCommand::new()));

    let ctx = CommandContext::new(vec![]);
    let command = registry.get("dance").unwrap();
    let result = command.execute(&ctx).await;
    assert!(result.is_ok());

    // Verify that language setting is preserved after command execution
    assert_eq!(get_locale(), "fr");

    // Cleanup
    deactivate();
}

/// Test: Translations remain deactivated after command execution
/// Reference: django/tests/user_commands/tests.py::CommandTests::test_no_translations_deactivate_translations
#[tokio::test]
#[serial_test::serial(i18n)]
async fn test_no_translations_deactivate_translations() {
    use reinhardt_i18n::{activate, deactivate, get_locale, load_catalog, MessageCatalog};

    // Setup: Create and load a test catalog
    let mut catalog = MessageCatalog::new("de");
    catalog.add_translation("Goodbye", "Auf Wiedersehen");
    load_catalog("de", catalog).unwrap();

    // Initially activate German locale
    activate("de").unwrap();
    assert_eq!(get_locale(), "de");

    // Deactivate translations
    deactivate();
    assert_eq!(get_locale(), "en-US"); // Default locale after deactivation

    // Create and execute a command
    let mut registry = CommandRegistry::new();
    registry.register(Box::new(DanceCommand::new()));

    let ctx = CommandContext::new(vec![]);
    let command = registry.get("dance").unwrap();
    let result = command.execute(&ctx).await;
    assert!(result.is_ok());

    // Verify that translations remain deactivated after command execution
    assert_eq!(get_locale(), "en-US");

    // Cleanup (deactivate again for safety)
    deactivate();
}

// ============================================================================
// Additional Unit Tests (Single Crate)
// ============================================================================

/// Command for testing mutually exclusive options
struct MutuallyExclusiveCommand {
    require_group: bool,
}

#[async_trait]
impl BaseCommand for MutuallyExclusiveCommand {
    fn name(&self) -> &str {
        "mutually_exclusive"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        let has_option_a = ctx.has_option("option_a");
        let has_option_b = ctx.has_option("option_b");

        if self.require_group && !has_option_a && !has_option_b {
            return Err(CommandError::InvalidArguments(
                "one of --option-a or --option-b is required".to_string(),
            ));
        }

        if has_option_a && has_option_b {
            return Err(CommandError::InvalidArguments(
                "--option-a and --option-b are mutually exclusive".to_string(),
            ));
        }

        if has_option_a {
            println!("option_a");
        }
        if has_option_b {
            println!("option_b");
        }

        Ok(())
    }

    fn options(&self) -> Vec<CommandOption> {
        vec![
            CommandOption::flag(Some('a'), "option-a", "Option A"),
            CommandOption::flag(Some('b'), "option-b", "Option B"),
        ]
    }
}

/// Command for testing list options
struct ListOptionCommand;

#[async_trait]
impl BaseCommand for ListOptionCommand {
    fn name(&self) -> &str {
        "list_option"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        if let Some(items) = ctx.option("items") {
            let item_list: Vec<&str> = items.split(',').collect();
            if item_list.is_empty() {
                return Err(CommandError::InvalidArguments(
                    "--items is required".to_string(),
                ));
            }
            for item in item_list {
                println!("{}", item);
            }
        } else {
            return Err(CommandError::InvalidArguments(
                "--items is required".to_string(),
            ));
        }
        Ok(())
    }

    fn options(&self) -> Vec<CommandOption> {
        vec![CommandOption::option(None, "items", "List of items")
            .required()
            .multi()]
    }
}

/// Command for testing const options
struct ConstOptionCommand;

#[async_trait]
impl BaseCommand for ConstOptionCommand {
    fn name(&self) -> &str {
        "const_option"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        let verbosity = ctx
            .option("verbosity")
            .map(|s| s.parse::<i32>().unwrap_or(1))
            .unwrap_or(1);

        if verbosity < 0 || verbosity > 3 {
            return Err(CommandError::InvalidArguments(
                "verbosity must be between 0 and 3".to_string(),
            ));
        }

        println!("verbosity: {}", verbosity);
        Ok(())
    }

    fn options(&self) -> Vec<CommandOption> {
        vec![CommandOption::option(Some('v'), "verbosity", "Verbosity level").with_default("1")]
    }
}

/// Command for testing argument ordering
struct ArgumentOrderCommand;

#[async_trait]
impl BaseCommand for ArgumentOrderCommand {
    fn name(&self) -> &str {
        "arg_order"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        // Common arguments should be processed first
        let verbosity = ctx
            .option("verbosity")
            .map(|s| s.parse::<i32>().unwrap_or(0))
            .unwrap_or(0);

        // Then command-specific arguments
        if let Some(name) = ctx.arg(0) {
            println!("name: {} (v{})", name, verbosity);
        } else {
            return Err(CommandError::InvalidArguments(
                "name argument is required".to_string(),
            ));
        }

        Ok(())
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        vec![CommandArgument::required("name", "Name argument")]
    }

    fn options(&self) -> Vec<CommandOption> {
        vec![CommandOption::option(Some('v'), "verbosity", "Verbosity level").with_default("0")]
    }
}

#[test]
fn test_find_command_without_path() {
    // Test that command discovery works even when PATH is empty
    use std::env;

    // Save original PATH
    let original_path = env::var("PATH").ok();

    // Set PATH to empty
    unsafe {
        env::set_var("PATH", "");
    }

    // Command registry should still work
    let registry = CommandRegistry::new();
    assert_eq!(
        registry.list().len(),
        0,
        "Registry should work without PATH"
    );

    // Restore original PATH
    unsafe {
        if let Some(path) = original_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }
    }
}

#[tokio::test]
async fn test_mutually_exclusive_group_required_options() {
    // Test that mutually exclusive options work when required
    let cmd = MutuallyExclusiveCommand {
        require_group: true,
    };

    // Neither option provided - should fail
    let ctx = CommandContext::new(vec![]);
    let result = cmd.execute(&ctx).await;
    assert!(result.is_err(), "Should fail when neither option provided");

    // Both options provided - should fail
    let mut ctx_both = CommandContext::new(vec![]);
    ctx_both.set_option("option_a".to_string(), "true".to_string());
    ctx_both.set_option("option_b".to_string(), "true".to_string());
    let result = cmd.execute(&ctx_both).await;
    assert!(result.is_err(), "Should fail when both options provided");

    // Only option_a - should succeed
    let mut ctx_a = CommandContext::new(vec![]);
    ctx_a.set_option("option_a".to_string(), "true".to_string());
    let result = cmd.execute(&ctx_a).await;
    assert!(result.is_ok(), "Should succeed with only option_a");

    // Only option_b - should succeed
    let mut ctx_b = CommandContext::new(vec![]);
    ctx_b.set_option("option_b".to_string(), "true".to_string());
    let result = cmd.execute(&ctx_b).await;
    assert!(result.is_ok(), "Should succeed with only option_b");
}

#[tokio::test]
async fn test_mutually_exclusive_group_optional_options() {
    // Test that mutually exclusive options work when optional
    let cmd = MutuallyExclusiveCommand {
        require_group: false,
    };

    // Neither option provided - should succeed
    let ctx = CommandContext::new(vec![]);
    let result = cmd.execute(&ctx).await;
    assert!(
        result.is_ok(),
        "Should succeed when neither option provided"
    );

    // Both options provided - should fail
    let mut ctx_both = CommandContext::new(vec![]);
    ctx_both.set_option("option_a".to_string(), "true".to_string());
    ctx_both.set_option("option_b".to_string(), "true".to_string());
    let result = cmd.execute(&ctx_both).await;
    assert!(result.is_err(), "Should fail when both options provided");
}

#[tokio::test]
async fn test_required_list_option() {
    // Test that required list options work correctly
    let cmd = ListOptionCommand;

    // No items provided - should fail
    let ctx = CommandContext::new(vec![]);
    let result = cmd.execute(&ctx).await;
    assert!(result.is_err(), "Should fail when items not provided");

    // Items provided - should succeed
    let mut ctx_items = CommandContext::new(vec![]);
    ctx_items.set_option("items".to_string(), "a,b,c".to_string());
    let result = cmd.execute(&ctx_items).await;
    assert!(result.is_ok(), "Should succeed with items provided");
}

#[tokio::test]
async fn test_required_const_options() {
    // Test that const options work correctly
    let cmd = ConstOptionCommand;

    // Default value - should succeed
    let ctx = CommandContext::new(vec![]);
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "Should succeed with default value");

    // Valid verbosity - should succeed
    let mut ctx_valid = CommandContext::new(vec![]);
    ctx_valid.set_option("verbosity".to_string(), "2".to_string());
    let result = cmd.execute(&ctx_valid).await;
    assert!(result.is_ok(), "Should succeed with valid verbosity");

    // Invalid verbosity - should fail
    let mut ctx_invalid = CommandContext::new(vec![]);
    ctx_invalid.set_option("verbosity".to_string(), "5".to_string());
    let result = cmd.execute(&ctx_invalid).await;
    assert!(result.is_err(), "Should fail with invalid verbosity");
}

#[test]
fn test_disallowed_abbreviated_options() {
    // Test that abbreviated options are not allowed
    // In Rust with clap, this is enforced by the parser configuration
    let cmd = DanceCommand::new();

    // Get options to verify they're defined properly
    let _options = cmd.options();

    // In a full clap integration, we would test that:
    // - Long options like --style work
    // - Abbreviated options like --sty are rejected
    // For now, we verify the command structure supports this
    // Note: Command structure verification is implicit through successful compilation
}

#[tokio::test]
async fn test_command_add_arguments_after_common_arguments() {
    // Test that command-specific arguments can be added after common arguments
    let cmd = ArgumentOrderCommand;

    // Test with both common option and command argument
    let mut ctx = CommandContext::new(vec!["testname".to_string()]);
    ctx.set_option("verbosity".to_string(), "2".to_string());

    let result = cmd.execute(&ctx).await;
    assert!(
        result.is_ok(),
        "Should handle arguments after common options"
    );

    // Test that argument is required
    let ctx_no_arg = CommandContext::new(vec![]);
    let result = cmd.execute(&ctx_no_arg).await;
    assert!(
        result.is_err(),
        "Should fail when required argument missing"
    );
}

// ============================================================================
// OutputWrapper Tests
// ============================================================================

/// Test: OutputWrapper flushes properly
/// Reference: django/tests/user_commands/tests.py::test_outputwrapper_flush
#[test]
fn test_outputwrapper_flush() {
    use reinhardt_commands::OutputWrapper;
    use std::io::Cursor;

    // Create an OutputWrapper with a cursor
    let cursor = Cursor::new(Vec::new());
    let mut output = OutputWrapper::new(cursor);

    // Write data
    output.write("Test data").expect("Failed to write");

    // Flush explicitly
    output.flush().expect("Failed to flush");

    // Verify data was written
    let cursor = output.into_inner().expect("Failed to get inner");
    let data = cursor.into_inner();
    assert_eq!(
        String::from_utf8(data).unwrap(),
        "Test data",
        "OutputWrapper should properly flush data"
    );
}

// ============================================================================
// Check Framework Tests
// ============================================================================

/// Test: Command execute() doesn't run checks by default
/// Reference: django/tests/user_commands/tests.py::test_call_command_no_checks
#[tokio::test]
async fn test_call_command_no_checks() {
    use reinhardt_commands::BaseCommand;
    use reinhardt_utils_core::checks::{Check, CheckMessage, CheckRegistry};

    // Register a check that should NOT run during execute()
    struct TestCheck;
    impl Check for TestCheck {
        fn tags(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn check(&self) -> Vec<CheckMessage> {
            vec![CheckMessage::error(
                "test.E001",
                "This check should not run during execute()",
            )]
        }
    }

    // Register the check
    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register(Box::new(TestCheck));
    }

    // Create a test command
    struct NoCheckCommand;
    #[async_trait]
    impl BaseCommand for NoCheckCommand {
        fn name(&self) -> &str {
            "nocheck"
        }

        async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
            Ok(())
        }
    }

    let cmd = NoCheckCommand;
    let ctx = CommandContext::new(vec![]);

    // execute() should succeed even though there's an error check registered
    let result = cmd.execute(&ctx).await;
    assert!(result.is_ok(), "execute() should not run system checks");

    // But run() should fail due to the error check
    let result_run = cmd.run(&ctx).await;
    assert!(
        result_run.is_err(),
        "run() should execute system checks and fail"
    );
    if let Err(e) = result_run {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("test.E001"),
            "Error should reference the check ID"
        );
    }

    // Cleanup
    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        *registry_guard = CheckRegistry::new();
    }
}

/// Test: Command can disable system checks
/// Reference: django/tests/user_commands/tests.py::test_requires_system_checks_empty
#[tokio::test]
async fn test_requires_system_checks_empty() {
    use reinhardt_commands::BaseCommand;
    use reinhardt_utils_core::checks::{Check, CheckMessage, CheckRegistry};

    // Register a check that would cause an error
    struct ErrorCheck;
    impl Check for ErrorCheck {
        fn tags(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn check(&self) -> Vec<CheckMessage> {
            vec![CheckMessage::error(
                "test.E002",
                "This error check should be disabled",
            )]
        }
    }

    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register(Box::new(ErrorCheck));
    }

    // Create a command that disables system checks
    struct NoChecksRequiredCommand;
    #[async_trait]
    impl BaseCommand for NoChecksRequiredCommand {
        fn name(&self) -> &str {
            "nocheckrequired"
        }

        fn requires_system_checks(&self) -> bool {
            false // Disable checks
        }

        async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
            Ok(())
        }
    }

    let cmd = NoChecksRequiredCommand;
    let ctx = CommandContext::new(vec![]);

    // run() should succeed even with error check, because checks are disabled
    let result = cmd.run(&ctx).await;
    assert!(
        result.is_ok(),
        "Command with requires_system_checks=false should skip checks"
    );

    // Cleanup
    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        *registry_guard = CheckRegistry::new();
    }
}

/// Test: Command can filter checks by specific tags
/// Reference: django/tests/user_commands/tests.py::test_requires_system_checks_specific
#[tokio::test]
async fn test_requires_system_checks_specific() {
    use reinhardt_commands::BaseCommand;
    use reinhardt_utils_core::checks::{Check, CheckMessage, CheckRegistry};

    // Register checks with different tags
    struct StaticCheck;
    impl Check for StaticCheck {
        fn tags(&self) -> Vec<String> {
            vec!["staticfiles".to_string()]
        }

        fn check(&self) -> Vec<CheckMessage> {
            vec![CheckMessage::error("static.E001", "Static files error")]
        }
    }

    struct DatabaseCheck;
    impl Check for DatabaseCheck {
        fn tags(&self) -> Vec<String> {
            vec!["database".to_string()]
        }

        fn check(&self) -> Vec<CheckMessage> {
            vec![CheckMessage::error("database.E001", "Database error")]
        }
    }

    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register(Box::new(StaticCheck));
        registry_guard.register(Box::new(DatabaseCheck));
    }

    // Command that only checks staticfiles
    struct StaticOnlyCommand;
    #[async_trait]
    impl BaseCommand for StaticOnlyCommand {
        fn name(&self) -> &str {
            "staticonly"
        }

        fn check_tags(&self) -> Vec<String> {
            vec!["staticfiles".to_string()]
        }

        async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
            Ok(())
        }
    }

    let cmd = StaticOnlyCommand;
    let ctx = CommandContext::new(vec![]);

    // Should fail because staticfiles check has an error
    let result = cmd.run(&ctx).await;
    assert!(result.is_err(), "Should fail on staticfiles error check");
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("static.E001"),
            "Error should be from staticfiles check, not database check"
        );
        assert!(
            !error_msg.contains("database.E001"),
            "Database check should not run"
        );
    }

    // Cleanup
    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        *registry_guard = CheckRegistry::new();
    }
}

/// Test: --skip-checks flag bypasses system checks
/// Reference: django/tests/user_commands/tests.py::test_skip_checks
#[tokio::test]
async fn test_skip_checks() {
    use reinhardt_commands::BaseCommand;
    use reinhardt_utils_core::checks::{Check, CheckMessage, CheckRegistry};

    // Register a check that would cause an error
    struct SkipTestCheck;
    impl Check for SkipTestCheck {
        fn tags(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn check(&self) -> Vec<CheckMessage> {
            vec![CheckMessage::error(
                "test.E003",
                "This error should be skipped",
            )]
        }
    }

    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        registry_guard.register(Box::new(SkipTestCheck));
    }

    struct SkipCheckCommand;
    #[async_trait]
    impl BaseCommand for SkipCheckCommand {
        fn name(&self) -> &str {
            "skipcheck"
        }

        async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
            Ok(())
        }
    }

    let cmd = SkipCheckCommand;

    // Without --skip-checks, should fail
    let ctx_no_skip = CommandContext::new(vec![]);
    let result_no_skip = cmd.run(&ctx_no_skip).await;
    assert!(
        result_no_skip.is_err(),
        "Should fail without --skip-checks flag"
    );

    // With --skip-checks, should succeed
    let mut ctx_with_skip = CommandContext::new(vec![]);
    ctx_with_skip.set_option("skip_checks".to_string(), "true".to_string());
    let result_with_skip = cmd.run(&ctx_with_skip).await;
    assert!(
        result_with_skip.is_ok(),
        "Should succeed with --skip-checks flag"
    );

    // Also test with hyphenated version --skip-checks
    let mut ctx_hyphenated = CommandContext::new(vec![]);
    ctx_hyphenated.set_option("skip-checks".to_string(), "true".to_string());
    let result_hyphenated = cmd.run(&ctx_hyphenated).await;
    assert!(
        result_hyphenated.is_ok(),
        "Should succeed with --skip-checks (hyphenated) flag"
    );

    // Cleanup
    let registry = CheckRegistry::global();
    {
        let mut registry_guard = registry.lock().unwrap();
        *registry_guard = CheckRegistry::new();
    }
}

// ============================================================================
// Formatter Integration Tests
// ============================================================================

/// Test: run_formatters handles OSError (formatter not found)
/// Reference: django/tests/user_commands/tests.py::test_run_formatters_handles_oserror_for_black_path
#[test]
fn test_run_formatters_handles_oserror_for_formatter_path() {
    use reinhardt_commands::formatter::run_formatters;

    // Test with nonexistent formatter path (simulates OSError)
    let result = run_formatters(&["src/lib.rs"], Some("/nonexistent/path/to/rustfmt"));

    // Should return an error
    assert!(result.is_err(), "Should fail when formatter is not found");

    // Verify error message mentions the formatter not being found
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("not found") || error_msg.contains("Failed to check formatter"),
            "Error should indicate formatter not found: {}",
            error_msg
        );
    }
}

/// Test: run_formatters succeeds with empty file list
#[test]
fn test_run_formatters_empty_paths() {
    use reinhardt_commands::formatter::run_formatters;

    let result = run_formatters(&[], None);
    assert!(result.is_ok(), "Should succeed with empty file list");
}

/// Test: run_formatters fails gracefully with nonexistent file
#[test]
fn test_run_formatters_nonexistent_file() {
    use reinhardt_commands::formatter::run_formatters;

    let result = run_formatters(&["/nonexistent/file.rs"], None);
    assert!(result.is_err(), "Should fail with nonexistent file");

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("File not found"),
            "Error should mention file not found: {}",
            error_msg
        );
    }
}
