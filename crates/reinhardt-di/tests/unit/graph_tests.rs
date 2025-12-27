//! Unit tests for DependencyGraph

use reinhardt_di::graph::DependencyGraph;
use reinhardt_di::registry::DependencyRegistry;
use rstest::*;
use std::any::TypeId;
use std::sync::Arc;

// Test type definitions
struct ServiceA;
struct ServiceB;
struct ServiceC;

#[rstest]
fn dependency_graph_new_empty() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());

	// Act
	let graph = DependencyGraph::new(registry);

	// Assert
	let cycles = graph.detect_cycles();
	assert!(cycles.is_empty());
}

#[rstest]
fn add_node_creates_node() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());
	let type_id = TypeId::of::<ServiceA>();

	// Act
	registry.register_type_name(type_id, "ServiceA");
	registry.register_dependencies(type_id, vec![]);

	// Assert
	let type_names = registry.get_type_names();
	assert!(type_names.contains_key(&type_id));
	assert_eq!(type_names.get(&type_id), Some(&"ServiceA"));
}

#[rstest]
fn add_edge_creates_edge() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());
	let type_id_a = TypeId::of::<ServiceA>();
	let type_id_b = TypeId::of::<ServiceB>();

	// Act
	registry.register_dependencies(type_id_a, vec![type_id_b]);

	// Assert
	let deps = registry.get_dependencies(type_id_a);
	assert_eq!(deps.len(), 1);
	assert_eq!(deps[0], type_id_b);
}

#[rstest]
fn topological_sort_returns_correct_order() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());

	// A -> B -> C の依存チェーンを作成
	let type_id_a = TypeId::of::<ServiceA>();
	let type_id_b = TypeId::of::<ServiceB>();
	let type_id_c = TypeId::of::<ServiceC>();

	registry.register_type_name(type_id_a, "ServiceA");
	registry.register_type_name(type_id_b, "ServiceB");
	registry.register_type_name(type_id_c, "ServiceC");

	registry.register_dependencies(type_id_a, vec![type_id_b]);
	registry.register_dependencies(type_id_b, vec![type_id_c]);
	registry.register_dependencies(type_id_c, vec![]);

	let graph = DependencyGraph::new(registry);

	// Act
	let result = graph.topological_sort();

	// Assert
	assert!(result.is_ok());
	let order = result.unwrap();
	assert!(!order.is_empty());
}

#[rstest]
fn detect_cycle_finds_circular_dependency() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());

	// A -> B -> C -> A の循環を作成
	let type_id_a = TypeId::of::<ServiceA>();
	let type_id_b = TypeId::of::<ServiceB>();
	let type_id_c = TypeId::of::<ServiceC>();

	registry.register_type_name(type_id_a, "ServiceA");
	registry.register_type_name(type_id_b, "ServiceB");
	registry.register_type_name(type_id_c, "ServiceC");

	registry.register_dependencies(type_id_a, vec![type_id_b]);
	registry.register_dependencies(type_id_b, vec![type_id_c]);
	registry.register_dependencies(type_id_c, vec![type_id_a]); // 循環

	let graph = DependencyGraph::new(registry);

	// Act
	let cycles = graph.detect_cycles();

	// Assert
	assert!(!cycles.is_empty());
	assert!(cycles[0].len() >= 3);
}

#[rstest]
fn transitive_dependencies_computed() {
	// Arrange
	let registry = Arc::new(DependencyRegistry::new());

	// A -> B -> C の依存チェーンを作成
	let type_id_a = TypeId::of::<ServiceA>();
	let type_id_b = TypeId::of::<ServiceB>();
	let type_id_c = TypeId::of::<ServiceC>();

	registry.register_type_name(type_id_a, "ServiceA");
	registry.register_type_name(type_id_b, "ServiceB");
	registry.register_type_name(type_id_c, "ServiceC");

	registry.register_dependencies(type_id_a, vec![type_id_b]);
	registry.register_dependencies(type_id_b, vec![type_id_c]);
	registry.register_dependencies(type_id_c, vec![]);

	let graph = DependencyGraph::new(registry);

	// Act
	let tree = graph.build_tree(type_id_a);

	// Assert
	assert!(tree.is_some());
	let tree = tree.unwrap();
	assert_eq!(tree.type_id, type_id_a);
	assert!(!tree.dependencies.is_empty());
}
