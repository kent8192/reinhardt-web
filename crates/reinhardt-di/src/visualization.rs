//! Dependency graph visualization for development and debugging
//!
//! This module provides tools to visualize dependency injection graphs in DOT format,
//! which can be rendered using Graphviz.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_di::visualization::{DependencyGraph, GraphNode};
//!
//! let mut graph = DependencyGraph::new();
//! graph.add_node("Database", "singleton");
//! graph.add_node("UserService", "request");
//! graph.add_dependency("UserService", "Database");
//!
//! let dot = graph.to_dot();
//! println!("{}", dot);
//! ```

#[cfg(feature = "dev-tools")]
use std::collections::{HashMap, HashSet};

/// Represents a node in the dependency graph
#[cfg(feature = "dev-tools")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    /// Name of the dependency
    pub name: String,
    /// Scope type: "singleton", "request", or "transient"
    pub scope: String,
    /// Optional type information
    pub type_name: Option<String>,
}

/// Dependency graph for visualization
#[cfg(feature = "dev-tools")]
#[derive(Debug, Default)]
pub struct DependencyGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<(String, String)>,
}

#[cfg(feature = "dev-tools")]
impl DependencyGraph {
    /// Create a new empty dependency graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let graph = DependencyGraph::new();
    /// ```
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node to the graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("Database", "singleton");
    /// ```
    pub fn add_node(&mut self, name: impl Into<String>, scope: impl Into<String>) {
        let name = name.into();
        self.nodes.insert(
            name.clone(),
            GraphNode {
                name,
                scope: scope.into(),
                type_name: None,
            },
        );
    }

    /// Add a node with type information
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_typed_node("db", "singleton", "Arc<DatabasePool>");
    /// ```
    pub fn add_typed_node(
        &mut self,
        name: impl Into<String>,
        scope: impl Into<String>,
        type_name: impl Into<String>,
    ) {
        let name = name.into();
        self.nodes.insert(
            name.clone(),
            GraphNode {
                name,
                scope: scope.into(),
                type_name: Some(type_name.into()),
            },
        );
    }

    /// Add a dependency edge from `from` to `to`
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("Service", "request");
    /// graph.add_node("Database", "singleton");
    /// graph.add_dependency("Service", "Database");
    /// ```
    pub fn add_dependency(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.edges.push((from.into(), to.into()));
    }

    /// Generate DOT format output for Graphviz
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("Database", "singleton");
    /// graph.add_node("UserService", "request");
    /// graph.add_dependency("UserService", "Database");
    ///
    /// let dot = graph.to_dot();
    /// assert!(dot.contains("digraph"));
    /// assert!(dot.contains("Database"));
    /// assert!(dot.contains("UserService"));
    /// ```
    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph DependencyGraph {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box, style=rounded];\n\n");

        for node in self.nodes.values() {
            let color = match node.scope.as_str() {
                "singleton" => "lightblue",
                "request" => "lightgreen",
                "transient" => "lightyellow",
                _ => "white",
            };

            let label = if let Some(ref type_name) = node.type_name {
                format!("{}\\n({})", node.name, type_name)
            } else {
                node.name.clone()
            };

            output.push_str(&format!(
                "  \"{}\" [label=\"{}\", fillcolor={}, style=filled];\n",
                node.name, label, color
            ));
        }

        output.push('\n');

        for (from, to) in &self.edges {
            output.push_str(&format!("  \"{}\" -> \"{}\";\n", from, to));
        }

        output.push_str("}\n");
        output
    }

    /// Detect circular dependencies in the graph
    ///
    /// Returns a list of dependency cycles if found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("A", "request");
    /// graph.add_node("B", "request");
    /// graph.add_dependency("A", "B");
    /// graph.add_dependency("B", "A");
    ///
    /// let cycles = graph.detect_cycles();
    /// assert!(!cycles.is_empty());
    /// ```
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_name in self.nodes.keys() {
            if !visited.contains(node_name) {
                let mut path = Vec::new();
                self.dfs_detect_cycles(
                    node_name,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_detect_cycles(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        let dependencies: Vec<_> = self
            .edges
            .iter()
            .filter_map(|(from, to)| if from == node { Some(to.as_str()) } else { None })
            .collect();

        for dep in dependencies {
            if !visited.contains(dep) {
                self.dfs_detect_cycles(dep, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(dep) {
                if let Some(cycle_start) = path.iter().position(|p| p == dep) {
                    cycles.push(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Get statistics about the dependency graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_di::visualization::DependencyGraph;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("A", "singleton");
    /// graph.add_node("B", "request");
    /// graph.add_dependency("B", "A");
    ///
    /// let stats = graph.statistics();
    /// assert_eq!(stats.node_count, 2);
    /// assert_eq!(stats.edge_count, 1);
    /// assert_eq!(stats.singleton_count, 1);
    /// assert_eq!(stats.request_count, 1);
    /// ```
    pub fn statistics(&self) -> GraphStatistics {
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        let singleton_count = self
            .nodes
            .values()
            .filter(|n| n.scope == "singleton")
            .count();
        let request_count = self
            .nodes
            .values()
            .filter(|n| n.scope == "request")
            .count();
        let transient_count = self
            .nodes
            .values()
            .filter(|n| n.scope == "transient")
            .count();

        GraphStatistics {
            node_count,
            edge_count,
            singleton_count,
            request_count,
            transient_count,
        }
    }
}

/// Statistics about a dependency graph
#[cfg(feature = "dev-tools")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStatistics {
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Number of singleton-scoped dependencies
    pub singleton_count: usize,
    /// Number of request-scoped dependencies
    pub request_count: usize,
    /// Number of transient dependencies
    pub transient_count: usize,
}
