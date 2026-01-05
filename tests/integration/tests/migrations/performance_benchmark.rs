//! Performance Benchmark Tests for Migrations
//!
//! This module contains performance benchmark tests to verify that migrations
//! operations complete within acceptable time limits for large-scale schemas.
//!
//! # Test Categories
//!
//! 1. **Model Detection Performance** - Large number of models
//! 2. **Field Addition Performance** - Many fields at once
//! 3. **Dependency Chain Performance** - Deep dependency resolution
//! 4. **Migration Execution Performance** - Many migrations
//! 5. **Memory Usage** - Resource consumption under load

use reinhardt_migrations::{
	FieldState, FieldType, ModelState, ProjectState, autodetector::MigrationAutodetector,
};
use std::time::{Duration, Instant};

/// Helper function to create a model with specified fields
fn create_model_with_fields(app_name: &str, model_name: &str, field_count: usize) -> ModelState {
	let mut model = ModelState::new(app_name, model_name);

	// Add primary key
	let id_field = FieldState::new("id", FieldType::Integer, false);
	model.add_field(id_field);

	// Add additional fields
	for i in 0..field_count {
		let field = FieldState::new(&format!("field_{}", i), FieldType::Text, true);
		model.add_field(field);
	}

	model
}

/// Helper function to create a simple model with just an ID field
fn create_simple_model(app_name: &str, model_name: &str) -> ModelState {
	let mut model = ModelState::new(app_name, model_name);
	let id_field = FieldState::new("id", FieldType::Integer, false);
	model.add_field(id_field);
	model
}

// =============================================================================
// Model Detection Performance Tests
// =============================================================================

/// Test: Detection performance with 1000 models
///
/// Verifies that the autodetector can process 1000 model definitions
/// within acceptable time limits (target: < 5 seconds).
///
/// # Performance Target
/// - 1000 models detection: < 5 seconds
#[test]
fn test_1000_models_detection() {
	let mut to_state = ProjectState::new();

	// Generate 1000 models
	for i in 0..1000 {
		let model = create_simple_model("testapp", &format!("Model{}", i));
		to_state.add_model(model);
	}

	let from_state = ProjectState::new();
	let autodetector = MigrationAutodetector::new(from_state, to_state.clone());

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify all models were detected
	assert_eq!(
		changes.created_models.len(),
		1000,
		"Should detect 1000 new models"
	);

	// Verify performance target
	assert!(
		elapsed < Duration::from_secs(5),
		"Detection took {:?}, expected < 5 seconds",
		elapsed
	);

	println!("1000 models detection completed in {:?}", elapsed);
}

/// Test: Detection performance with 500 models (medium scale)
///
/// Verifies detection performance at medium scale.
///
/// # Performance Target
/// - 500 models detection: < 2 seconds
#[test]
fn test_500_models_detection() {
	let mut to_state = ProjectState::new();

	// Generate 500 models
	for i in 0..500 {
		let model = create_simple_model("testapp", &format!("Model{}", i));
		to_state.add_model(model);
	}

	let from_state = ProjectState::new();
	let autodetector = MigrationAutodetector::new(from_state, to_state.clone());

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify all models were detected
	assert_eq!(
		changes.created_models.len(),
		500,
		"Should detect 500 new models"
	);

	// Verify performance target
	assert!(
		elapsed < Duration::from_secs(2),
		"Detection took {:?}, expected < 2 seconds",
		elapsed
	);

	println!("500 models detection completed in {:?}", elapsed);
}

// =============================================================================
// Field Addition Performance Tests
// =============================================================================

