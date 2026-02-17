//! Property-based tests using proptest
//!
//! Tests that verify invariants hold for all inputs.

use proptest::prelude::*;
use reinhardt_commands::{CommandContext, CommandRegistry, generate_secret_key};
use rstest::rstest;

proptest! {
	#![proptest_config(ProptestConfig::with_cases(100))]

	// =============================================================================
	// CommandContext Property Tests
	// =============================================================================

	/// Property: ctx.arg(i) == ctx.args.get(i)
	///
	/// **Category**: Property-based
	/// **Verifies**: arg() method is consistent with direct access
	#[rstest]
	fn prop_arg_index_consistency(args in prop::collection::vec(any::<String>(), 0..100)) {
		let ctx = CommandContext::new(args.clone());

		for (i, arg) in args.iter().enumerate() {
			prop_assert_eq!(ctx.arg(i), Some(arg), "Index {} should return correct arg", i);
		}
		prop_assert_eq!(ctx.arg(args.len()), None, "Out of bounds should return None");
	}

	/// Property: ctx.option(key) == ctx.option_values(key).first()
	///
	/// **Category**: Property-based
	/// **Verifies**: option() returns first value from option_values()
	#[rstest]
	fn prop_option_first_matches_values_first(
		key in "[a-z]+",
		values in prop::collection::vec(any::<String>(), 1..10)
	) {
		let mut ctx = CommandContext::new(vec![]);
		ctx.set_option_multi(key.clone(), values.clone());

		let option_result = ctx.option(&key);
		let values_result = ctx.option_values(&key);

		prop_assert!(values_result.is_some());
		let values_vec = values_result.unwrap();
		prop_assert_eq!(option_result, values_vec.first());
	}

	/// Property: has_option returns true iff option exists
	///
	/// **Category**: Property-based
	/// **Verifies**: has_option is consistent with option()
	#[rstest]
	fn prop_has_option_consistency(
		existing_keys in prop::collection::vec("[a-z]+", 0..10),
		query_key in "[a-z]+"
	) {
		let mut ctx = CommandContext::new(vec![]);

		for key in &existing_keys {
			ctx.set_option(key.clone(), "value".to_string());
		}

		let has = ctx.has_option(&query_key);
		let get = ctx.option(&query_key);

		prop_assert_eq!(has, get.is_some());
	}

	/// Property: add_arg increases length by 1
	///
	/// **Category**: Property-based
	/// **Verifies**: add_arg always appends exactly one argument
	#[rstest]
	fn prop_add_arg_increases_length(
		initial in prop::collection::vec(any::<String>(), 0..20),
		new_arg in any::<String>()
	) {
		let mut ctx = CommandContext::new(initial.clone());
		let initial_len = ctx.args.len();

		ctx.add_arg(new_arg);

		prop_assert_eq!(ctx.args.len(), initial_len + 1);
	}

	/// Property: verbosity getter equals setter value
	///
	/// **Category**: Property-based
	/// **Verifies**: set_verbosity and verbosity are consistent
	#[rstest]
	fn prop_verbosity_roundtrip(level in any::<u8>()) {
		let mut ctx = CommandContext::new(vec![]);
		ctx.set_verbosity(level);

		prop_assert_eq!(ctx.verbosity(), level);
	}

	/// Property: set_option overwrites previous value
	///
	/// **Category**: Property-based
	/// **Verifies**: set_option with same key replaces value
	#[rstest]
	fn prop_set_option_overwrites(
		key in "[a-z]+",
		value1 in any::<String>(),
		value2 in any::<String>()
	) {
		let mut ctx = CommandContext::new(vec![]);

		ctx.set_option(key.clone(), value1);
		ctx.set_option(key.clone(), value2.clone());

		prop_assert_eq!(ctx.option(&key), Some(&value2));
	}

	// =============================================================================
	// CommandRegistry Property Tests
	// =============================================================================

	/// Property: Registered command is retrievable by name
	///
	/// **Category**: Property-based
	/// **Verifies**: register -> get returns the command
	#[rstest]
	fn prop_registry_register_get_roundtrip(name in "[a-zA-Z][a-zA-Z0-9]*") {
		use async_trait::async_trait;
		use reinhardt_commands::{BaseCommand, CommandResult};

		struct TestCommand(String);

		#[async_trait]
		impl BaseCommand for TestCommand {
			fn name(&self) -> &str { &self.0 }
			async fn execute(&self, _: &CommandContext) -> CommandResult<()> { Ok(()) }
		}

		let mut registry = CommandRegistry::new();
		registry.register(Box::new(TestCommand(name.clone())));

		let retrieved = registry.get(&name);
		prop_assert!(retrieved.is_some());
		prop_assert_eq!(retrieved.unwrap().name(), name);
	}

	/// Property: List contains all registered names
	///
	/// **Category**: Property-based
	/// **Verifies**: list() returns all registered command names
	#[rstest]
	fn prop_registry_list_contains_all(
		names in prop::collection::hash_set("[a-zA-Z][a-zA-Z0-9]*", 0..20)
	) {
		use async_trait::async_trait;
		use reinhardt_commands::{BaseCommand, CommandResult};

		struct TestCommand(String);

		#[async_trait]
		impl BaseCommand for TestCommand {
			fn name(&self) -> &str { &self.0 }
			async fn execute(&self, _: &CommandContext) -> CommandResult<()> { Ok(()) }
		}

		let mut registry = CommandRegistry::new();

		for name in &names {
			registry.register(Box::new(TestCommand(name.clone())));
		}

		let list = registry.list();
		prop_assert_eq!(list.len(), names.len());

		for name in &names {
			prop_assert!(list.contains(&name.as_str()));
		}
	}

	/// Property: Duplicate registration overwrites
	///
	/// **Category**: Property-based
	/// **Verifies**: Same name registration replaces previous
	#[rstest]
	fn prop_registry_duplicate_overwrites(name in "[a-zA-Z]+") {
		use async_trait::async_trait;
		use reinhardt_commands::{BaseCommand, CommandResult};

		struct TestCommand { name: String, desc: String }

		#[async_trait]
		impl BaseCommand for TestCommand {
			fn name(&self) -> &str { &self.name }
			fn description(&self) -> &str { &self.desc }
			async fn execute(&self, _: &CommandContext) -> CommandResult<()> { Ok(()) }
		}

		let mut registry = CommandRegistry::new();

		registry.register(Box::new(TestCommand { name: name.clone(), desc: "first".to_string() }));
		registry.register(Box::new(TestCommand { name: name.clone(), desc: "second".to_string() }));

		let retrieved = registry.get(&name).unwrap();
		prop_assert_eq!(retrieved.description(), "second");

		// List should still have only one entry
		prop_assert_eq!(registry.list().len(), 1);
	}

	// =============================================================================
	// Utility Function Property Tests
	// =============================================================================

	/// Property: generate_secret_key always returns 50 chars
	///
	/// **Category**: Property-based
	/// **Verifies**: Secret key length is always 50
	#[rstest]
	fn prop_secret_key_length(_dummy in 0..50u32) {
		let key = generate_secret_key();
		prop_assert_eq!(key.len(), 50);
	}

	/// Property: generate_secret_key contains valid characters only
	///
	/// **Category**: Property-based
	/// **Verifies**: All characters in key are from valid set
	#[rstest]
	fn prop_secret_key_valid_chars(_dummy in 0..50u32) {
		let key = generate_secret_key();
		let valid = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+";

		for c in key.chars() {
			prop_assert!(valid.contains(c), "Character '{}' not in valid set", c);
		}
	}

	/// Property: Multiple secret keys are unique (probabilistically)
	///
	/// **Category**: Property-based
	/// **Verifies**: Generated keys are different
	#[rstest]
	fn prop_secret_key_unique(_dummy in 0..20u32) {
		let key1 = generate_secret_key();
		let key2 = generate_secret_key();

		// With 50 chars from ~70 char alphabet, collision is extremely unlikely
		prop_assert_ne!(key1, key2);
	}

	// =============================================================================
	// Clone Property Tests
	// =============================================================================

	/// Property: Cloned context equals original
	///
	/// **Category**: Property-based
	/// **Verifies**: Clone produces equal context
	#[rstest]
	fn prop_context_clone_equals(
		args in prop::collection::vec(any::<String>(), 0..10),
		verbosity in any::<u8>()
	) {
		let mut ctx = CommandContext::new(args.clone());
		ctx.set_verbosity(verbosity);

		let cloned = ctx.clone();

		// Compare by reference to avoid move
		prop_assert_eq!(&cloned.args, &ctx.args);
		prop_assert_eq!(cloned.verbosity(), ctx.verbosity());
		prop_assert_eq!(&cloned.options, &ctx.options);
	}
}

// =============================================================================
// Additional Property Tests (Non-proptest)
// =============================================================================

/// Property: Default context is empty
///
/// **Category**: Property
/// **Verifies**: Default returns minimal context
#[rstest]
fn test_context_default_is_empty() {
	let ctx = CommandContext::default();

	assert!(ctx.args.is_empty());
	assert!(ctx.options.is_empty());
	assert_eq!(ctx.verbosity(), 0);
	assert!(ctx.settings.is_none());
}

/// Property: Registry default equals new
///
/// **Category**: Property
/// **Verifies**: Default and new are equivalent
#[rstest]
fn test_registry_default_equals_new() {
	let default_reg = CommandRegistry::default();
	let new_reg = CommandRegistry::new();

	assert_eq!(default_reg.list().len(), new_reg.list().len());
	assert!(default_reg.list().is_empty());
}
