//! Migration dependency graph
//!
//! This module provides the MigrationGraph structure for managing migration dependencies
//! and determining execution order through topological sorting.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
//!
//! let mut graph = MigrationGraph::new();
//!
//! // Add migrations with dependencies
//! let key1 = MigrationKey::new("myapp", "0001_initial");
//! let key2 = MigrationKey::new("myapp", "0002_add_field");
//!
//! graph.add_migration(key1.clone(), vec![]);
//! graph.add_migration(key2.clone(), vec![key1.clone()]);
//!
//! // Get execution order
//! let order = graph.topological_sort().unwrap();
//! assert_eq!(order.len(), 2);
//! assert_eq!(order[0], key1);
//! assert_eq!(order[1], key2);
//! ```

use crate::{MigrationError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Key identifying a migration (app_label, migration_name)
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::graph::MigrationKey;
///
/// let key = MigrationKey::new("auth", "0001_initial");
/// assert_eq!(key.app_label, "auth");
/// assert_eq!(key.name, "0001_initial");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MigrationKey {
    pub app_label: String,
    pub name: String,
}

impl MigrationKey {
    /// Create a new migration key
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::MigrationKey;
    ///
    /// let key = MigrationKey::new("users", "0001_initial");
    /// assert_eq!(key.app_label, "users");
    /// assert_eq!(key.name, "0001_initial");
    /// ```
    pub fn new(app_label: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            app_label: app_label.into(),
            name: name.into(),
        }
    }

    /// Get a string representation of this key
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::MigrationKey;
    ///
    /// let key = MigrationKey::new("auth", "0001_initial");
    /// assert_eq!(key.id(), "auth.0001_initial");
    /// ```
    pub fn id(&self) -> String {
        format!("{}.{}", self.app_label, self.name)
    }
}

impl std::fmt::Display for MigrationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// Migration graph node
#[derive(Debug, Clone)]
pub struct MigrationNode {
    pub key: MigrationKey,
    pub dependencies: Vec<MigrationKey>,
}

impl MigrationNode {
    /// Create a new migration node
    pub fn new(key: MigrationKey, dependencies: Vec<MigrationKey>) -> Self {
        Self { key, dependencies }
    }
}

/// Migration dependency graph
///
/// Manages migration dependencies and determines execution order.
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
///
/// let mut graph = MigrationGraph::new();
///
/// let initial = MigrationKey::new("users", "0001_initial");
/// let add_field = MigrationKey::new("users", "0002_add_email");
///
/// graph.add_migration(initial.clone(), vec![]);
/// graph.add_migration(add_field.clone(), vec![initial.clone()]);
///
/// let order = graph.topological_sort().unwrap();
/// assert_eq!(order.len(), 2);
/// ```
pub struct MigrationGraph {
    nodes: HashMap<MigrationKey, MigrationNode>,
}

impl MigrationGraph {
    /// Create a new empty migration graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::MigrationGraph;
    ///
    /// let graph = MigrationGraph::new();
    /// assert!(graph.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Add a migration to the graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    /// let key = MigrationKey::new("auth", "0001_initial");
    ///
    /// graph.add_migration(key.clone(), vec![]);
    /// assert!(graph.has_migration(&key));
    /// ```
    pub fn add_migration(&mut self, key: MigrationKey, dependencies: Vec<MigrationKey>) {
        let node = MigrationNode::new(key.clone(), dependencies);
        self.nodes.insert(key, node);
    }

    /// Check if a migration exists in the graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    /// let key = MigrationKey::new("auth", "0001_initial");
    ///
    /// assert!(!graph.has_migration(&key));
    /// graph.add_migration(key.clone(), vec![]);
    /// assert!(graph.has_migration(&key));
    /// ```
    pub fn has_migration(&self, key: &MigrationKey) -> bool {
        self.nodes.contains_key(key)
    }

    /// Get a migration node
    pub fn get_node(&self, key: &MigrationKey) -> Option<&MigrationNode> {
        self.nodes.get(key)
    }

