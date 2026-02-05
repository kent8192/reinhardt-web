//! Dependency resolution for static assets
//!
//! Provides a dependency graph for resolving the order in which files should
//! be processed or bundled.

use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency graph for resolving file processing order
///
/// Tracks dependencies between files and provides topological sorting
/// to determine the correct order for processing.
pub struct DependencyGraph {
	/// Adjacency list: file -> dependencies
	dependencies: HashMap<String, HashSet<String>>,
	/// All files in the graph
	files: HashSet<String>,
}

impl DependencyGraph {
	/// Create a new dependency graph
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::DependencyGraph;
	///
	/// let graph = DependencyGraph::new();
	/// ```
	pub fn new() -> Self {
		Self {
			dependencies: HashMap::new(),
			files: HashSet::new(),
		}
	}

	/// Add a file to the graph
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::DependencyGraph;
	///
	/// let mut graph = DependencyGraph::new();
	/// graph.add_file("app.js".to_string());
	/// graph.add_file("utils.js".to_string());
	/// ```
	pub fn add_file(&mut self, file: String) {
		self.files.insert(file.clone());
		self.dependencies.entry(file).or_default();
	}

	/// Add a dependency between two files
	///
	/// This indicates that `from` depends on `to`, meaning `to` must be
	/// processed before `from`.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::DependencyGraph;
	///
	/// let mut graph = DependencyGraph::new();
	/// graph.add_file("main.js".to_string());
	/// graph.add_file("utils.js".to_string());
	/// graph.add_dependency("main.js".to_string(), "utils.js".to_string());
	/// ```
	pub fn add_dependency(&mut self, from: String, to: String) {
		self.add_file(from.clone());
		self.add_file(to.clone());
		self.dependencies.entry(from).or_default().insert(to);
	}

	/// Resolve the processing order using topological sort
	///
	/// Returns files in dependency order (dependencies first).
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::staticfiles::DependencyGraph;
	///
	/// let mut graph = DependencyGraph::new();
	/// graph.add_file("main.js".to_string());
	/// graph.add_file("utils.js".to_string());
	/// graph.add_dependency("main.js".to_string(), "utils.js".to_string());
	///
	/// let order = graph.resolve_order();
	/// // utils.js comes before main.js
	/// let utils_idx = order.iter().position(|f| f == "utils.js").unwrap();
	/// let main_idx = order.iter().position(|f| f == "main.js").unwrap();
	/// assert!(utils_idx < main_idx);
	/// ```
	pub fn resolve_order(&self) -> Vec<String> {
		let mut in_degree: HashMap<String, usize> = HashMap::new();
		let mut reverse_deps: HashMap<String, HashSet<String>> = HashMap::new();

		// Initialize in-degree count
		for file in &self.files {
			in_degree.insert(file.clone(), 0);
			reverse_deps.insert(file.clone(), HashSet::new());
		}

		// Count in-degrees
		// `from` depends on `to`, so `from` has an incoming edge from `to`
		for (from, deps) in &self.dependencies {
			for to in deps {
				*in_degree.get_mut(from).unwrap() += 1;
				reverse_deps.get_mut(to).unwrap().insert(from.clone());
			}
		}

		// Kahn's algorithm for topological sort
		let mut queue = VecDeque::new();
		for (file, &degree) in &in_degree {
			if degree == 0 {
				queue.push_back(file.clone());
			}
		}

		let mut result = Vec::new();
		while let Some(file) = queue.pop_front() {
			result.push(file.clone());

			// Process files that depend on this file
			if let Some(dependents) = reverse_deps.get(&file) {
				for dependent in dependents {
					let degree = in_degree.get_mut(dependent).unwrap();
					*degree -= 1;
					if *degree == 0 {
						queue.push_back(dependent.clone());
					}
				}
			}
		}

		result
	}

	/// Get the number of files in the graph
	pub fn len(&self) -> usize {
		self.files.len()
	}

	/// Check if the graph is empty
	pub fn is_empty(&self) -> bool {
		self.files.is_empty()
	}
}

impl Default for DependencyGraph {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new_graph() {
		let graph = DependencyGraph::new();
		assert!(graph.is_empty());
		assert_eq!(graph.len(), 0);
	}

	#[test]
	fn test_add_file() {
		let mut graph = DependencyGraph::new();
		graph.add_file("test.js".to_string());
		assert_eq!(graph.len(), 1);
		assert!(!graph.is_empty());
	}

	#[test]
	fn test_add_dependency() {
		let mut graph = DependencyGraph::new();
		graph.add_dependency("main.js".to_string(), "utils.js".to_string());
		assert_eq!(graph.len(), 2);
	}

	#[test]
	fn test_resolve_order_no_dependencies() {
		let mut graph = DependencyGraph::new();
		graph.add_file("a.js".to_string());
		graph.add_file("b.js".to_string());
		graph.add_file("c.js".to_string());

		let order = graph.resolve_order();
		assert_eq!(order.len(), 3);
	}

	#[test]
	fn test_resolve_order_with_dependencies() {
		let mut graph = DependencyGraph::new();
		graph.add_dependency("main.js".to_string(), "utils.js".to_string());
		graph.add_dependency("main.js".to_string(), "config.js".to_string());

		let order = graph.resolve_order();
		let main_idx = order.iter().position(|f| f == "main.js").unwrap();
		let utils_idx = order.iter().position(|f| f == "utils.js").unwrap();
		let config_idx = order.iter().position(|f| f == "config.js").unwrap();

		// Dependencies must come before main
		assert!(utils_idx < main_idx);
		assert!(config_idx < main_idx);
	}

	#[test]
	fn test_resolve_order_chain() {
		let mut graph = DependencyGraph::new();
		graph.add_dependency("a.js".to_string(), "b.js".to_string());
		graph.add_dependency("b.js".to_string(), "c.js".to_string());

		let order = graph.resolve_order();
		let a_idx = order.iter().position(|f| f == "a.js").unwrap();
		let b_idx = order.iter().position(|f| f == "b.js").unwrap();
		let c_idx = order.iter().position(|f| f == "c.js").unwrap();

		// c -> b -> a
		assert!(c_idx < b_idx);
		assert!(b_idx < a_idx);
	}

	#[test]
	fn test_resolve_order_multiple_roots() {
		let mut graph = DependencyGraph::new();
		graph.add_file("root1.js".to_string());
		graph.add_file("root2.js".to_string());
		graph.add_dependency("root1.js".to_string(), "shared.js".to_string());
		graph.add_dependency("root2.js".to_string(), "shared.js".to_string());

		let order = graph.resolve_order();
		let shared_idx = order.iter().position(|f| f == "shared.js").unwrap();
		let root1_idx = order.iter().position(|f| f == "root1.js").unwrap();
		let root2_idx = order.iter().position(|f| f == "root2.js").unwrap();

		// shared must come before both roots
		assert!(shared_idx < root1_idx);
		assert!(shared_idx < root2_idx);
	}

	#[test]
	fn test_default() {
		let graph = DependencyGraph::default();
		assert!(graph.is_empty());
	}
}
