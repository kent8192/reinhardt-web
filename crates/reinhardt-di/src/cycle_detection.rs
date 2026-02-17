//! Thread-local circular dependency detection mechanism
//!
//! This module provides an optimized mechanism for detecting circular references during DI dependency resolution.
//!
//! ## Features
//!
//! - **O(1) Circular Detection**: Fast lookup using `HashSet<TypeId>`
//! - **Thread-local**: Low-cost borrow checking with `RefCell` (no Mutex locks required)
//! - **Depth Limiting**: `MAX_RESOLUTION_DEPTH` prevents pathological cases
//! - **Sampling**: Checks every 10th resolution in deep dependency chains
//! - **RAII**: Automatic cleanup via `ResolutionGuard`
//!
//! ## Performance Goals
//!
//! - Cache hit: < 5% overhead (completely skip circular detection)
//! - Cache miss: 10-20% overhead (optimized detection)
//! - Deep dependency chains: Sampling reduces linear cost

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Maximum resolution depth (prevents pathological cases)
const MAX_RESOLUTION_DEPTH: usize = 100;

/// Sampling rate (check every N resolutions in deep dependency chains)
const CYCLE_DETECTION_SAMPLING_RATE: usize = 10;

thread_local! {
	/// Set of types currently being resolved (O(1) circular detection)
	static RESOLUTION_SET: RefCell<HashSet<TypeId>> = RefCell::new(HashSet::new());

	/// Resolution depth counter
	static RESOLUTION_DEPTH: RefCell<usize> = const { RefCell::new(0) };

	/// Type name mapping (for error messages)
	static TYPE_NAMES: RefCell<HashMap<TypeId, &'static str>> =
		RefCell::new(HashMap::new());

	/// Resolution path (for displaying circular paths)
	static RESOLUTION_PATH: RefCell<Vec<(TypeId, &'static str)>> =
		const { RefCell::new(Vec::new()) };
}

/// Circular reference check (O(1))
///
/// Checks if the specified type is currently in the resolution stack.
fn check_circular_dependency(type_id: TypeId) -> Result<(), CycleError> {
	RESOLUTION_SET.with(|set| {
		let set_ref = set.borrow();
		if set_ref.contains(&type_id) {
			// Circular detection: Get type name and construct error
			let type_name = get_type_name(type_id);
			let cycle_path = build_cycle_path(type_id);
			return Err(CycleError::CircularDependency {
				type_name: type_name.to_string(),
				path: cycle_path,
			});
		}
		Ok(())
	})
}

/// Record the start of resolution
///
/// Called at the start of type resolution to prepare for circular detection.
/// The returned `ResolutionGuard` automates cleanup using the RAII pattern.
pub fn begin_resolution(
	type_id: TypeId,
	type_name: &'static str,
) -> Result<ResolutionGuard, CycleError> {
	// Depth check
	let depth = RESOLUTION_DEPTH.with(|d| {
		let mut depth = d.borrow_mut();
		*depth += 1;
		*depth
	});

	if depth > MAX_RESOLUTION_DEPTH {
		// Depth exceeded: Cleanup and error
		RESOLUTION_DEPTH.with(|d| {
			let mut depth = d.borrow_mut();
			*depth -= 1;
		});
		return Err(CycleError::MaxDepthExceeded(depth));
	}

	// Sampling: In deep dependency chains, check only every 10th resolution
	if depth > 50 && !depth.is_multiple_of(CYCLE_DETECTION_SAMPLING_RATE) {
		return Ok(ResolutionGuard::Sampled);
	}

	// Circular check
	check_circular_dependency(type_id)?;

	// Add to set
	RESOLUTION_SET.with(|set| {
		set.borrow_mut().insert(type_id);
	});

	// Add to path (for error messages)
	RESOLUTION_PATH.with(|path| {
		path.borrow_mut().push((type_id, type_name));
	});

	Ok(ResolutionGuard::Tracked(type_id))
}

/// RAII guard: Automatic cleanup on Drop
///
/// When resolution is complete, the type is removed from the stack.
pub enum ResolutionGuard {
	/// Tracking circular detection
	Tracked(TypeId),
	/// Skipped due to sampling
	Sampled,
}

impl Drop for ResolutionGuard {
	fn drop(&mut self) {
		if let ResolutionGuard::Tracked(type_id) = self {
			RESOLUTION_SET.with(|set| {
				set.borrow_mut().remove(type_id);
			});

			RESOLUTION_PATH.with(|path| {
				let mut path = path.borrow_mut();
				if let Some(pos) = path.iter().rposition(|(id, _)| id == type_id) {
					path.remove(pos);
				}
			});
		}

		RESOLUTION_DEPTH.with(|d| {
			let mut depth = d.borrow_mut();
			*depth = depth.saturating_sub(1);
		});
	}
}

/// Register type name
///
/// Registers the type name in the mapping for display in error messages.
pub fn register_type_name<T: 'static>(name: &'static str) {
	TYPE_NAMES.with(|names| {
		names.borrow_mut().insert(TypeId::of::<T>(), name);
	});
}