    /// Check if the graph is empty
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    /// assert!(graph.is_empty());
    ///
    /// graph.add_migration(MigrationKey::new("auth", "0001_initial"), vec![]);
    /// assert!(!graph.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the number of migrations in the graph
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Get all migrations in the graph
    pub fn all_migrations(&self) -> Vec<&MigrationKey> {
        self.nodes.keys().collect()
    }

    /// Get direct dependencies of a migration
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    /// let key1 = MigrationKey::new("auth", "0001_initial");
    /// let key2 = MigrationKey::new("auth", "0002_add_field");
    ///
    /// graph.add_migration(key1.clone(), vec![]);
    /// graph.add_migration(key2.clone(), vec![key1.clone()]);
    ///
    /// let deps = graph.get_dependencies(&key2).unwrap();
    /// assert_eq!(deps.len(), 1);
    /// assert_eq!(deps[0], &key1);
    /// ```
    pub fn get_dependencies(&self, key: &MigrationKey) -> Option<&[MigrationKey]> {
        self.nodes.get(key).map(|node| node.dependencies.as_slice())
    }

    /// Get all migrations that depend on this migration
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    /// let key1 = MigrationKey::new("auth", "0001_initial");
    /// let key2 = MigrationKey::new("auth", "0002_add_field");
    ///
    /// graph.add_migration(key1.clone(), vec![]);
    /// graph.add_migration(key2.clone(), vec![key1.clone()]);
    ///
    /// let dependents = graph.get_dependents(&key1);
    /// assert_eq!(dependents.len(), 1);
    /// assert_eq!(dependents[0], &key2);
    /// ```
    pub fn get_dependents(&self, key: &MigrationKey) -> Vec<&MigrationKey> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.dependencies.contains(key))
            .map(|(k, _)| k)
            .collect()
    }

    /// Perform topological sort to determine migration execution order
    ///
    /// Returns migrations in the order they should be executed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    ///
    /// let key1 = MigrationKey::new("auth", "0001_initial");
    /// let key2 = MigrationKey::new("auth", "0002_add_field");
    /// let key3 = MigrationKey::new("auth", "0003_alter_field");
    ///
    /// graph.add_migration(key1.clone(), vec![]);
    /// graph.add_migration(key2.clone(), vec![key1.clone()]);
    /// graph.add_migration(key3.clone(), vec![key2.clone()]);
    ///
    /// let order = graph.topological_sort().unwrap();
    /// assert_eq!(order.len(), 3);
    /// assert_eq!(order[0], key1);
    /// assert_eq!(order[1], key2);
    /// assert_eq!(order[2], key3);
    /// ```
    pub fn topological_sort(&self) -> Result<Vec<MigrationKey>> {
        // Calculate in-degree for each node
        let mut in_degree: HashMap<MigrationKey, usize> = HashMap::new();

        for key in self.nodes.keys() {
            in_degree.entry(key.clone()).or_insert(0);
        }

        for node in self.nodes.values() {
            for dep in &node.dependencies {
                *in_degree.entry(dep.clone()).or_insert(0);
                *in_degree.entry(node.key.clone()).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0 (no dependencies)
        let mut queue: VecDeque<MigrationKey> = in_degree
            .iter()
            .filter(|&(_, &degree)| degree == 0)
            .map(|(key, _)| key.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(key) = queue.pop_front() {
            result.push(key.clone());

            // Reduce in-degree for all dependents
            for (other_key, node) in &self.nodes {
                if node.dependencies.contains(&key) {
                    if let Some(degree) = in_degree.get_mut(other_key) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(other_key.clone());
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        if result.len() != self.nodes.len() {
            let remaining: Vec<String> = self
                .nodes
                .keys()
                .filter(|k| !result.contains(k))
                .map(|k| k.id())
                .collect();

            return Err(MigrationError::CircularDependency {
                cycle: format!("Circular dependency detected: {}", remaining.join(", ")),
            });
        }

        Ok(result)
    }

    /// Get leaf nodes (migrations with no dependents)
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    ///
    /// let key1 = MigrationKey::new("auth", "0001_initial");
    /// let key2 = MigrationKey::new("auth", "0002_add_field");
    ///
    /// graph.add_migration(key1.clone(), vec![]);
    /// graph.add_migration(key2.clone(), vec![key1.clone()]);
    ///
    /// let leaves = graph.get_leaf_nodes();
    /// assert_eq!(leaves.len(), 1);
    /// assert_eq!(leaves[0], &key2);
    /// ```
    pub fn get_leaf_nodes(&self) -> Vec<&MigrationKey> {
        self.nodes
            .keys()
            .filter(|key| self.get_dependents(key).is_empty())
            .collect()
    }

    /// Get root nodes (migrations with no dependencies)
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::graph::{MigrationGraph, MigrationKey};
    ///
    /// let mut graph = MigrationGraph::new();
    ///
    /// let key1 = MigrationKey::new("auth", "0001_initial");
    /// let key2 = MigrationKey::new("auth", "0002_add_field");
    ///
    /// graph.add_migration(key1.clone(), vec![]);
    /// graph.add_migration(key2.clone(), vec![key1.clone()]);
    ///
    /// let roots = graph.get_root_nodes();
    /// assert_eq!(roots.len(), 1);
    /// assert_eq!(roots[0], &key1);
    /// ```
    pub fn get_root_nodes(&self) -> Vec<&MigrationKey> {
        self.nodes
            .values()
            .filter(|node| node.dependencies.is_empty())
            .map(|node| &node.key)
            .collect()
    }

    /// Remove a migration from the graph
    pub fn remove_migration(&mut self, key: &MigrationKey) {
        self.nodes.remove(key);
    }

    /// Clear all migrations from the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

impl Default for MigrationGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_key_creation() {
        let key = MigrationKey::new("auth", "0001_initial");
        assert_eq!(key.app_label, "auth");
        assert_eq!(key.name, "0001_initial");
        assert_eq!(key.id(), "auth.0001_initial");
    }

    #[test]
    fn test_migration_key_display() {
        let key = MigrationKey::new("users", "0002_add_email");
        assert_eq!(format!("{}", key), "users.0002_add_email");
    }

    #[test]
    fn test_graph_creation() {
        let graph = MigrationGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_add_migration() {
        let mut graph = MigrationGraph::new();
        let key = MigrationKey::new("auth", "0001_initial");

        graph.add_migration(key.clone(), vec![]);
        assert!(!graph.is_empty());
        assert_eq!(graph.len(), 1);
        assert!(graph.has_migration(&key));
    }

    #[test]
    fn test_get_dependencies() {
        let mut graph = MigrationGraph::new();
        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");

        graph.add_migration(key1.clone(), vec![]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);

        let deps = graph.get_dependencies(&key2).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], key1);
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = MigrationGraph::new();
        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");
        let key3 = MigrationKey::new("auth", "0003_alter_field");

        graph.add_migration(key1.clone(), vec![]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);
        graph.add_migration(key3.clone(), vec![key2.clone()]);

        let dependents = graph.get_dependents(&key1);
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], &key2);

        let dependents2 = graph.get_dependents(&key2);
        assert_eq!(dependents2.len(), 1);
        assert_eq!(dependents2[0], &key3);
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = MigrationGraph::new();
        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");

        graph.add_migration(key1.clone(), vec![]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], key1);
        assert_eq!(order[1], key2);
    }

    #[test]
    fn test_topological_sort_complex() {
        let mut graph = MigrationGraph::new();

        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");
        let key3 = MigrationKey::new("users", "0001_initial");
        let key4 = MigrationKey::new("users", "0002_add_auth");

        graph.add_migration(key1.clone(), vec![]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);
        graph.add_migration(key3.clone(), vec![]);
        graph.add_migration(key4.clone(), vec![key2.clone(), key3.clone()]);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 4);

        // key1 must come before key2
        let pos1 = order.iter().position(|k| k == &key1).unwrap();
        let pos2 = order.iter().position(|k| k == &key2).unwrap();
        assert!(pos1 < pos2);

        // key2 and key3 must come before key4
        let pos4 = order.iter().position(|k| k == &key4).unwrap();
        assert!(pos2 < pos4);
        let pos3 = order.iter().position(|k| k == &key3).unwrap();
        assert!(pos3 < pos4);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = MigrationGraph::new();

        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");

        // Create circular dependency
        graph.add_migration(key1.clone(), vec![key2.clone()]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);

        let result = graph.topological_sort();
        assert!(result.is_err());

        if let Err(MigrationError::CircularDependency { cycle }) = result {
            assert!(cycle.contains("Circular dependency"));
        } else {
            panic!("Expected CircularDependency error");
        }
    }

    #[test]
    fn test_get_leaf_nodes() {
        let mut graph = MigrationGraph::new();

        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");
        let key3 = MigrationKey::new("users", "0001_initial");

        graph.add_migration(key1.clone(), vec![]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);
        graph.add_migration(key3.clone(), vec![]);

        let leaves = graph.get_leaf_nodes();
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&&key2));
        assert!(leaves.contains(&&key3));
    }

    #[test]
    fn test_get_root_nodes() {
        let mut graph = MigrationGraph::new();

        let key1 = MigrationKey::new("auth", "0001_initial");
        let key2 = MigrationKey::new("auth", "0002_add_field");
        let key3 = MigrationKey::new("users", "0001_initial");

        graph.add_migration(key1.clone(), vec![]);
        graph.add_migration(key2.clone(), vec![key1.clone()]);
        graph.add_migration(key3.clone(), vec![]);

        let roots = graph.get_root_nodes();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&&key1));
        assert!(roots.contains(&&key3));
    }

    #[test]
    fn test_remove_migration() {
        let mut graph = MigrationGraph::new();
        let key = MigrationKey::new("auth", "0001_initial");

        graph.add_migration(key.clone(), vec![]);
        assert!(graph.has_migration(&key));

        graph.remove_migration(&key);
        assert!(!graph.has_migration(&key));
        assert!(graph.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut graph = MigrationGraph::new();

        graph.add_migration(MigrationKey::new("auth", "0001_initial"), vec![]);
        graph.add_migration(MigrationKey::new("users", "0001_initial"), vec![]);

        assert_eq!(graph.len(), 2);

        graph.clear();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_multiple_root_nodes_sort() {
        let mut graph = MigrationGraph::new();

        let auth_0001 = MigrationKey::new("auth", "0001_initial");
        let users_0001 = MigrationKey::new("users", "0001_initial");
        let posts_0001 = MigrationKey::new("posts", "0001_initial");

        graph.add_migration(auth_0001.clone(), vec![]);
        graph.add_migration(users_0001.clone(), vec![]);
        graph.add_migration(posts_0001.clone(), vec![]);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
        // All three should be in the result
        assert!(order.contains(&auth_0001));
        assert!(order.contains(&users_0001));
        assert!(order.contains(&posts_0001));
    }

    #[test]
    fn test_complex_dependency_chain() {
        let mut graph = MigrationGraph::new();

        // Create a diamond-shaped dependency graph
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        let a = MigrationKey::new("app", "0001_a");
        let b = MigrationKey::new("app", "0002_b");
        let c = MigrationKey::new("app", "0003_c");
        let d = MigrationKey::new("app", "0004_d");

        graph.add_migration(a.clone(), vec![]);
        graph.add_migration(b.clone(), vec![a.clone()]);
        graph.add_migration(c.clone(), vec![a.clone()]);
        graph.add_migration(d.clone(), vec![b.clone(), c.clone()]);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 4);

        let pos_a = order.iter().position(|k| k == &a).unwrap();
        let pos_b = order.iter().position(|k| k == &b).unwrap();
        let pos_c = order.iter().position(|k| k == &c).unwrap();
        let pos_d = order.iter().position(|k| k == &d).unwrap();

        // A must come before B and C
        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);

        // B and C must come before D
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }
}