/// Test: Performance when adding 100 fields to a model
///
/// Verifies that detecting 100 new fields on a single model
/// completes within acceptable time limits.
///
/// # Performance Target
/// - 100 fields detection: < 2 seconds
#[test]
fn test_100_fields_addition_detection() {
	// From state: model with just ID
	let mut from_state = ProjectState::new();
	let simple_model = create_simple_model("testapp", "LargeModel");
	from_state.add_model(simple_model);

	// To state: model with 100 additional fields
	let mut to_state = ProjectState::new();
	let large_model = create_model_with_fields("testapp", "LargeModel", 100);
	to_state.add_model(large_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify fields were detected (100 new fields added)
	assert_eq!(
		changes.added_fields.len(),
		100,
		"Should detect 100 new fields"
	);

	// Verify performance target
	assert!(
		elapsed < Duration::from_secs(2),
		"Field addition detection took {:?}, expected < 2 seconds",
		elapsed
	);

	println!("100 fields addition detection completed in {:?}", elapsed);
}

/// Test: Performance when adding 50 fields across multiple models
///
/// Verifies field addition detection across multiple models.
///
/// # Performance Target
/// - 50 fields across 10 models: < 1 second
#[test]
fn test_50_fields_across_models_detection() {
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create 10 models, each getting 5 new fields
	for i in 0..10 {
		let model_name = format!("Model{}", i);

		// From state: simple model
		let simple_model = create_simple_model("testapp", &model_name);
		from_state.add_model(simple_model);

		// To state: model with 5 additional fields
		let large_model = create_model_with_fields("testapp", &model_name, 5);
		to_state.add_model(large_model);
	}

	let autodetector = MigrationAutodetector::new(from_state, to_state);

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify fields were detected (5 fields × 10 models = 50 fields)
	assert_eq!(
		changes.added_fields.len(),
		50,
		"Should detect 50 new fields across models"
	);

	// Verify performance target
	assert!(
		elapsed < Duration::from_secs(1),
		"Multi-model field detection took {:?}, expected < 1 second",
		elapsed
	);

	println!(
		"50 fields across 10 models detection completed in {:?}",
		elapsed
	);
}

// =============================================================================
// Dependency Chain Performance Tests
// =============================================================================

/// Test: Deep dependency chain resolution (100 levels)
///
/// Verifies that the autodetector can handle deep dependency chains
/// without stack overflow or excessive time consumption.
///
/// # Performance Target
/// - 100-level dependency chain: < 3 seconds
#[test]
fn test_deep_dependency_chain_100_levels() {
	let mut to_state = ProjectState::new();

	// Create a chain of 100 models with dependencies
	// Model0 -> Model1 -> Model2 -> ... -> Model99
	for i in 0..100 {
		let mut model = ModelState::new("testapp", &format!("Model{}", i));

		// Add primary key
		let id_field = FieldState::new("id", FieldType::Integer, false);
		model.add_field(id_field);

		// Add foreign key to previous model (if not first)
		if i > 0 {
			let fk_field = FieldState::new(
				&format!("parent_{}", i - 1),
				FieldType::ForeignKey {
					to_table: format!("testapp_model{}", i - 1),
					to_field: "id".to_string(),
					on_delete: reinhardt_migrations::ForeignKeyAction::Cascade,
				},
				true,
			);
			model.add_field(fk_field);
		}

		to_state.add_model(model);
	}

	let from_state = ProjectState::new();
	let autodetector = MigrationAutodetector::new(from_state, to_state.clone());

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify all models were detected
	assert_eq!(
		changes.created_models.len(),
		100,
		"Should detect 100 models in dependency chain"
	);

	// Verify performance target
	assert!(
		elapsed < Duration::from_secs(3),
		"Deep dependency chain detection took {:?}, expected < 3 seconds",
		elapsed
	);

	println!(
		"100-level dependency chain detection completed in {:?}",
		elapsed
	);
}

/// Test: Wide dependency graph (many references to one model)
///
/// Verifies handling of a star topology where many models reference
/// a central model.
///
/// # Performance Target
/// - 50 models referencing 1 central model: < 2 seconds
#[test]
fn test_wide_dependency_graph() {
	let mut to_state = ProjectState::new();

	// Create central model
	let mut central_model = ModelState::new("testapp", "CentralModel");
	let id_field = FieldState::new("id", FieldType::Integer, false);
	central_model.add_field(id_field);
	to_state.add_model(central_model);

	// Create 50 models that reference the central model
	for i in 0..50 {
		let mut model = ModelState::new("testapp", &format!("Satellite{}", i));

		let id_field = FieldState::new("id", FieldType::Integer, false);
		model.add_field(id_field);

		let fk_field = FieldState::new(
			"central",
			FieldType::ForeignKey {
				to_table: "testapp_centralmodel".to_string(),
				to_field: "id".to_string(),
				on_delete: reinhardt_migrations::ForeignKeyAction::Cascade,
			},
			false,
		);
		model.add_field(fk_field);

		to_state.add_model(model);
	}

	let from_state = ProjectState::new();
	let autodetector = MigrationAutodetector::new(from_state, to_state.clone());

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify all models were detected (1 central + 50 satellites)
	assert_eq!(
		changes.created_models.len(),
		51,
		"Should detect 51 models total"
	);

	// Verify performance target
	assert!(
		elapsed < Duration::from_secs(2),
		"Wide dependency graph detection took {:?}, expected < 2 seconds",
		elapsed
	);

	println!("Wide dependency graph detection completed in {:?}", elapsed);
}

// =============================================================================
// Multiple Migrations Performance Tests
// =============================================================================

/// Test: Incremental change detection performance
///
/// Simulates detecting changes across multiple incremental updates.
///
/// # Performance Target
/// - 100 incremental detections: < 10 seconds total
#[test]
fn test_incremental_change_detection_performance() {
	let mut total_elapsed = Duration::ZERO;

	let mut current_state = ProjectState::new();

	// Perform 100 incremental change detections
	for i in 0..100 {
		// Previous state
		let from_state = current_state.clone();

		// Add one new model
		let model = create_simple_model("testapp", &format!("IncrementalModel{}", i));
		current_state.add_model(model);

		let autodetector = MigrationAutodetector::new(from_state, current_state.clone());

		// Measure detection time
		let start = Instant::now();
		let changes = autodetector.detect_changes();
		let elapsed = start.elapsed();
		total_elapsed += elapsed;

		// Verify exactly one new model detected
		assert_eq!(
			changes.created_models.len(),
			1,
			"Should detect 1 new model in iteration {}",
			i
		);
	}

	// Verify total performance target
	assert!(
		total_elapsed < Duration::from_secs(10),
		"100 incremental detections took {:?}, expected < 10 seconds",
		total_elapsed
	);

	println!(
		"100 incremental change detections completed in {:?}",
		total_elapsed
	);
}

// =============================================================================
// Memory Usage Tests
// =============================================================================

/// Test: Memory efficiency with large model count
///
/// Verifies that memory usage remains reasonable when processing
/// a large number of models. This test focuses on ensuring the
/// autodetector doesn't leak memory or create excessive allocations.
///
/// Note: This test doesn't measure exact memory usage but ensures
/// the operation completes without memory-related failures.
#[test]
fn test_large_scale_memory_efficiency() {
	let mut to_state = ProjectState::new();

	// Create 500 models, each with 10 fields
	for i in 0..500 {
		let model = create_model_with_fields("testapp", &format!("LargeModel{}", i), 10);
		to_state.add_model(model);
	}

	let from_state = ProjectState::new();
	let autodetector = MigrationAutodetector::new(from_state, to_state.clone());

	// Measure detection time
	let start = Instant::now();
	let changes = autodetector.detect_changes();
	let elapsed = start.elapsed();

	// Verify all models were detected
	assert_eq!(
		changes.created_models.len(),
		500,
		"Should detect 500 models"
	);

	// Verify performance (memory-efficient operation should be fast)
	assert!(
		elapsed < Duration::from_secs(10),
		"Large scale detection took {:?}, expected < 10 seconds",
		elapsed
	);

	println!(
		"500 models with 10 fields each - detection completed in {:?}",
		elapsed
	);
}

/// Test: Repeated operations don't accumulate resources
///
/// Verifies that running the autodetector multiple times doesn't
/// cause memory growth or resource leaks.
#[test]
fn test_repeated_operations_no_resource_leak() {
	let mut to_state = ProjectState::new();

	// Create 100 models
	for i in 0..100 {
		let model = create_simple_model("testapp", &format!("Model{}", i));
		to_state.add_model(model);
	}

	let from_state = ProjectState::new();

	// Run detection 10 times
	let mut durations = Vec::new();
	for _ in 0..10 {
		let autodetector = MigrationAutodetector::new(from_state.clone(), to_state.clone());

		let start = Instant::now();
		let changes = autodetector.detect_changes();
		let elapsed = start.elapsed();

		durations.push(elapsed);

		assert_eq!(changes.created_models.len(), 100);
	}

	// Verify that later runs aren't significantly slower (would indicate resource leak)
	let first_duration = durations[0];
	let last_duration = durations[9];

	// Last run shouldn't take more than 3x the first run
	// (some variance is expected, but extreme slowdown indicates problems)
	assert!(
		last_duration < first_duration * 3,
		"Last run ({:?}) significantly slower than first run ({:?}), possible resource leak",
		last_duration,
		first_duration
	);

	println!("Repeated operations completed without resource leak");
	println!(
		"First run: {:?}, Last run: {:?}",
		first_duration, last_duration
	);
}

// =============================================================================
// Scaling Tests
// =============================================================================

/// Test: Linear scaling verification
///
/// Verifies that detection time scales approximately linearly with
/// the number of models.
#[test]
fn test_linear_scaling() {
	let sizes = [100, 200, 400];
	let mut times = Vec::new();

	for &size in &sizes {
		let mut to_state = ProjectState::new();

		for i in 0..size {
			let model = create_simple_model("testapp", &format!("Model{}", i));
			to_state.add_model(model);
		}

		let from_state = ProjectState::new();
		let autodetector = MigrationAutodetector::new(from_state, to_state);

		let start = Instant::now();
		let changes = autodetector.detect_changes();
		let elapsed = start.elapsed();

		assert_eq!(changes.created_models.len(), size);
		times.push((size, elapsed));
	}

	// Log scaling behavior
	println!("Scaling test results:");
	for (size, time) in &times {
		println!("  {} models: {:?}", size, time);
	}

	// Verify roughly linear scaling (4x models shouldn't take more than 8x time)
	let time_100 = times[0].1;
	let time_400 = times[2].1;

	// 400 models (4x) shouldn't take more than 8x the time of 100 models
	// This allows for some overhead while still catching quadratic behavior
	assert!(
		time_400 < time_100 * 8,
		"Scaling appears non-linear: 100 models took {:?}, 400 models took {:?}",
		time_100,
		time_400
	);
}