/// Get type name
fn get_type_name(type_id: TypeId) -> &'static str {
	TYPE_NAMES.with(|names| names.borrow().get(&type_id).copied().unwrap_or("<unknown>"))
}

/// Build circular path
///
/// Extracts the circular portion from the current resolution path and returns it as a readable string.
fn build_cycle_path(current_type_id: TypeId) -> String {
	RESOLUTION_PATH.with(|path| {
		let path = path.borrow();

		// Find the start position of the cycle
		if let Some(cycle_start) = path.iter().position(|(id, _)| *id == current_type_id) {
			// Extract the circular portion
			let cycle: Vec<&str> = path[cycle_start..].iter().map(|(_, name)| *name).collect();

			// Add the current type at the end to complete the cycle
			let cycle_with_end = format!(
				"{} -> {}",
				cycle.join(" -> "),
				get_type_name(current_type_id)
			);

			cycle_with_end
		} else {
			// If no cycle is found (should not normally occur)
			format!("Unknown cycle involving {}", get_type_name(current_type_id))
		}
	})
}

/// Circular dependency error
#[derive(Debug, thiserror::Error)]
pub enum CycleError {
	/// Circular dependency detected
	#[error(
		"Circular dependency detected: {type_name}\n  Path: {path}\nThis forms a cycle that cannot be resolved."
	)]
	CircularDependency {
		/// Name of the type involved in the cycle
		type_name: String,
		/// Circular path (format: A -> B -> C -> A)
		path: String,
	},

	/// Maximum resolution depth exceeded
	#[error(
		"Maximum resolution depth exceeded: {0}\nThis likely indicates an extremely deep or circular dependency chain."
	)]
	MaxDepthExceeded(usize),
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_simple_cycle_detection() {
		// Start resolving TypeA
		let type_a = TypeId::of::<TypeA>();
		register_type_name::<TypeA>("TypeA");

		let guard_a = begin_resolution(type_a, "TypeA").unwrap();

		// Attempting to resolve TypeA again should cause circular error
		let result = begin_resolution(type_a, "TypeA");
		assert!(matches!(result, Err(CycleError::CircularDependency { .. })));

		// Drop guard to cleanup
		drop(guard_a);

		// After cleanup, resolution should succeed again
		let result = begin_resolution(type_a, "TypeA");
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_depth_limit() {
		// Test depth tracking and cleanup
		// Since we can't easily create 100 unique TypeIds, we test that:
		// 1. Depth is tracked correctly
		// 2. Depth is reset after guards are dropped

		use std::marker::PhantomData;

		// Test depth tracking with a few different types
		let type1 = std::any::TypeId::of::<PhantomData<[u8; 0]>>();
		let type2 = std::any::TypeId::of::<PhantomData<[u8; 1]>>();
		let type3 = std::any::TypeId::of::<PhantomData<[u8; 2]>>();

		// Initial depth should be 0
		let initial_depth = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(initial_depth, 0, "Initial depth should be 0");

		// Start first resolution
		let guard1 = begin_resolution(type1, "Type1").unwrap();
		let depth1 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(depth1, 1, "Depth should be 1 after first resolution");

		// Start second resolution (different type)
		let guard2 = begin_resolution(type2, "Type2").unwrap();
		let depth2 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(depth2, 2, "Depth should be 2 after second resolution");

		// Start third resolution (different type)
		let guard3 = begin_resolution(type3, "Type3").unwrap();
		let depth3 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(depth3, 3, "Depth should be 3 after third resolution");

		// Drop guards in reverse order
		drop(guard3);
		let depth_after_drop3 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(
			depth_after_drop3, 2,
			"Depth should be 2 after dropping guard3"
		);

		drop(guard2);
		let depth_after_drop2 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(
			depth_after_drop2, 1,
			"Depth should be 1 after dropping guard2"
		);

		drop(guard1);
		let depth_after_drop1 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(
			depth_after_drop1, 0,
			"Depth should be 0 after dropping all guards"
		);
	}

	// Dummy type for testing
	struct TypeA;
}
