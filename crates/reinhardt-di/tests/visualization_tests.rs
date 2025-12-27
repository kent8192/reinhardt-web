//! Tests for dependency graph visualization (dev-tools feature)

#![cfg(feature = "dev-tools")]

use reinhardt_di::visualization::DependencyGraph;
use rstest::*;

/// Test DOT output generation for simple dependency graph
#[rstest]
#[tokio::test]
async fn test_graph_to_dot_simple() {
	let mut graph = DependencyGraph::new();
	graph.add_node("Database", "singleton");
	graph.add_node("UserService", "request");
	graph.add_dependency("UserService", "Database");

	let dot = graph.to_dot();

	assert!(dot.contains("digraph DependencyGraph"));
	assert!(dot.contains("Database"));
	assert!(dot.contains("UserService"));
	assert!(dot.contains("\"UserService\" -> \"Database\""));
	assert!(dot.contains("lightblue")); // singleton color
	assert!(dot.contains("lightgreen")); // request color
}

/// Test DOT output for complex dependency graph with multiple scopes
#[rstest]
#[tokio::test]
async fn test_graph_to_dot_complex() {
	let mut graph = DependencyGraph::new();

	// Add nodes with different scopes
	graph.add_node("Database", "singleton");
	graph.add_node("Cache", "singleton");
	graph.add_node("UserService", "request");
	graph.add_node("AuthService", "request");
	graph.add_node("Logger", "transient");

	// Add dependencies
	graph.add_dependency("UserService", "Database");
	graph.add_dependency("UserService", "Cache");
	graph.add_dependency("AuthService", "Database");
	graph.add_dependency("UserService", "Logger");

	let dot = graph.to_dot();

	// Verify all nodes are present
	assert!(dot.contains("Database"));
	assert!(dot.contains("Cache"));
	assert!(dot.contains("UserService"));
	assert!(dot.contains("AuthService"));
	assert!(dot.contains("Logger"));

	// Verify all edges are present
	assert!(dot.contains("\"UserService\" -> \"Database\""));
	assert!(dot.contains("\"UserService\" -> \"Cache\""));
	assert!(dot.contains("\"AuthService\" -> \"Database\""));
	assert!(dot.contains("\"UserService\" -> \"Logger\""));

	// Verify scope colors
	assert!(dot.contains("lightblue")); // singleton
	assert!(dot.contains("lightgreen")); // request
	assert!(dot.contains("lightyellow")); // transient
}

/// Test adding nodes and edges to the graph
#[rstest]
#[tokio::test]
async fn test_graph_add_node_and_edge() {
	let mut graph = DependencyGraph::new();

	// Add nodes
	graph.add_node("Database", "singleton");
	graph.add_typed_node("cache", "singleton", "Arc<RedisCache>");

	// Add edge
	graph.add_dependency("Service", "Database");
	graph.add_dependency("Service", "cache");

	let stats = graph.statistics();
	assert_eq!(stats.node_count, 2);
	assert_eq!(stats.edge_count, 2);
	assert_eq!(stats.singleton_count, 2);

	let dot = graph.to_dot();
	assert!(dot.contains("Database"));
	assert!(dot.contains("cache"));
	assert!(dot.contains("Arc<RedisCache>"));
}

/// Test scope visualization in the graph
#[rstest]
#[tokio::test]
async fn test_graph_scope_visualization() {
	let mut graph = DependencyGraph::new();

	// Add nodes with different scopes
	graph.add_node("SingletonDB", "singleton");
	graph.add_node("RequestCache", "request");
	graph.add_node("TransientLogger", "transient");

	let stats = graph.statistics();
	assert_eq!(stats.node_count, 3);
	assert_eq!(stats.singleton_count, 1);
	assert_eq!(stats.request_count, 1);
	assert_eq!(stats.transient_count, 1);

	let dot = graph.to_dot();

	// Verify scope-specific coloring
	assert!(dot.contains("SingletonDB"));
	assert!(dot.contains("RequestCache"));
	assert!(dot.contains("TransientLogger"));
	assert!(dot.contains("lightblue")); // singleton
	assert!(dot.contains("lightgreen")); // request
	assert!(dot.contains("lightyellow")); // transient
}

/// Test circular dependency detection and highlighting
#[rstest]
#[tokio::test]
async fn test_graph_circular_dependency_highlight() {
	let mut graph = DependencyGraph::new();

	// Create circular dependency: A -> B -> C -> A
	graph.add_node("ServiceA", "request");
	graph.add_node("ServiceB", "request");
	graph.add_node("ServiceC", "request");
	graph.add_dependency("ServiceA", "ServiceB");
	graph.add_dependency("ServiceB", "ServiceC");
	graph.add_dependency("ServiceC", "ServiceA");

	let cycles = graph.detect_cycles();
	assert!(!cycles.is_empty(), "Should detect circular dependency");

	// Verify cycle contains expected services
	let cycle = &cycles[0];
	assert!(cycle.contains(&"ServiceA".to_string()));
	assert!(cycle.contains(&"ServiceB".to_string()));
	assert!(cycle.contains(&"ServiceC".to_string()));

	// Test with no cycles
	let mut graph_no_cycle = DependencyGraph::new();
	graph_no_cycle.add_node("A", "request");
	graph_no_cycle.add_node("B", "request");
	graph_no_cycle.add_dependency("A", "B");

	let no_cycles = graph_no_cycle.detect_cycles();
	assert!(
		no_cycles.is_empty(),
		"Should not detect cycles in acyclic graph"
	);
}
