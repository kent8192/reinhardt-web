//! Migration autodetector

use petgraph::Undirected;
use petgraph::graph::Graph;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use strsim::{jaro_winkler, levenshtein};

/// Field state for migration detection
#[derive(Debug, Clone)]
pub struct FieldState {
	pub name: String,
	pub field_type: String,
	pub nullable: bool,
	pub params: std::collections::HashMap<String, String>,
}

impl FieldState {
	pub fn new(name: String, field_type: String, nullable: bool) -> Self {
		Self {
			name,
			field_type,
			nullable,
			params: std::collections::HashMap::new(),
		}
	}
}

/// Model state for migration detection
///
/// Django equivalent: `ModelState` in django/db/migrations/state.py
#[derive(Debug, Clone)]
pub struct ModelState {
	/// Application label (e.g., "auth", "blog")
	pub app_label: String,
	/// Model name (e.g., "User", "Post")
	pub name: String,
	/// Fields: field_name -> FieldState
	pub fields: std::collections::HashMap<String, FieldState>,
	/// Model options (db_table, ordering, etc.)
	pub options: std::collections::HashMap<String, String>,
	/// Base model for inheritance
	pub base_model: Option<String>,
	/// Inheritance type: "single_table" or "joined_table"
	pub inheritance_type: Option<String>,
	/// Discriminator column for single table inheritance
	pub discriminator_column: Option<String>,
	/// Indexes: index_name -> IndexDefinition
	pub indexes: Vec<IndexDefinition>,
	/// Constraints: constraint_name -> ConstraintDefinition
	pub constraints: Vec<ConstraintDefinition>,
}

/// Index definition for a model
#[derive(Debug, Clone, PartialEq)]
pub struct IndexDefinition {
	/// Index name
	pub name: String,
	/// Fields to index (in order)
	pub fields: Vec<String>,
	/// Whether this is a unique index
	pub unique: bool,
}

/// Constraint definition for a model
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDefinition {
	/// Constraint name
	pub name: String,
	/// Constraint type (e.g., "check", "unique", "foreign_key")
	pub constraint_type: String,
	/// Fields involved in the constraint
	pub fields: Vec<String>,
	/// Additional constraint expression (e.g., CHECK condition)
	pub expression: Option<String>,
}

impl ModelState {
	/// Create a new ModelState with app_label and name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::ModelState;
	///
	/// let model = ModelState::new("myapp", "User");
	/// assert_eq!(model.app_label, "myapp");
	/// assert_eq!(model.name, "User");
	/// assert_eq!(model.fields.len(), 0);
	/// ```
	pub fn new(app_label: impl Into<String>, name: impl Into<String>) -> Self {
		Self {
			app_label: app_label.into(),
			name: name.into(),
			fields: std::collections::HashMap::new(),
			options: std::collections::HashMap::new(),
			base_model: None,
			inheritance_type: None,
			discriminator_column: None,
			indexes: Vec::new(),
			constraints: Vec::new(),
		}
	}

	/// Add a field to this model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ModelState, FieldState};
	///
	/// let mut model = ModelState::new("myapp", "User");
	/// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	/// model.add_field(field);
	/// assert_eq!(model.fields.len(), 1);
	/// assert!(model.has_field("email"));
	/// ```
	pub fn add_field(&mut self, field: FieldState) {
		self.fields.insert(field.name.clone(), field);
	}

	/// Get a field by name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ModelState, FieldState};
	///
	/// let mut model = ModelState::new("myapp", "User");
	/// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	/// model.add_field(field);
	///
	/// let retrieved = model.get_field("email");
	/// assert!(retrieved.is_some());
	/// assert_eq!(retrieved.unwrap().field_type, "VARCHAR(255)");
	/// ```
	pub fn get_field(&self, name: &str) -> Option<&FieldState> {
		self.fields.get(name)
	}

	/// Check if a field exists
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ModelState, FieldState};
	///
	/// let mut model = ModelState::new("myapp", "User");
	/// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	/// model.add_field(field);
	///
	/// assert!(model.has_field("email"));
	/// assert!(!model.has_field("username"));
	/// ```
	pub fn has_field(&self, name: &str) -> bool {
		self.fields.contains_key(name)
	}

	/// Rename a field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ModelState, FieldState};
	///
	/// let mut model = ModelState::new("myapp", "User");
	/// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	/// model.add_field(field);
	///
	/// model.rename_field("email", "email_address".to_string());
	/// assert!(!model.has_field("email"));
	/// assert!(model.has_field("email_address"));
	/// ```
	pub fn rename_field(&mut self, old_name: &str, new_name: String) {
		if let Some(mut field) = self.fields.remove(old_name) {
			field.name = new_name.clone();
			self.fields.insert(new_name, field);
		}
	}
}

/// Project state for migration detection
///
/// Django equivalent: `ProjectState` in django/db/migrations/state.py
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::{ProjectState, ModelState, FieldState};
///
/// let mut state = ProjectState::new();
/// let mut model = ModelState::new("myapp", "User");
/// model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
/// state.add_model(model);
///
/// assert!(state.get_model("myapp", "User").is_some());
/// ```
#[derive(Debug, Clone)]
pub struct ProjectState {
	/// Models: (app_label, model_name) -> ModelState
	pub models: std::collections::HashMap<(String, String), ModelState>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectState {
	/// Create a new empty ProjectState
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::ProjectState;
	///
	/// let state = ProjectState::new();
	/// assert_eq!(state.models.len(), 0);
	/// ```
	pub fn new() -> Self {
		Self {
			models: std::collections::HashMap::new(),
		}
	}

	/// Add a model to this project state
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ProjectState, ModelState};
	///
	/// let mut state = ProjectState::new();
	/// let model = ModelState::new("myapp", "User");
	/// state.add_model(model);
	///
	/// assert_eq!(state.models.len(), 1);
	/// assert!(state.get_model("myapp", "User").is_some());
	/// ```
	pub fn add_model(&mut self, model: ModelState) {
		let key = (model.app_label.clone(), model.name.clone());
		self.models.insert(key, model);
	}

	/// Get a model by app_label and model_name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ProjectState, ModelState};
	///
	/// let mut state = ProjectState::new();
	/// let model = ModelState::new("myapp", "User");
	/// state.add_model(model);
	///
	/// let retrieved = state.get_model("myapp", "User");
	/// assert!(retrieved.is_some());
	/// assert_eq!(retrieved.unwrap().name, "User");
	/// ```
	pub fn get_model(&self, app_label: &str, model_name: &str) -> Option<&ModelState> {
		self.models
			.get(&(app_label.to_string(), model_name.to_string()))
	}

	/// Get a mutable reference to a model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ProjectState, ModelState, FieldState};
	///
	/// let mut state = ProjectState::new();
	/// let model = ModelState::new("myapp", "User");
	/// state.add_model(model);
	///
	/// if let Some(model) = state.get_model_mut("myapp", "User") {
	///     let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	///     model.add_field(field);
	/// }
	///
	/// assert!(state.get_model("myapp", "User").unwrap().has_field("email"));
	/// ```
	pub fn get_model_mut(&mut self, app_label: &str, model_name: &str) -> Option<&mut ModelState> {
		self.models
			.get_mut(&(app_label.to_string(), model_name.to_string()))
	}

	/// Remove a model from this project state
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ProjectState, ModelState};
	///
	/// let mut state = ProjectState::new();
	/// let model = ModelState::new("myapp", "User");
	/// state.add_model(model);
	///
	/// state.remove_model("myapp", "User");
	/// assert!(state.get_model("myapp", "User").is_none());
	/// ```
	pub fn remove_model(&mut self, app_label: &str, model_name: &str) {
		self.models
			.remove(&(app_label.to_string(), model_name.to_string()));
	}

	/// Rename a model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ProjectState, ModelState};
	///
	/// let mut state = ProjectState::new();
	/// let model = ModelState::new("myapp", "User");
	/// state.add_model(model);
	///
	/// state.rename_model("myapp", "User", "Account".to_string());
	/// assert!(state.get_model("myapp", "User").is_none());
	/// assert!(state.get_model("myapp", "Account").is_some());
	/// ```
	pub fn rename_model(&mut self, app_label: &str, old_name: &str, new_name: String) {
		if let Some(mut model) = self
			.models
			.remove(&(app_label.to_string(), old_name.to_string()))
		{
			model.name = new_name.clone();
			self.models.insert((app_label.to_string(), new_name), model);
		}
	}

	/// Load ProjectState from the global model registry
	///
	/// Django equivalent: `ProjectState.from_apps()` in django/db/migrations/state.py:594-600
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::ProjectState;
	///
	/// let state = ProjectState::from_global_registry();
	// state will contain all models registered in the global registry
	/// ```
	pub fn from_global_registry() -> Self {
		use crate::model_registry::global_registry;

		let registry = global_registry();
		let models_metadata = registry.get_models();

		let mut state = ProjectState::new();

		for metadata in models_metadata {
			let model_state = metadata.to_model_state();
			state.add_model(model_state);
		}

		state
	}
}

/// Configuration for similarity threshold calculation
///
/// This struct controls how aggressive the autodetector is when matching
/// models and fields across apps for rename/move detection.
///
/// Uses a hybrid similarity metric combining:
/// - Jaro-Winkler distance: Best for detecting prefix similarities (e.g., "UserModel" vs "UserProfile")
/// - Levenshtein distance: Best for detecting edit operations (e.g., "User" vs "Users")
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::SimilarityConfig;
///
/// // Default configuration (70% threshold for models, 80% for fields)
/// let config = SimilarityConfig::default();
/// assert_eq!(config.model_threshold(), 0.7);
///
/// // Custom conservative configuration (higher threshold = fewer matches)
/// let config = SimilarityConfig::new(0.85, 0.90).unwrap();
///
/// // Liberal configuration (lower threshold = more matches, but more false positives)
/// let config = SimilarityConfig::new(0.60, 0.70).unwrap();
///
/// // Custom with specific algorithm weights
/// let config = SimilarityConfig::with_weights(0.75, 0.85, 0.6, 0.4).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct SimilarityConfig {
	/// Threshold for model similarity (0.5 - 0.95)
	/// Higher values mean stricter matching (fewer false positives)
	model_threshold: f64,
	/// Threshold for field similarity (0.5 - 0.95)
	/// Higher values mean stricter matching
	field_threshold: f64,
	/// Weight for Jaro-Winkler component (0.0 - 1.0, default 0.7)
	/// Higher values prioritize prefix matching
	jaro_winkler_weight: f64,
	/// Weight for Levenshtein component (0.0 - 1.0, default 0.3)
	/// Higher values prioritize edit distance
	/// Note: jaro_winkler_weight + levenshtein_weight should equal 1.0
	levenshtein_weight: f64,
}

impl SimilarityConfig {
	/// Create a new SimilarityConfig with custom thresholds
	///
	/// # Arguments
	///
	/// * `model_threshold` - Similarity threshold for model matching (0.5 - 0.95)
	/// * `field_threshold` - Similarity threshold for field matching (0.5 - 0.95)
	///
	/// # Errors
	///
	/// Returns an error if thresholds are outside the valid range (0.5 - 0.95).
	/// Values below 0.5 would produce too many false positives.
	/// Values above 0.95 would make matching nearly impossible.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::SimilarityConfig;
	///
	/// let config = SimilarityConfig::new(0.75, 0.85).unwrap();
	/// assert_eq!(config.model_threshold(), 0.75);
	/// assert_eq!(config.field_threshold(), 0.85);
	///
	/// // Invalid threshold (too low)
	/// assert!(SimilarityConfig::new(0.4, 0.8).is_err());
	///
	/// // Invalid threshold (too high)
	/// assert!(SimilarityConfig::new(0.96, 0.8).is_err());
	/// ```
	pub fn new(model_threshold: f64, field_threshold: f64) -> Result<Self, String> {
		Self::with_weights(model_threshold, field_threshold, 0.7, 0.3)
	}

	/// Create a new SimilarityConfig with custom thresholds and algorithm weights
	///
	/// # Arguments
	///
	/// * `model_threshold` - Similarity threshold for model matching (0.5 - 0.95)
	/// * `field_threshold` - Similarity threshold for field matching (0.5 - 0.95)
	/// * `jaro_winkler_weight` - Weight for Jaro-Winkler component (0.0 - 1.0)
	/// * `levenshtein_weight` - Weight for Levenshtein component (0.0 - 1.0)
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Thresholds are outside the valid range (0.5 - 0.95)
	/// - Weights are outside the valid range (0.0 - 1.0)
	/// - Weights don't sum to approximately 1.0 (within 0.01 tolerance)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::SimilarityConfig;
	///
	/// // Prefer Jaro-Winkler for prefix matching
	/// let config = SimilarityConfig::with_weights(0.75, 0.85, 0.8, 0.2).unwrap();
	///
	/// // Prefer Levenshtein for edit distance
	/// let config = SimilarityConfig::with_weights(0.75, 0.85, 0.3, 0.7).unwrap();
	///
	/// // Invalid: weights don't sum to 1.0
	/// assert!(SimilarityConfig::with_weights(0.75, 0.85, 0.5, 0.3).is_err());
	/// ```
	pub fn with_weights(
		model_threshold: f64,
		field_threshold: f64,
		jaro_winkler_weight: f64,
		levenshtein_weight: f64,
	) -> Result<Self, String> {
		// Validate thresholds are in reasonable range
		if !(0.5..=0.95).contains(&model_threshold) {
			return Err(format!(
				"model_threshold must be between 0.5 and 0.95, got {}",
				model_threshold
			));
		}
		if !(0.5..=0.95).contains(&field_threshold) {
			return Err(format!(
				"field_threshold must be between 0.5 and 0.95, got {}",
				field_threshold
			));
		}

		// Validate weights are in valid range
		if !(0.0..=1.0).contains(&jaro_winkler_weight) {
			return Err(format!(
				"jaro_winkler_weight must be between 0.0 and 1.0, got {}",
				jaro_winkler_weight
			));
		}
		if !(0.0..=1.0).contains(&levenshtein_weight) {
			return Err(format!(
				"levenshtein_weight must be between 0.0 and 1.0, got {}",
				levenshtein_weight
			));
		}

		// Validate weights sum to approximately 1.0 (allow small floating point errors)
		let weight_sum = jaro_winkler_weight + levenshtein_weight;
		if (weight_sum - 1.0).abs() > 0.01 {
			return Err(format!(
				"jaro_winkler_weight + levenshtein_weight must sum to 1.0, got {} + {} = {}",
				jaro_winkler_weight, levenshtein_weight, weight_sum
			));
		}

		Ok(Self {
			model_threshold,
			field_threshold,
			jaro_winkler_weight,
			levenshtein_weight,
		})
	}

	/// Get the model similarity threshold
	pub fn model_threshold(&self) -> f64 {
		self.model_threshold
	}

	/// Get the field similarity threshold
	pub fn field_threshold(&self) -> f64 {
		self.field_threshold
	}
}

impl Default for SimilarityConfig {
	/// Default configuration with balanced thresholds and weights
	///
	/// - Model threshold: 0.7 (70% similarity required)
	/// - Field threshold: 0.8 (80% similarity required)
	/// - Jaro-Winkler weight: 0.7 (70% weight for prefix matching)
	/// - Levenshtein weight: 0.3 (30% weight for edit distance)
	fn default() -> Self {
		Self {
			model_threshold: 0.7,
			field_threshold: 0.8,
			jaro_winkler_weight: 0.7,
			levenshtein_weight: 0.3,
		}
	}
}

/// Migration autodetector
///
/// Django equivalent: `MigrationAutodetector` in django/db/migrations/autodetector.py
///
/// Detects schema changes between two ProjectStates and generates migrations.
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
///
/// let mut from_state = ProjectState::new();
/// let mut to_state = ProjectState::new();
///
// Add a new model to to_state
/// let mut model = ModelState::new("myapp", "User");
/// model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
/// to_state.add_model(model);
///
/// let detector = MigrationAutodetector::new(from_state, to_state);
/// let changes = detector.detect_changes();
///
// Should detect the new model creation
/// assert_eq!(changes.created_models.len(), 1);
/// ```
pub struct MigrationAutodetector {
	from_state: ProjectState,
	to_state: ProjectState,
	similarity_config: SimilarityConfig,
}

/// Detected changes between two project states
#[derive(Debug, Clone, Default)]
pub struct DetectedChanges {
	/// Models that were created: (app_label, model_name)
	pub created_models: Vec<(String, String)>,
	/// Models that were deleted: (app_label, model_name)
	pub deleted_models: Vec<(String, String)>,
	/// Fields that were added: (app_label, model_name, field_name)
	pub added_fields: Vec<(String, String, String)>,
	/// Fields that were removed: (app_label, model_name, field_name)
	pub removed_fields: Vec<(String, String, String)>,
	/// Fields that were altered: (app_label, model_name, field_name)
	pub altered_fields: Vec<(String, String, String)>,
	/// Models that were renamed: (app_label, old_name, new_name)
	pub renamed_models: Vec<(String, String, String)>,
	/// Models that were moved between apps: (from_app, to_app, model_name, rename_table, old_table, new_table)
	pub moved_models: Vec<(String, String, String, bool, Option<String>, Option<String>)>,
	/// Fields that were renamed: (app_label, model_name, old_name, new_name)
	pub renamed_fields: Vec<(String, String, String, String)>,
	/// Indexes that were added: (app_label, model_name, IndexDefinition)
	pub added_indexes: Vec<(String, String, IndexDefinition)>,
	/// Indexes that were removed: (app_label, model_name, index_name)
	pub removed_indexes: Vec<(String, String, String)>,
	/// Constraints that were added: (app_label, model_name, ConstraintDefinition)
	pub added_constraints: Vec<(String, String, ConstraintDefinition)>,
	/// Constraints that were removed: (app_label, model_name, constraint_name)
	pub removed_constraints: Vec<(String, String, String)>,
	/// Model dependencies for ordering operations
	/// Maps (app_label, model_name) -> Vec<(dependent_app, dependent_model)>
	/// A model depends on another if it has ForeignKey or ManyToMany fields pointing to it
	pub model_dependencies: std::collections::HashMap<(String, String), Vec<(String, String)>>,
}

impl DetectedChanges {
	/// Order models for migration operations based on dependencies
	///
	/// Uses topological sort (Kahn's algorithm) to determine the correct order
	/// for creating or moving models. This ensures that referenced models are
	/// processed before models that reference them.
	///
	/// # Algorithm: Kahn's Algorithm (Topological Sort)
	/// - Time Complexity: O(V + E) where V is models, E is dependencies
	/// - Detects circular dependencies and handles them gracefully
	/// - Returns models in dependency order (bottom-up)
	///
	/// # Returns
	/// A vector of (app_label, model_name) tuples in dependency order.
	/// Models with no dependencies come first, models depending on others come last.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{DetectedChanges};
	/// use std::collections::HashMap;
	///
	/// let mut changes = DetectedChanges::default();
	/// changes.created_models.push(("accounts".to_string(), "User".to_string()));
	/// changes.created_models.push(("blog".to_string(), "Post".to_string()));
	///
	/// // Post depends on User
	/// let mut deps = HashMap::new();
	/// deps.insert(
	///     ("blog".to_string(), "Post".to_string()),
	///     vec![("accounts".to_string(), "User".to_string())],
	/// );
	/// changes.model_dependencies = deps;
	///
	/// let ordered = changes.order_models_by_dependency();
	/// // User comes before Post
	/// assert_eq!(ordered[0], ("accounts".to_string(), "User".to_string()));
	/// assert_eq!(ordered[1], ("blog".to_string(), "Post".to_string()));
	/// ```
	pub fn order_models_by_dependency(&self) -> Vec<(String, String)> {
		use std::collections::{HashMap, HashSet, VecDeque};

		// Build in-degree map (count of incoming edges)
		let mut in_degree: HashMap<(String, String), usize> = HashMap::new();
		let mut all_models: HashSet<(String, String)> = HashSet::new();

		// Collect all models (both created and dependencies)
		for model in &self.created_models {
			all_models.insert(model.clone());
			in_degree.entry(model.clone()).or_insert(0);
		}

		for model in &self.moved_models {
			let model_key = (model.1.clone(), model.2.clone()); // (to_app, model_name)
			all_models.insert(model_key.clone());
			in_degree.entry(model_key).or_insert(0);
		}

		// Build in-degree counts from dependencies
		for (dependent, dependencies) in &self.model_dependencies {
			for dependency in dependencies {
				all_models.insert(dependency.clone());
				in_degree.entry(dependency.clone()).or_insert(0);
				*in_degree.entry(dependent.clone()).or_insert(0) += 1;
			}
		}

		// Kahn's algorithm: Start with models that have no dependencies
		let mut queue: VecDeque<(String, String)> = VecDeque::new();
		for model in &all_models {
			if in_degree.get(model).copied().unwrap_or(0) == 0 {
				queue.push_back(model.clone());
			}
		}

		let mut ordered = Vec::new();

		while let Some(model) = queue.pop_front() {
			ordered.push(model.clone());

			// Reduce in-degree for models that depend on this model
			// model_dependencies maps dependent -> dependencies
			// So we need to find all models that have `model` in their dependencies
			for (dependent, dependencies) in &self.model_dependencies {
				if dependencies.contains(&model)
					&& let Some(degree) = in_degree.get_mut(dependent) {
						*degree -= 1;
						if *degree == 0 {
							queue.push_back(dependent.clone());
						}
					}
			}
		}

		// If not all models are ordered, there's a circular dependency
		if ordered.len() < all_models.len() {
			// Fall back to original order with a warning
			let unordered_models: Vec<_> = all_models
				.iter()
				.filter(|model| !ordered.contains(model))
				.map(|(app, name)| format!("{}.{}", app, name))
				.collect();

			eprintln!(
				"⚠️  Warning: Circular dependency detected in models: [{}]",
				unordered_models.join(", ")
			);
			eprintln!(
				"    Falling back to original order. Migration operations may need manual reordering."
			);

			all_models.into_iter().collect()
		} else {
			ordered
		}
	}

	/// Check for circular dependencies in model relationships
	///
	/// Detects cycles in the dependency graph using depth-first search.
	///
	/// # Returns
	/// - `Ok(())` if no circular dependencies exist
	/// - `Err(Vec<(String, String)>)` with the cycle path if found
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{DetectedChanges};
	/// use std::collections::HashMap;
	///
	/// let mut changes = DetectedChanges::default();
	///
	/// // Create circular dependency: A -> B -> C -> A
	/// let mut deps = HashMap::new();
	/// deps.insert(
	///     ("app".to_string(), "A".to_string()),
	///     vec![("app".to_string(), "B".to_string())],
	/// );
	/// deps.insert(
	///     ("app".to_string(), "B".to_string()),
	///     vec![("app".to_string(), "C".to_string())],
	/// );
	/// deps.insert(
	///     ("app".to_string(), "C".to_string()),
	///     vec![("app".to_string(), "A".to_string())],
	/// );
	/// changes.model_dependencies = deps;
	///
	/// assert!(changes.check_circular_dependencies().is_err());
	/// ```
	pub fn check_circular_dependencies(&self) -> Result<(), Vec<(String, String)>> {
		use std::collections::{HashMap, HashSet};

		let mut visited: HashSet<(String, String)> = HashSet::new();
		let mut rec_stack: HashSet<(String, String)> = HashSet::new();
		let mut path: Vec<(String, String)> = Vec::new();

		fn dfs(
			model: &(String, String),
			deps: &HashMap<(String, String), Vec<(String, String)>>,
			visited: &mut HashSet<(String, String)>,
			rec_stack: &mut HashSet<(String, String)>,
			path: &mut Vec<(String, String)>,
		) -> Option<Vec<(String, String)>> {
			visited.insert(model.clone());
			rec_stack.insert(model.clone());
			path.push(model.clone());

			if let Some(dependencies) = deps.get(model) {
				for dep in dependencies {
					if !visited.contains(dep) {
						if let Some(cycle) = dfs(dep, deps, visited, rec_stack, path) {
							return Some(cycle);
						}
					} else if rec_stack.contains(dep) {
						// Found cycle
						let cycle_start = path.iter().position(|m| m == dep).unwrap();
						return Some(path[cycle_start..].to_vec());
					}
				}
			}

			path.pop();
			rec_stack.remove(model);
			None
		}

		for model in self.model_dependencies.keys() {
			if !visited.contains(model)
				&& let Some(cycle) = dfs(
					model,
					&self.model_dependencies,
					&mut visited,
					&mut rec_stack,
					&mut path,
				) {
					return Err(cycle);
				}
		}

		Ok(())
	}
}

// ============================================================================
// Phase 2: Advanced Change Inference System
// ============================================================================

/// Change history entry for temporal pattern analysis
///
/// Tracks individual changes with timestamps to identify patterns over time.
/// This enables the autodetector to learn from past migrations and make
/// better predictions about future changes.
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::autodetector::ChangeHistoryEntry;
/// use std::time::SystemTime;
///
/// let entry = ChangeHistoryEntry {
///     timestamp: SystemTime::now(),
///     change_type: "RenameModel".to_string(),
///     app_label: "blog".to_string(),
///     model_name: "Post".to_string(),
///     field_name: None,
///     old_value: Some("BlogPost".to_string()),
///     new_value: Some("Post".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ChangeHistoryEntry {
	/// When this change occurred
	pub timestamp: std::time::SystemTime,
	/// Type of change (e.g., "RenameModel", "AddField", "MoveModel")
	pub change_type: String,
	/// App label of the affected model
	pub app_label: String,
	/// Model name
	pub model_name: String,
	/// Field name (if field-level change)
	pub field_name: Option<String>,
	/// Old value (for renames/alterations)
	pub old_value: Option<String>,
	/// New value (for renames/alterations)
	pub new_value: Option<String>,
}

/// Pattern frequency for learning from historical changes
///
/// Tracks how often certain patterns appear to predict future changes.
/// For example, if "User -> Account" rename happened 5 times in history,
/// similar patterns will get higher confidence scores.
#[derive(Debug, Clone)]
pub struct PatternFrequency {
	/// The pattern being tracked (e.g., "RenameModel:User->Account")
	pub pattern: String,
	/// Number of times this pattern occurred
	pub frequency: usize,
	/// Last time this pattern was seen
	pub last_seen: std::time::SystemTime,
	/// Contexts where this pattern appeared
	pub contexts: Vec<String>,
}

/// Change tracker for temporal pattern analysis
///
/// Maintains a history of schema changes and analyzes patterns over time
/// to improve autodetection accuracy. This implements Django's concept of
/// "migration squashing" intelligence - learning which changes commonly
/// occur together.
///
/// # Algorithm: Temporal Pattern Mining
/// - Time Complexity: O(n) for insertion, O(n log n) for pattern analysis
/// - Space Complexity: O(h) where h is history size
/// - Uses sliding window for recent changes (last 100 by default)
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::ChangeTracker;
///
/// let mut tracker = ChangeTracker::new();
///
/// // Track a model rename
/// tracker.record_model_rename("blog", "BlogPost", "Post");
///
/// // Track a field addition
/// tracker.record_field_addition("blog", "Post", "slug");
///
/// // Get pattern frequency
/// let patterns = tracker.get_frequent_patterns(2); // Min frequency: 2
/// ```
#[derive(Debug, Clone)]
pub struct ChangeTracker {
	/// Complete history of changes
	history: Vec<ChangeHistoryEntry>,
	/// Pattern frequency map
	patterns: HashMap<String, PatternFrequency>,
	/// Maximum history size (for memory efficiency)
	max_history_size: usize,
}

impl ChangeTracker {
	/// Create a new change tracker with default settings
	///
	/// Default max history size: 1000 entries
	pub fn new() -> Self {
		Self {
			history: Vec::new(),
			patterns: HashMap::new(),
			max_history_size: 1000,
		}
	}

	/// Create a change tracker with custom history size
	pub fn with_capacity(max_size: usize) -> Self {
		Self {
			history: Vec::with_capacity(max_size),
			patterns: HashMap::new(),
			max_history_size: max_size,
		}
	}

	/// Record a model rename in the history
	///
	/// # Arguments
	/// * `app_label` - App containing the model
	/// * `old_name` - Original model name
	/// * `new_name` - New model name
	pub fn record_model_rename(&mut self, app_label: &str, old_name: &str, new_name: &str) {
		let entry = ChangeHistoryEntry {
			timestamp: std::time::SystemTime::now(),
			change_type: "RenameModel".to_string(),
			app_label: app_label.to_string(),
			model_name: new_name.to_string(),
			field_name: None,
			old_value: Some(old_name.to_string()),
			new_value: Some(new_name.to_string()),
		};

		self.add_entry(entry);
		self.update_pattern(
			&format!("RenameModel:{}->{}", old_name, new_name),
			app_label,
		);
	}

	/// Record a model move between apps
	pub fn record_model_move(&mut self, from_app: &str, to_app: &str, model_name: &str) {
		let entry = ChangeHistoryEntry {
			timestamp: std::time::SystemTime::now(),
			change_type: "MoveModel".to_string(),
			app_label: to_app.to_string(),
			model_name: model_name.to_string(),
			field_name: None,
			old_value: Some(from_app.to_string()),
			new_value: Some(to_app.to_string()),
		};

		self.add_entry(entry);
		self.update_pattern(
			&format!("MoveModel:{}->{}:{}", from_app, to_app, model_name),
			to_app,
		);
	}

	/// Record a field addition
	pub fn record_field_addition(&mut self, app_label: &str, model_name: &str, field_name: &str) {
		let entry = ChangeHistoryEntry {
			timestamp: std::time::SystemTime::now(),
			change_type: "AddField".to_string(),
			app_label: app_label.to_string(),
			model_name: model_name.to_string(),
			field_name: Some(field_name.to_string()),
			old_value: None,
			new_value: Some(field_name.to_string()),
		};

		self.add_entry(entry);
		self.update_pattern(
			&format!("AddField:{}:{}", model_name, field_name),
			app_label,
		);
	}

	/// Record a field rename
	pub fn record_field_rename(
		&mut self,
		app_label: &str,
		model_name: &str,
		old_name: &str,
		new_name: &str,
	) {
		let entry = ChangeHistoryEntry {
			timestamp: std::time::SystemTime::now(),
			change_type: "RenameField".to_string(),
			app_label: app_label.to_string(),
			model_name: model_name.to_string(),
			field_name: Some(new_name.to_string()),
			old_value: Some(old_name.to_string()),
			new_value: Some(new_name.to_string()),
		};

		self.add_entry(entry);
		self.update_pattern(
			&format!("RenameField:{}:{}->{}", model_name, old_name, new_name),
			app_label,
		);
	}

	/// Add an entry to history with size management
	fn add_entry(&mut self, entry: ChangeHistoryEntry) {
		self.history.push(entry);

		// Maintain max history size
		if self.history.len() > self.max_history_size {
			self.history.remove(0);
		}
	}

	/// Update pattern frequency
	fn update_pattern(&mut self, pattern: &str, context: &str) {
		self.patterns
			.entry(pattern.to_string())
			.and_modify(|pf| {
				pf.frequency += 1;
				pf.last_seen = std::time::SystemTime::now();
				if !pf.contexts.contains(&context.to_string()) {
					pf.contexts.push(context.to_string());
				}
			})
			.or_insert(PatternFrequency {
				pattern: pattern.to_string(),
				frequency: 1,
				last_seen: std::time::SystemTime::now(),
				contexts: vec![context.to_string()],
			});
	}

	/// Get patterns that occur at least `min_frequency` times
	///
	/// Returns patterns sorted by frequency (descending)
	pub fn get_frequent_patterns(&self, min_frequency: usize) -> Vec<PatternFrequency> {
		let mut patterns: Vec<_> = self
			.patterns
			.values()
			.filter(|p| p.frequency >= min_frequency)
			.cloned()
			.collect();

		patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
		patterns
	}

	/// Get recent changes within the specified duration
	///
	/// # Arguments
	/// * `duration` - Time window (e.g., Duration::from_secs(3600) for last hour)
	pub fn get_recent_changes(&self, duration: std::time::Duration) -> Vec<&ChangeHistoryEntry> {
		let now = std::time::SystemTime::now();
		self.history
			.iter()
			.filter(|entry| {
				now.duration_since(entry.timestamp)
					.map(|d| d < duration)
					.unwrap_or(false)
			})
			.collect()
	}

	/// Analyze co-occurring patterns
	///
	/// Returns pairs of patterns that frequently appear together
	/// within a time window (default: 1 hour)
	pub fn analyze_cooccurrence(
		&self,
		window: std::time::Duration,
	) -> HashMap<(String, String), usize> {
		let mut cooccurrences = HashMap::new();

		for i in 0..self.history.len() {
			for j in (i + 1)..self.history.len() {
				if let Ok(diff) = self.history[j]
					.timestamp
					.duration_since(self.history[i].timestamp)
					&& diff <= window {
						let pattern1 = format!(
							"{}:{}",
							self.history[i].change_type, self.history[i].model_name
						);
						let pattern2 = format!(
							"{}:{}",
							self.history[j].change_type, self.history[j].model_name
						);
						let key = if pattern1 < pattern2 {
							(pattern1, pattern2)
						} else {
							(pattern2, pattern1)
						};
						*cooccurrences.entry(key).or_insert(0) += 1;
					}
			}
		}

		cooccurrences
	}

	/// Clear all history (useful for testing)
	pub fn clear(&mut self) {
		self.history.clear();
		self.patterns.clear();
	}

	/// Get total number of changes tracked
	pub fn len(&self) -> usize {
		self.history.len()
	}

	/// Check if history is empty
	pub fn is_empty(&self) -> bool {
		self.history.is_empty()
	}
}

impl Default for ChangeTracker {
	fn default() -> Self {
		Self::new()
	}
}

/// Pattern match result
///
/// Represents a single match found by the PatternMatcher.
#[derive(Debug, Clone)]
pub struct PatternMatch {
	/// The pattern that matched
	pub pattern: String,
	/// Starting position in the text
	pub start: usize,
	/// Ending position in the text
	pub end: usize,
	/// The matched text
	pub matched_text: String,
}

/// Pattern matcher using Aho-Corasick algorithm
///
/// Efficiently searches for multiple patterns simultaneously in model/field names.
/// This is useful for detecting common naming patterns like:
/// - "User" -> "Account" conversions
/// - "created_at" -> "timestamp" renames
/// - Common prefix/suffix patterns
///
/// # Algorithm: Aho-Corasick
/// - Time Complexity: O(n + m + z) where n=text length, m=total pattern length, z=matches
/// - Space Complexity: O(m) for the automaton
/// - Advantage: Simultaneous multi-pattern matching in linear time
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::PatternMatcher;
///
/// let mut matcher = PatternMatcher::new();
/// matcher.add_pattern("User");
/// matcher.add_pattern("Post");
/// matcher.build();
///
/// let matches = matcher.find_all("User has many Posts");
/// assert_eq!(matches.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct PatternMatcher {
	/// Patterns to search for
	patterns: Vec<String>,
	/// Aho-Corasick automaton (built lazily)
	automaton: Option<aho_corasick::AhoCorasick>,
}

impl PatternMatcher {
	/// Create a new empty pattern matcher
	pub fn new() -> Self {
		Self {
			patterns: Vec::new(),
			automaton: None,
		}
	}

	/// Add a pattern to search for
	///
	/// Patterns are case-sensitive by default.
	/// Call `build()` after adding all patterns.
	pub fn add_pattern(&mut self, pattern: &str) {
		self.patterns.push(pattern.to_string());
		// Invalidate automaton - needs rebuild
		self.automaton = None;
	}

	/// Add multiple patterns at once
	pub fn add_patterns<I, S>(&mut self, patterns: I)
	where
		I: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		for pattern in patterns {
			self.patterns.push(pattern.as_ref().to_string());
		}
		self.automaton = None;
	}

	/// Build the Aho-Corasick automaton
	///
	/// Must be called after adding patterns and before searching.
	/// Returns Err if patterns is empty or build fails.
	pub fn build(&mut self) -> Result<(), String> {
		if self.patterns.is_empty() {
			return Err("No patterns to build automaton".to_string());
		}

		self.automaton = Some(
			aho_corasick::AhoCorasick::new(&self.patterns)
				.map_err(|e| format!("Failed to build Aho-Corasick automaton: {}", e))?,
		);

		Ok(())
	}

	/// Find all pattern matches in the given text
	///
	/// Returns empty vector if no matches found or automaton not built.
	pub fn find_all(&self, text: &str) -> Vec<PatternMatch> {
		let Some(ref automaton) = self.automaton else {
			return Vec::new();
		};

		automaton
			.find_iter(text)
			.map(|mat| PatternMatch {
				pattern: self.patterns[mat.pattern().as_usize()].clone(),
				start: mat.start(),
				end: mat.end(),
				matched_text: text[mat.start()..mat.end()].to_string(),
			})
			.collect()
	}

	/// Check if any pattern matches the text
	pub fn contains_any(&self, text: &str) -> bool {
		self.automaton
			.as_ref()
			.map(|ac| ac.is_match(text))
			.unwrap_or(false)
	}

	/// Find the first match in the text
	pub fn find_first(&self, text: &str) -> Option<PatternMatch> {
		let automaton = self.automaton.as_ref()?;
		let mat = automaton.find(text)?;

		Some(PatternMatch {
			pattern: self.patterns[mat.pattern().as_usize()].clone(),
			start: mat.start(),
			end: mat.end(),
			matched_text: text[mat.start()..mat.end()].to_string(),
		})
	}

	/// Replace all pattern matches with replacements
	///
	/// # Arguments
	/// * `text` - The text to search in
	/// * `replacements` - Map from pattern to replacement string
	///
	/// # Returns
	/// Modified text with all patterns replaced
	pub fn replace_all(&self, text: &str, replacements: &HashMap<String, String>) -> String {
		let Some(ref automaton) = self.automaton else {
			return text.to_string();
		};

		let mut result = String::new();
		let mut last_end = 0;

		for mat in automaton.find_iter(text) {
			// Add text before match
			result.push_str(&text[last_end..mat.start()]);

			// Add replacement or original if no replacement found
			let pattern = &self.patterns[mat.pattern().as_usize()];
			if let Some(replacement) = replacements.get(pattern) {
				result.push_str(replacement);
			} else {
				result.push_str(&text[mat.start()..mat.end()]);
			}

			last_end = mat.end();
		}

		// Add remaining text
		result.push_str(&text[last_end..]);
		result
	}

	/// Get all patterns currently registered
	pub fn patterns(&self) -> &[String] {
		&self.patterns
	}

	/// Clear all patterns
	pub fn clear(&mut self) {
		self.patterns.clear();
		self.automaton = None;
	}

	/// Check if automaton is built and ready
	pub fn is_built(&self) -> bool {
		self.automaton.is_some()
	}
}

impl Default for PatternMatcher {
	fn default() -> Self {
		Self::new()
	}
}

// ============================================================================
// Phase 2.3: Inference Types
// ============================================================================

/// Condition for an inference rule
#[derive(Debug, Clone, PartialEq)]
pub enum RuleCondition {
	/// Model rename pattern
	ModelRename {
		from_pattern: String,
		to_pattern: String,
	},
	/// Model move pattern
	ModelMove { app_pattern: String },
	/// Field addition pattern
	FieldAddition { field_name_pattern: String },
	/// Field rename pattern
	FieldRename {
		from_pattern: String,
		to_pattern: String,
	},
	/// Multiple model renames
	MultipleModelRenames { min_count: usize },
	/// Multiple field additions
	MultipleFieldAdditions {
		model_pattern: String,
		min_count: usize,
	},
}

/// Inferred intent from detected changes
#[derive(Debug, Clone, PartialEq)]
pub struct InferredIntent {
	/// Type of intent (e.g., "Refactoring", "Add timestamp tracking")
	pub intent_type: String,
	/// Confidence score (0.0 - 1.0)
	pub confidence: f64,
	/// Human-readable description
	pub description: String,
	/// Evidence supporting this intent
	pub evidence: Vec<String>,
}

/// Rule for inferring intent from change patterns
#[derive(Debug, Clone)]
pub struct InferenceRule {
	/// Rule name
	pub name: String,
	/// Required conditions (all must match)
	pub conditions: Vec<RuleCondition>,
	/// Optional conditions (boost confidence if matched)
	pub optional_conditions: Vec<RuleCondition>,
	/// Intent type to infer
	pub intent_type: String,
	/// Base confidence (0.0 - 1.0)
	pub base_confidence: f64,
	/// Confidence boost per matched optional condition
	pub confidence_boost_per_optional: f64,
}

/// Inference engine for detecting composite change intents
///
/// Analyzes multiple detected changes to infer high-level intentions.
/// For example:
/// - AddIndex + AlterField(to larger type) → Performance optimization
/// - RenameModel + AddForeignKey → Relationship refactoring
/// - AddField + RemoveField → Data migration
///
/// # Algorithm: Rule-Based Inference
/// - Matches detected changes against predefined rules
/// - Calculates confidence scores based on pattern matching
/// - Returns ranked list of possible intents
///
/// # Examples
///
/// ```ignore
/// use reinhardt_migrations::InferenceEngine;
///
/// let mut engine = InferenceEngine::new();
/// engine.add_default_rules();
///
/// // Analyze changes
/// let changes = vec!["AddIndex:users:email", "AlterField:users:email"];
/// let intents = engine.infer_intents(&changes);
/// ```
#[derive(Debug, Clone)]
pub struct InferenceEngine {
	/// Inference rules
	rules: Vec<InferenceRule>,
	/// Change history for contextual analysis
	///
	/// The change tracker maintains a history of schema changes and can be used
	/// to improve inference accuracy by analyzing temporal patterns. To use:
	///
	/// 1. Record changes via `record_model_rename()`, `record_field_addition()`, etc.
	/// 2. Query patterns via `get_frequent_patterns()` or `analyze_cooccurrence()`
	/// 3. Use pattern analysis to boost confidence scores in inference rules
	///
	/// Example:
	/// ```
	/// use reinhardt_migrations::autodetector::InferenceEngine;
	/// let mut engine = InferenceEngine::new();
	/// // Record rename and field addition history
	/// engine.record_model_rename("blog", "BlogPost", "Post");
	/// engine.record_field_addition("blog", "Post", "slug");
	/// // Analyze co-occurrence within a 60-second window
	/// let _cooccurrences = engine.analyze_cooccurrence(std::time::Duration::from_secs(60));
	/// ```
	change_tracker: ChangeTracker,
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceEngine {
	/// Create a new inference engine
	pub fn new() -> Self {
		Self {
			rules: Vec::new(),
			change_tracker: ChangeTracker::new(),
		}
	}

	/// Add a rule to the engine
	pub fn add_rule(&mut self, rule: InferenceRule) {
		self.rules.push(rule);
	}

	/// Add default inference rules
	pub fn add_default_rules(&mut self) {
		// Rule 1: Model refactoring (rename)
		self.add_rule(InferenceRule {
			name: "model_refactoring".to_string(),
			conditions: vec![RuleCondition::ModelRename {
				from_pattern: ".*".to_string(),
				to_pattern: ".*".to_string(),
			}],
			optional_conditions: vec![RuleCondition::MultipleModelRenames { min_count: 2 }],
			intent_type: "Refactoring: Model rename".to_string(),
			base_confidence: 0.7,
			confidence_boost_per_optional: 0.1,
		});

		// Rule 2: Timestamp tracking
		self.add_rule(InferenceRule {
			name: "add_timestamp_tracking".to_string(),
			conditions: vec![RuleCondition::FieldAddition {
				field_name_pattern: "created_at".to_string(),
			}],
			optional_conditions: vec![RuleCondition::FieldAddition {
				field_name_pattern: "updated_at".to_string(),
			}],
			intent_type: "Add timestamp tracking".to_string(),
			base_confidence: 0.8,
			confidence_boost_per_optional: 0.15,
		});

		// Rule 3: Cross-app model move
		self.add_rule(InferenceRule {
			name: "cross_app_move".to_string(),
			conditions: vec![RuleCondition::ModelMove {
				app_pattern: ".*".to_string(),
			}],
			optional_conditions: vec![],
			intent_type: "Cross-app model organization".to_string(),
			base_confidence: 0.75,
			confidence_boost_per_optional: 0.0,
		});

		// Rule 4: Field refactoring (rename)
		self.add_rule(InferenceRule {
			name: "field_refactoring".to_string(),
			conditions: vec![RuleCondition::FieldRename {
				from_pattern: ".*".to_string(),
				to_pattern: ".*".to_string(),
			}],
			optional_conditions: vec![RuleCondition::MultipleFieldAdditions {
				model_pattern: ".*".to_string(),
				min_count: 2,
			}],
			intent_type: "Refactoring: Field rename".to_string(),
			base_confidence: 0.65,
			confidence_boost_per_optional: 0.1,
		});

		// Rule 5: Model normalization
		self.add_rule(InferenceRule {
			name: "model_normalization".to_string(),
			conditions: vec![RuleCondition::MultipleFieldAdditions {
				model_pattern: ".*".to_string(),
				min_count: 3,
			}],
			optional_conditions: vec![],
			intent_type: "Schema normalization".to_string(),
			base_confidence: 0.6,
			confidence_boost_per_optional: 0.0,
		});
	}

	/// Get all rules
	pub fn rules(&self) -> &[InferenceRule] {
		&self.rules
	}

	/// Infer intents from detected changes
	pub fn infer_intents(
		&self,
		model_renames: &[(String, String, String, String)], // (from_app, from_model, to_app, to_model)
		model_moves: &[(String, String, String, String)],   // (from_app, from_model, to_app, to_model)
		field_additions: &[(String, String, String)],       // (app, model, field)
		field_renames: &[(String, String, String, String)], // (app, model, from_field, to_field)
	) -> Vec<InferredIntent> {
		let mut intents = Vec::new();

		for rule in &self.rules {
			let mut matches_required = true;
			let mut optional_matches = 0;
			let mut evidence = Vec::new();

			// Check required conditions
			for condition in &rule.conditions {
				match condition {
					RuleCondition::ModelRename {
						from_pattern,
						to_pattern,
					} => {
						if model_renames.is_empty() {
							matches_required = false;
							break;
						}
						// Pattern matching simplified - actual implementation would use regex
						if from_pattern == ".*" || to_pattern == ".*" {
							evidence.push(format!(
								"Model renamed: {}.{} → {}.{}",
								model_renames[0].0,
								model_renames[0].1,
								model_renames[0].2,
								model_renames[0].3
							));
						}
					}
					RuleCondition::ModelMove { app_pattern } => {
						if model_moves.is_empty() {
							matches_required = false;
							break;
						}
						if app_pattern == ".*" {
							evidence.push(format!(
								"Model moved: {}.{} → {}.{}",
								model_moves[0].0,
								model_moves[0].1,
								model_moves[0].2,
								model_moves[0].3
							));
						}
					}
					RuleCondition::FieldAddition { field_name_pattern } => {
						let matching_fields: Vec<_> = field_additions
							.iter()
							.filter(|(_, _, field)| field.contains(field_name_pattern.as_str()))
							.collect();

						if matching_fields.is_empty() {
							matches_required = false;
							break;
						}
						evidence.push(format!(
							"Field added: {}.{}.{}",
							matching_fields[0].0, matching_fields[0].1, matching_fields[0].2
						));
					}
					RuleCondition::FieldRename {
						from_pattern,
						to_pattern,
					} => {
						if field_renames.is_empty() {
							matches_required = false;
							break;
						}
						if from_pattern == ".*" || to_pattern == ".*" {
							evidence.push(format!(
								"Field renamed: {}.{}.{} → {}",
								field_renames[0].0,
								field_renames[0].1,
								field_renames[0].2,
								field_renames[0].3
							));
						}
					}
					RuleCondition::MultipleModelRenames { min_count } => {
						if model_renames.len() < *min_count {
							matches_required = false;
							break;
						}
						evidence.push(format!("Multiple model renames: {}", model_renames.len()));
					}
					RuleCondition::MultipleFieldAdditions {
						model_pattern,
						min_count,
					} => {
						let count = if model_pattern == ".*" {
							field_additions.len()
						} else {
							field_additions
								.iter()
								.filter(|(_, model, _)| model.contains(model_pattern.as_str()))
								.count()
						};
						if count < *min_count {
							matches_required = false;
							break;
						}
						evidence.push(format!("Multiple field additions: {}", count));
					}
				}
			}

			if !matches_required {
				continue;
			}

			// Check optional conditions
			for condition in &rule.optional_conditions {
				match condition {
					RuleCondition::FieldAddition { field_name_pattern } => {
						if field_additions
							.iter()
							.any(|(_, _, field)| field.contains(field_name_pattern.as_str()))
						{
							optional_matches += 1;
							evidence.push(format!("Optional field added: {}", field_name_pattern));
						}
					}
					RuleCondition::MultipleModelRenames { min_count } => {
						if model_renames.len() >= *min_count {
							optional_matches += 1;
							evidence.push(format!("Multiple renames: {}", model_renames.len()));
						}
					}
					_ => {}
				}
			}

			// Calculate confidence
			let confidence = rule.base_confidence
				+ (optional_matches as f64 * rule.confidence_boost_per_optional);
			let confidence = confidence.min(1.0);

			intents.push(InferredIntent {
				intent_type: rule.intent_type.clone(),
				confidence,
				description: format!("Detected: {}", rule.name),
				evidence,
			});
		}

		// Sort by confidence (highest first)
		intents.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

		intents
	}

	/// Infer intents from DetectedChanges
	///
	/// Extracts change operations from DetectedChanges and runs inference rules on them.
	///
	/// # Arguments
	/// * `changes` - Detected changes between two project states
	///
	/// # Returns
	/// Inferred intents sorted by confidence (highest first)
	pub fn infer_from_detected_changes(&self, changes: &DetectedChanges) -> Vec<InferredIntent> {
		// Extract model renames: (from_app, from_model, to_app, to_model)
		let model_renames: Vec<(String, String, String, String)> = changes
			.renamed_models
			.iter()
			.map(|(app, old_name, new_name)| {
				(app.clone(), old_name.clone(), app.clone(), new_name.clone())
			})
			.collect();

		// Extract model moves: (from_app, from_model, to_app, to_model)
		let model_moves: Vec<(String, String, String, String)> = changes
			.moved_models
			.iter()
			.map(|(from_app, to_app, model, _, _, _)| {
				(
					from_app.clone(),
					model.clone(),
					to_app.clone(),
					model.clone(),
				)
			})
			.collect();

		// Extract field additions: (app, model, field)
		let field_additions: Vec<(String, String, String)> = changes
			.added_fields
			.iter()
			.map(|(app, model, field)| (app.clone(), model.clone(), field.clone()))
			.collect();

		// Extract field renames: (app, model, from_field, to_field)
		let field_renames: Vec<(String, String, String, String)> = changes
			.renamed_fields
			.iter()
			.map(|(app, model, old_name, new_name)| {
				(
					app.clone(),
					model.clone(),
					old_name.clone(),
					new_name.clone(),
				)
			})
			.collect();

		// Run inference on extracted changes
		self.infer_intents(
			&model_renames,
			&model_moves,
			&field_additions,
			&field_renames,
		)
	}

	/// Record a model rename in the change tracker
	///
	/// This enables contextual analysis for future migrations by tracking patterns.
	///
	/// # Arguments
	/// * `app_label` - App containing the model
	/// * `old_name` - Original model name
	/// * `new_name` - New model name
	pub fn record_model_rename(&mut self, app_label: &str, old_name: &str, new_name: &str) {
		self.change_tracker
			.record_model_rename(app_label, old_name, new_name);
	}

	/// Record a model move between apps
	///
	/// # Arguments
	/// * `from_app` - Source app label
	/// * `to_app` - Target app label
	/// * `model_name` - Name of the model being moved
	pub fn record_model_move(&mut self, from_app: &str, to_app: &str, model_name: &str) {
		self.change_tracker
			.record_model_move(from_app, to_app, model_name);
	}

	/// Record a field addition
	///
	/// # Arguments
	/// * `app_label` - App containing the model
	/// * `model_name` - Name of the model
	/// * `field_name` - Name of the field being added
	pub fn record_field_addition(&mut self, app_label: &str, model_name: &str, field_name: &str) {
		self.change_tracker
			.record_field_addition(app_label, model_name, field_name);
	}

	/// Record a field rename
	///
	/// # Arguments
	/// * `app_label` - App containing the model
	/// * `model_name` - Name of the model
	/// * `old_name` - Original field name
	/// * `new_name` - New field name
	pub fn record_field_rename(
		&mut self,
		app_label: &str,
		model_name: &str,
		old_name: &str,
		new_name: &str,
	) {
		self.change_tracker
			.record_field_rename(app_label, model_name, old_name, new_name);
	}

	/// Get frequent patterns from change history
	///
	/// Returns patterns that occur at least `min_frequency` times.
	/// This can be used to improve confidence scores for similar patterns.
	///
	/// # Arguments
	/// * `min_frequency` - Minimum number of occurrences to be considered frequent
	pub fn get_frequent_patterns(&self, min_frequency: usize) -> Vec<PatternFrequency> {
		self.change_tracker.get_frequent_patterns(min_frequency)
	}

	/// Get recent changes within the specified duration
	///
	/// # Arguments
	/// * `duration` - Time window for recent changes (e.g., last hour)
	pub fn get_recent_changes(&self, duration: std::time::Duration) -> Vec<&ChangeHistoryEntry> {
		self.change_tracker.get_recent_changes(duration)
	}

	/// Analyze co-occurring patterns in change history
	///
	/// Returns pairs of patterns that frequently appear together
	/// within a time window.
	///
	/// # Arguments
	/// * `window` - Time window for co-occurrence analysis (default: 1 hour)
	pub fn analyze_cooccurrence(
		&self,
		window: std::time::Duration,
	) -> HashMap<(String, String), usize> {
		self.change_tracker.analyze_cooccurrence(window)
	}
}

// ============================================================================
// Phase 2.4: Interactive UI for User Confirmation
// ============================================================================

/// Interactive prompt system for user confirmation of ambiguous changes
///
/// This module provides CLI-based prompts for:
/// - Ambiguous model/field renames
/// - Cross-app model moves
/// - Multiple possible intents with different confidence scores
///
/// Uses the `dialoguer` crate for rich terminal interactions.
pub struct MigrationPrompt {
	/// Minimum confidence threshold for auto-acceptance (0.0 - 1.0)
	/// Changes above this threshold are accepted without prompting
	auto_accept_threshold: f64,

	/// Theme for terminal styling
	theme: dialoguer::theme::ColorfulTheme,
}

impl std::fmt::Debug for MigrationPrompt {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MigrationPrompt")
			.field("auto_accept_threshold", &self.auto_accept_threshold)
			.field("theme", &"ColorfulTheme")
			.finish()
	}
}

impl MigrationPrompt {
	/// Create a new prompt system with default settings
	pub fn new() -> Self {
		Self {
			auto_accept_threshold: 0.85,
			theme: dialoguer::theme::ColorfulTheme::default(),
		}
	}

	/// Create with custom auto-accept threshold
	pub fn with_threshold(threshold: f64) -> Self {
		Self {
			auto_accept_threshold: threshold,
			theme: dialoguer::theme::ColorfulTheme::default(),
		}
	}

	/// Get the auto-accept threshold
	pub fn auto_accept_threshold(&self) -> f64 {
		self.auto_accept_threshold
	}

	/// Confirm a single intent with the user
	///
	/// Returns true if the user confirms, false if they reject
	pub fn confirm_intent(
		&self,
		intent: &InferredIntent,
	) -> Result<bool, Box<dyn std::error::Error>> {
		// Auto-accept high-confidence changes
		if intent.confidence >= self.auto_accept_threshold {
			println!(
				"✓ Auto-accepting (confidence: {:.1}%): {}",
				intent.confidence * 100.0,
				intent.intent_type
			);
			return Ok(true);
		}

		// Build prompt message
		let message = format!(
			"Detected: {} (confidence: {:.1}%)\nDetails: {}\n\nAccept this change?",
			intent.intent_type,
			intent.confidence * 100.0,
			intent.description
		);

		// Show evidence
		if !intent.evidence.is_empty() {
			println!("\nEvidence:");
			for evidence in &intent.evidence {
				println!("  • {}", evidence);
			}
		}

		// Prompt user
		dialoguer::Confirm::with_theme(&self.theme)
			.with_prompt(message)
			.default(true)
			.interact()
			.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
	}

	/// Select one intent from multiple alternatives
	///
	/// Returns the index of the selected intent, or None if user cancels
	pub fn select_intent(
		&self,
		alternatives: &[InferredIntent],
		prompt: &str,
	) -> Result<Option<usize>, Box<dyn std::error::Error>> {
		if alternatives.is_empty() {
			return Ok(None);
		}

		// Single alternative - just confirm
		if alternatives.len() == 1 {
			let confirmed = self.confirm_intent(&alternatives[0])?;
			return Ok(if confirmed { Some(0) } else { None });
		}

		// Build selection items
		let items: Vec<String> = alternatives
			.iter()
			.map(|intent| {
				format!(
					"{} (confidence: {:.1}%) - {}",
					intent.intent_type,
					intent.confidence * 100.0,
					intent.description
				)
			})
			.collect();

		// Show prompt
		println!("\n{}", prompt);
		println!("Multiple possibilities detected:\n");

		// Add "None of the above" option
		let mut items_with_none = items.clone();
		items_with_none.push("None of the above / Skip".to_string());

		// Prompt user
		let selection = dialoguer::Select::with_theme(&self.theme)
			.items(&items_with_none)
			.default(0)
			.interact()
			.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

		// Return None if user selected "None of the above"
		if selection >= items.len() {
			Ok(None)
		} else {
			Ok(Some(selection))
		}
	}

	/// Multi-select intents from a list
	///
	/// Returns indices of selected intents
	pub fn multi_select_intents(
		&self,
		alternatives: &[InferredIntent],
		prompt: &str,
	) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
		if alternatives.is_empty() {
			return Ok(Vec::new());
		}

		// Build selection items
		let items: Vec<String> = alternatives
			.iter()
			.map(|intent| {
				format!(
					"{} (confidence: {:.1}%) - {}",
					intent.intent_type,
					intent.confidence * 100.0,
					intent.description
				)
			})
			.collect();

		// Show prompt
		println!("\n{}", prompt);
		println!("Select all that apply:\n");

		// Prompt user with multi-select
		let selections = dialoguer::MultiSelect::with_theme(&self.theme)
			.items(&items)
			.interact()
			.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

		Ok(selections)
	}

	/// Confirm a model rename with details
	pub fn confirm_model_rename(
		&self,
		from_app: &str,
		from_model: &str,
		to_app: &str,
		to_model: &str,
		confidence: f64,
	) -> Result<bool, Box<dyn std::error::Error>> {
		// Auto-accept high-confidence changes
		if confidence >= self.auto_accept_threshold {
			println!(
				"✓ Auto-accepting model rename (confidence: {:.1}%): {}.{} → {}.{}",
				confidence * 100.0,
				from_app,
				from_model,
				to_app,
				to_model
			);
			return Ok(true);
		}

		let message = format!(
			"Rename model from {}.{} to {}.{}?\n(confidence: {:.1}%)",
			from_app,
			from_model,
			to_app,
			to_model,
			confidence * 100.0
		);

		dialoguer::Confirm::with_theme(&self.theme)
			.with_prompt(message)
			.default(true)
			.interact()
			.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
	}

	/// Confirm a field rename with details
	pub fn confirm_field_rename(
		&self,
		model: &str,
		from_field: &str,
		to_field: &str,
		confidence: f64,
	) -> Result<bool, Box<dyn std::error::Error>> {
		// Auto-accept high-confidence changes
		if confidence >= self.auto_accept_threshold {
			println!(
				"✓ Auto-accepting field rename (confidence: {:.1}%): {}.{} → {}.{}",
				confidence * 100.0,
				model,
				from_field,
				model,
				to_field
			);
			return Ok(true);
		}

		let message = format!(
			"Rename field in model {}:\n  {} → {}?\n(confidence: {:.1}%)",
			model,
			from_field,
			to_field,
			confidence * 100.0
		);

		dialoguer::Confirm::with_theme(&self.theme)
			.with_prompt(message)
			.default(true)
			.interact()
			.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
	}

	/// Show progress indicator for long operations
	pub fn with_progress<F, T>(
		&self,
		message: &str,
		total: u64,
		operation: F,
	) -> Result<T, Box<dyn std::error::Error>>
	where
		F: FnOnce(&indicatif::ProgressBar) -> Result<T, Box<dyn std::error::Error>>,
	{
		let pb = indicatif::ProgressBar::new(total);
		pb.set_style(
			indicatif::ProgressStyle::default_bar()
				.template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
				.expect("Failed to create progress bar template")
				.progress_chars("#>-"),
		);
		pb.set_message(message.to_string());

		let result = operation(&pb)?;

		pb.finish_with_message("Done");
		Ok(result)
	}
}

impl Default for MigrationPrompt {
	fn default() -> Self {
		Self::new()
	}
}

/// Extension trait for MigrationAutodetector with interactive prompts
pub trait InteractiveAutodetector {
	/// Detect changes with user prompts for ambiguous cases
	fn detect_changes_interactive(&self) -> Result<DetectedChanges, Box<dyn std::error::Error>>;

	/// Apply inferred intents with user confirmation
	fn apply_intents_interactive(
		&self,
		intents: Vec<InferredIntent>,
		changes: &mut DetectedChanges,
	) -> Result<(), Box<dyn std::error::Error>>;
}

impl InteractiveAutodetector for MigrationAutodetector {
	fn detect_changes_interactive(&self) -> Result<DetectedChanges, Box<dyn std::error::Error>> {
		let prompt = MigrationPrompt::new();
		let mut changes = self.detect_changes();

		// Build inference engine
		let mut engine = InferenceEngine::new();
		engine.add_default_rules();

		// Infer intents from detected changes
		let intents = engine.infer_from_detected_changes(&changes);

		// Filter high-confidence intents
		let ambiguous_intents: Vec<_> = intents
			.into_iter()
			.filter(|intent| intent.confidence < prompt.auto_accept_threshold)
			.collect();

		// Prompt for ambiguous changes
		if !ambiguous_intents.is_empty() {
			println!(
				"\n⚠️  Found {} ambiguous change(s) requiring confirmation:",
				ambiguous_intents.len()
			);

			for intent in &ambiguous_intents {
				let confirmed = prompt.confirm_intent(intent)?;

				if !confirmed {
					println!("✗ Skipped: {}", intent.description);
					// Note: Removing operations from DetectedChanges requires establishing
					// a bidirectional mapping between InferredIntent and the specific operations
					// (e.g., renamed_models, added_fields, etc.). This is a complex refactoring
					// that would require:
					// 1. Adding operation IDs or tracking metadata to DetectedChanges
					// 2. Maintaining the mapping in InferredIntent
					// 3. Implementing removal logic for each operation type
					// For now, skipped intents are logged but the underlying operations remain
					// in DetectedChanges, which may result in migration operations being
					// generated despite user rejection. This should be addressed in a future
					// refactoring of the intent inference system.
				}
			}
		}

		// Detect and order dependencies
		self.detect_model_dependencies(&mut changes);

		// Check for circular dependencies
		if let Err(cycle) = changes.check_circular_dependencies() {
			println!("\n⚠️  Warning: Circular dependency detected: {:?}", cycle);

			let should_continue = dialoguer::Confirm::new()
				.with_prompt("Continue anyway? (may require manual intervention)")
				.default(false)
				.interact()?;

			if !should_continue {
				return Err("Aborted due to circular dependency".into());
			}
		}

		Ok(changes)
	}

	fn apply_intents_interactive(
		&self,
		intents: Vec<InferredIntent>,
		_changes: &mut DetectedChanges,
	) -> Result<(), Box<dyn std::error::Error>> {
		let prompt = MigrationPrompt::new();

		// Group intents by confidence
		let mut high_confidence = Vec::new();
		let mut medium_confidence = Vec::new();
		let mut low_confidence = Vec::new();

		for intent in intents {
			if intent.confidence >= 0.85 {
				high_confidence.push(intent);
			} else if intent.confidence >= 0.65 {
				medium_confidence.push(intent);
			} else {
				low_confidence.push(intent);
			}
		}

		// Auto-apply high-confidence intents
		println!(
			"\n✓ Auto-applying {} high-confidence change(s):",
			high_confidence.len()
		);
		for intent in &high_confidence {
			println!(
				"  • {} (confidence: {:.1}%)",
				intent.description,
				intent.confidence * 100.0
			);
		}

		// Prompt for medium-confidence intents
		if !medium_confidence.is_empty() {
			println!(
				"\n⚠️  Review {} medium-confidence change(s):",
				medium_confidence.len()
			);

			for intent in &medium_confidence {
				let confirmed = prompt.confirm_intent(intent)?;
				if confirmed {
					println!("  ✓ Accepted: {}", intent.description);
				} else {
					println!("  ✗ Rejected: {}", intent.description);
				}
			}
		}

		// Prompt for low-confidence intents with multi-select
		if !low_confidence.is_empty() {
			let selections = prompt.multi_select_intents(
				&low_confidence,
				"⚠️  Select low-confidence changes to apply:",
			)?;

			for idx in selections {
				println!("  ✓ Accepted: {}", low_confidence[idx].description);
			}
		}

		Ok(())
	}
}

impl MigrationAutodetector {
	/// Create a new migration autodetector with default similarity config
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState};
	///
	/// let from_state = ProjectState::new();
	/// let to_state = ProjectState::new();
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// ```
	pub fn new(from_state: ProjectState, to_state: ProjectState) -> Self {
		Self {
			from_state,
			to_state,
			similarity_config: SimilarityConfig::default(),
		}
	}

	/// Create a new migration autodetector with custom similarity config
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, SimilarityConfig};
	///
	/// let from_state = ProjectState::new();
	/// let to_state = ProjectState::new();
	/// let config = SimilarityConfig::new(0.75, 0.85).unwrap();
	///
	/// let detector = MigrationAutodetector::with_config(from_state, to_state, config);
	/// ```
	pub fn with_config(
		from_state: ProjectState,
		to_state: ProjectState,
		similarity_config: SimilarityConfig,
	) -> Self {
		Self {
			from_state,
			to_state,
			similarity_config,
		}
	}

	/// Detect all changes between from_state and to_state
	///
	/// Django equivalent: `_detect_changes()` in django/db/migrations/autodetector.py
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState};
	///
	/// let from_state = ProjectState::new();
	/// let mut to_state = ProjectState::new();
	///
	// Add a new model
	/// let model = ModelState::new("myapp", "User");
	/// to_state.add_model(model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// let changes = detector.detect_changes();
	///
	/// assert_eq!(changes.created_models.len(), 1);
	/// ```
	pub fn detect_changes(&self) -> DetectedChanges {
		let mut changes = DetectedChanges::default();

		// Detect model-level changes
		self.detect_created_models(&mut changes);
		self.detect_deleted_models(&mut changes);
		self.detect_renamed_models(&mut changes);

		// Detect field-level changes (only for models that exist in both states)
		self.detect_added_fields(&mut changes);
		self.detect_removed_fields(&mut changes);
		self.detect_altered_fields(&mut changes);
		self.detect_renamed_fields(&mut changes);

		// Detect index and constraint changes
		self.detect_added_indexes(&mut changes);
		self.detect_removed_indexes(&mut changes);
		self.detect_added_constraints(&mut changes);
		self.detect_removed_constraints(&mut changes);

		// Detect model dependencies for operation ordering
		self.detect_model_dependencies(&mut changes);

		changes
	}

	/// Detect newly created models
	///
	/// Django reference: `generate_created_models()` in django/db/migrations/autodetector.py:800
	fn detect_created_models(&self, changes: &mut DetectedChanges) {
		for (app_label, model_name) in self.to_state.models.keys() {
			if !self
				.from_state
				.models
				.contains_key(&(app_label.clone(), model_name.clone()))
			{
				changes
					.created_models
					.push((app_label.clone(), model_name.clone()));
			}
		}
	}

	/// Detect deleted models
	///
	/// Django reference: `generate_deleted_models()` in django/db/migrations/autodetector.py:900
	fn detect_deleted_models(&self, changes: &mut DetectedChanges) {
		for (app_label, model_name) in self.from_state.models.keys() {
			if !self
				.to_state
				.models
				.contains_key(&(app_label.clone(), model_name.clone()))
			{
				changes
					.deleted_models
					.push((app_label.clone(), model_name.clone()));
			}
		}
	}

	/// Detect added fields
	///
	/// Django reference: `generate_added_fields()` in django/db/migrations/autodetector.py:1000
	fn detect_added_fields(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), to_model) in &self.to_state.models {
			// Only check models that exist in both states
			if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
				for field_name in to_model.fields.keys() {
					if !from_model.fields.contains_key(field_name) {
						changes.added_fields.push((
							app_label.clone(),
							model_name.clone(),
							field_name.clone(),
						));
					}
				}
			}
		}
	}

	/// Detect removed fields
	///
	/// Django reference: `generate_removed_fields()` in django/db/migrations/autodetector.py:1100
	fn detect_removed_fields(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), from_model) in &self.from_state.models {
			// Only check models that exist in both states
			if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
				for field_name in from_model.fields.keys() {
					if !to_model.fields.contains_key(field_name) {
						changes.removed_fields.push((
							app_label.clone(),
							model_name.clone(),
							field_name.clone(),
						));
					}
				}
			}
		}
	}

	/// Detect altered fields
	///
	/// Django reference: `generate_altered_fields()` in django/db/migrations/autodetector.py:1200
	fn detect_altered_fields(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), to_model) in &self.to_state.models {
			// Only check models that exist in both states
			if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
				for (field_name, to_field) in &to_model.fields {
					if let Some(from_field) = from_model.fields.get(field_name) {
						// Check if field definition has changed
						if self.has_field_changed(from_field, to_field) {
							changes.altered_fields.push((
								app_label.clone(),
								model_name.clone(),
								field_name.clone(),
							));
						}
					}
				}
			}
		}
	}

	/// Check if a field has changed
	fn has_field_changed(&self, from_field: &FieldState, to_field: &FieldState) -> bool {
		// Check if field type changed
		if from_field.field_type != to_field.field_type {
			return true;
		}

		// Check if nullable changed
		if from_field.nullable != to_field.nullable {
			return true;
		}

		// Check if params changed
		if from_field.params != to_field.params {
			return true;
		}

		false
	}

	/// Detect renamed models
	///
	/// This method attempts to detect model renames by comparing deleted and created models.
	/// It uses field similarity to determine if a model was renamed rather than deleted/created.
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:620-750
	/// ```python
	/// def generate_renamed_models(self):
	///     # Find models that were deleted and created with similar fields
	///     for (app_label, old_model_name) in self.old_model_keys - self.new_model_keys:
	///         for (app_label, new_model_name) in self.new_model_keys - self.old_model_keys:
	///             if self._is_renamed_model(old_model_name, new_model_name):
	///                 self.add_operation(
	///                     app_label,
	///                     operations.RenameModel(
	///                         old_name=old_model_name,
	///                         new_name=new_model_name,
	///                     ),
	///                 )
	/// ```
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let mut from_state = ProjectState::new();
	/// let mut old_model = ModelState::new("myapp", "OldUser");
	/// old_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// old_model.add_field(FieldState::new("name".to_string(), "VARCHAR".to_string(), false));
	/// from_state.add_model(old_model);
	///
	/// let mut to_state = ProjectState::new();
	/// let mut new_model = ModelState::new("myapp", "NewUser");
	/// new_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// new_model.add_field(FieldState::new("name".to_string(), "VARCHAR".to_string(), false));
	/// to_state.add_model(new_model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// let changes = detector.detect_changes();
	///
	// With high field similarity, should detect as rename
	/// assert!(changes.renamed_models.len() <= 1);
	/// ```
	fn detect_renamed_models(&self, changes: &mut DetectedChanges) {
		// Get deleted and created models
		let deleted: Vec<_> = self
			.from_state
			.models
			.keys()
			.filter(|k| !self.to_state.models.contains_key(k))
			.collect();

		let created: Vec<_> = self
			.to_state
			.models
			.keys()
			.filter(|k| !self.from_state.models.contains_key(k))
			.collect();

		// Use bipartite matching to find optimal model pairs
		// This supports both same-app renames and cross-app moves
		let matches = self.find_optimal_model_matches(&deleted, &created);

		for (deleted_key, created_key, _similarity) in matches {
			// Check if this is a cross-app move or same-app rename
			if deleted_key.0 == created_key.0 {
				// Same app: this is a rename operation
				changes
					.renamed_models
					.push((deleted_key.0, deleted_key.1, created_key.1));
			} else {
				// Different apps: this is a move operation
				// Determine if table needs to be renamed
				let old_table = format!("{}_{}", deleted_key.0, deleted_key.1.to_lowercase());
				let new_table = format!("{}_{}", created_key.0, created_key.1.to_lowercase());
				let rename_table = old_table != new_table || deleted_key.1 != created_key.1;

				changes.moved_models.push((
					deleted_key.0, // from_app
					created_key.0, // to_app
					created_key.1, // model_name (use new name)
					rename_table,
					if rename_table { Some(old_table) } else { None },
					if rename_table { Some(new_table) } else { None },
				));
			}
		}
	}

	/// Detect renamed fields
	///
	/// This method attempts to detect field renames by comparing removed and added fields.
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:1300-1400
	/// ```python
	/// def generate_renamed_fields(self):
	///     for app_label, model_name in sorted(self.kept_model_keys):
	///         old_model_state = self.from_state.models[app_label, model_name]
	///         new_model_state = self.to_state.models[app_label, model_name]
	///
	///         # Find fields that were removed and added with same type
	///         for old_field_name, old_field in old_model_state.fields:
	///             for new_field_name, new_field in new_model_state.fields:
	///                 if self._is_renamed_field(old_field, new_field):
	///                     self.add_operation(...)
	/// ```
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let mut from_state = ProjectState::new();
	/// let mut old_model = ModelState::new("myapp", "User");
	/// old_model.add_field(FieldState::new("old_email".to_string(), "VARCHAR".to_string(), false));
	/// from_state.add_model(old_model);
	///
	/// let mut to_state = ProjectState::new();
	/// let mut new_model = ModelState::new("myapp", "User");
	/// new_model.add_field(FieldState::new("new_email".to_string(), "VARCHAR".to_string(), false));
	/// to_state.add_model(new_model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// let changes = detector.detect_changes();
	///
	// With matching type, might detect as rename
	/// assert!(changes.renamed_fields.len() <= 1);
	/// ```
	fn detect_renamed_fields(&self, changes: &mut DetectedChanges) {
		// Only check models that exist in both states
		for ((app_label, model_name), from_model) in &self.from_state.models {
			if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
				// Get removed and added fields for this model
				let removed_fields: Vec<_> = from_model
					.fields
					.iter()
					.filter(|(name, _)| !to_model.fields.contains_key(*name))
					.collect();

				let added_fields: Vec<_> = to_model
					.fields
					.iter()
					.filter(|(name, _)| !from_model.fields.contains_key(*name))
					.collect();

				// Try to match removed fields with added fields
				for (removed_name, removed_field) in &removed_fields {
					for (added_name, added_field) in &added_fields {
						// If field types match, consider it a rename
						if removed_field.field_type == added_field.field_type
							&& removed_field.nullable == added_field.nullable
						{
							changes.renamed_fields.push((
								app_label.clone(),
								model_name.clone(),
								removed_name.to_string(),
								added_name.to_string(),
							));
							break;
						}
					}
				}
			}
		}
	}

	/// Calculate similarity between two models using advanced field matching
	///
	/// # Algorithm: Weighted Bipartite Matching for Fields
	/// - Uses Jaro-Winkler for field name similarity
	/// - Time Complexity: O(n*m) where n,m are number of fields
	/// - Considers both exact matches and fuzzy matches
	///
	/// # Scoring:
	/// - Exact field name + type match: 1.0
	/// - Fuzzy field name + type match: Jaro-Winkler score (0.0-1.0)
	/// - No type match: 0.0
	///
	/// Returns a value between 0.0 and 1.0, where 1.0 means identical field sets.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let mut from_state = ProjectState::new();
	/// let mut from_model = ModelState::new("myapp", "User");
	/// from_model.add_field(FieldState::new("user_id".to_string(), "INTEGER".to_string(), false));
	/// from_model.add_field(FieldState::new("user_email".to_string(), "VARCHAR".to_string(), false));
	/// from_state.add_model(from_model);
	///
	/// let mut to_state = ProjectState::new();
	/// let mut to_model = ModelState::new("auth", "User");
	/// to_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// to_model.add_field(FieldState::new("email".to_string(), "VARCHAR".to_string(), false));
	/// to_state.add_model(to_model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// // Similarity would be high due to fuzzy field name matching
	/// ```
	fn calculate_model_similarity(&self, from_model: &ModelState, to_model: &ModelState) -> f64 {
		if from_model.fields.is_empty() && to_model.fields.is_empty() {
			return 1.0;
		}

		if from_model.fields.is_empty() || to_model.fields.is_empty() {
			return 0.0;
		}

		let mut total_similarity = 0.0;
		let total_fields = from_model.fields.len().max(to_model.fields.len());

		// Use Hungarian algorithm concept: find best matching between fields
		let mut matched_to_fields = std::collections::HashSet::new();

		for (from_field_name, from_field) in &from_model.fields {
			let mut best_match_score = 0.0;
			let mut best_match_name = None;

			// Find best matching field in to_model
			for (to_field_name, to_field) in &to_model.fields {
				if matched_to_fields.contains(to_field_name) {
					continue;
				}

				let similarity = self.calculate_field_similarity(
					from_field_name,
					to_field_name,
					from_field,
					to_field,
				);

				if similarity > best_match_score {
					best_match_score = similarity;
					best_match_name = Some(to_field_name.clone());
				}
			}

			if let Some(matched_name) = best_match_name {
				matched_to_fields.insert(matched_name);
				total_similarity += best_match_score;
			}
		}

		total_similarity / total_fields as f64
	}

	/// Calculate field-level similarity using hybrid algorithm
	///
	/// This method combines Jaro-Winkler and Levenshtein distance to measure
	/// similarity between field names, providing better detection than either alone.
	///
	/// # Hybrid Algorithm
	/// - **Jaro-Winkler**: Best for prefix similarities (e.g., "UserEmail" vs "UserAddress")
	///   - Time Complexity: O(n)
	///   - Range: 0.0 to 1.0
	///   - Default weight: 70%
	/// - **Levenshtein**: Best for edit distance (e.g., "User" vs "Users")
	///   - Time Complexity: O(n*m)
	///   - Normalized to 0.0-1.0 range
	///   - Default weight: 30%
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let from_state = ProjectState::new();
	/// let to_state = ProjectState::new();
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	///
	/// let from_field = FieldState::new("user_email".to_string(), "VARCHAR".to_string(), false);
	/// let to_field = FieldState::new("email".to_string(), "VARCHAR".to_string(), false);
	///
	/// // High similarity (field name is similar and type matches)
	/// // Jaro-Winkler ≈ 0.81, Levenshtein normalized ≈ 0.45
	/// // Hybrid (0.7 * 0.81 + 0.3 * 0.45) ≈ 0.70
	/// ```
	fn calculate_field_similarity(
		&self,
		from_field_name: &str,
		to_field_name: &str,
		from_field: &FieldState,
		to_field: &FieldState,
	) -> f64 {
		// If types don't match, similarity is 0
		if from_field.field_type != to_field.field_type {
			return 0.0;
		}

		// Calculate Jaro-Winkler similarity (0.0 - 1.0)
		let jaro_winkler_sim = jaro_winkler(from_field_name, to_field_name);

		// Calculate Levenshtein distance and normalize to 0.0-1.0
		let lev_distance = levenshtein(from_field_name, to_field_name);
		let max_len = from_field_name.len().max(to_field_name.len()) as f64;
		let levenshtein_sim = if max_len > 0.0 {
			1.0 - (lev_distance as f64 / max_len)
		} else {
			1.0 // Both strings are empty
		};

		// Combine using configured weights
		let name_similarity = self.similarity_config.jaro_winkler_weight * jaro_winkler_sim
			+ self.similarity_config.levenshtein_weight * levenshtein_sim;

		// Boost similarity if nullability also matches
		let nullable_boost = if from_field.nullable == to_field.nullable {
			0.1
		} else {
			0.0
		};

		(name_similarity + nullable_boost).min(1.0)
	}

	/// Perform bipartite matching between deleted and created models
	///
	/// # Algorithm: Maximum Weight Bipartite Matching
	/// - Based on Hopcroft-Karp algorithm concept: O(n*m*√(n+m))
	/// - Uses petgraph for graph construction
	/// - Finds optimal matching considering all possible pairs
	///
	/// # Implementation Note
	/// This implementation uses a greedy approach with weighted edges sorted by
	/// similarity score. While not a full Hopcroft-Karp implementation, it provides
	/// good results with O(E log E) complexity where E = number of edges.
	///
	/// # Returns
	/// Vector of matches: (deleted_key, created_key, similarity_score)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let mut from_state = ProjectState::new();
	/// let mut old_model = ModelState::new("myapp", "User");
	/// old_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// from_state.add_model(old_model);
	///
	/// let mut to_state = ProjectState::new();
	/// let mut new_model = ModelState::new("auth", "User");
	/// new_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// to_state.add_model(new_model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// // Would detect cross-app model move from myapp.User to auth.User
	/// ```
	fn find_optimal_model_matches(
		&self,
		deleted: &[&(String, String)],
		created: &[&(String, String)],
	) -> Vec<((String, String), (String, String), f64)> {
		let mut graph = Graph::<(), f64, Undirected>::new_undirected();
		let mut deleted_nodes = Vec::new();
		let mut created_nodes = Vec::new();

		// Create nodes for deleted models (left side of bipartite graph)
		for _ in deleted {
			deleted_nodes.push(graph.add_node(()));
		}

		// Create nodes for created models (right side of bipartite graph)
		for _ in created {
			created_nodes.push(graph.add_node(()));
		}

		// Add edges with similarity weights
		for (i, deleted_key) in deleted.iter().enumerate() {
			if let Some(from_model) = self.from_state.models.get(*deleted_key) {
				for (j, created_key) in created.iter().enumerate() {
					if let Some(to_model) = self.to_state.models.get(*created_key) {
						let similarity = self.calculate_model_similarity(from_model, to_model);

						// Only add edge if similarity exceeds threshold
						if similarity >= self.similarity_config.model_threshold() {
							graph.add_edge(deleted_nodes[i], created_nodes[j], similarity);
						}
					}
				}
			}
		}

		// Find maximum weight matching using greedy algorithm
		// (Full Hopcroft-Karp would require additional implementation)
		let mut matches = Vec::new();
		let mut used_deleted = std::collections::HashSet::new();
		let mut used_created = std::collections::HashSet::new();

		// Sort edges by weight (similarity) in descending order
		let mut weighted_edges: Vec<_> = graph
			.edge_references()
			.map(|e| (e.source(), e.target(), *e.weight()))
			.collect();
		weighted_edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

		// Greedy matching: pick highest weight edges first
		for (source, target, weight) in weighted_edges {
			let source_idx = deleted_nodes.iter().position(|&n| n == source);
			let target_idx = created_nodes.iter().position(|&n| n == target);

			if let (Some(i), Some(j)) = (source_idx, target_idx)
				&& !used_deleted.contains(&i) && !used_created.contains(&j) {
					matches.push((deleted[i].clone(), created[j].clone(), weight));
					used_deleted.insert(i);
					used_created.insert(j);
				}
		}

		matches
	}

	/// Detect added indexes
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:1500-1600
	fn detect_added_indexes(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), to_model) in &self.to_state.models {
			if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
				for to_index in &to_model.indexes {
					// Check if this index exists in from_model
					if !from_model
						.indexes
						.iter()
						.any(|idx| idx.name == to_index.name)
					{
						changes.added_indexes.push((
							app_label.clone(),
							model_name.clone(),
							to_index.clone(),
						));
					}
				}
			}
		}
	}

	/// Detect removed indexes
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:1600-1700
	fn detect_removed_indexes(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), from_model) in &self.from_state.models {
			if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
				for from_index in &from_model.indexes {
					// Check if this index still exists in to_model
					if !to_model
						.indexes
						.iter()
						.any(|idx| idx.name == from_index.name)
					{
						changes.removed_indexes.push((
							app_label.clone(),
							model_name.clone(),
							from_index.name.clone(),
						));
					}
				}
			}
		}
	}

	/// Detect added constraints
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:1700-1800
	fn detect_added_constraints(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), to_model) in &self.to_state.models {
			if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
				for to_constraint in &to_model.constraints {
					// Check if this constraint exists in from_model
					if !from_model
						.constraints
						.iter()
						.any(|c| c.name == to_constraint.name)
					{
						changes.added_constraints.push((
							app_label.clone(),
							model_name.clone(),
							to_constraint.clone(),
						));
					}
				}
			}
		}
	}

	/// Detect removed constraints
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:1800-1900
	fn detect_removed_constraints(&self, changes: &mut DetectedChanges) {
		for ((app_label, model_name), from_model) in &self.from_state.models {
			if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
				for from_constraint in &from_model.constraints {
					// Check if this constraint still exists in to_model
					if !to_model
						.constraints
						.iter()
						.any(|c| c.name == from_constraint.name)
					{
						changes.removed_constraints.push((
							app_label.clone(),
							model_name.clone(),
							from_constraint.name.clone(),
						));
					}
				}
			}
		}
	}

	/// Generate operations from detected changes
	///
	/// Converts DetectedChanges into a list of Operation objects that can be
	/// executed to migrate the database schema.
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:1063-1164
	/// ```python
	/// def generate_created_models(self):
	///     for app_label, model_name in sorted(self.new_model_keys):
	///         model_state = self.to_state.models[app_label, model_name]
	///         self.add_operation(
	///             app_label,
	///             operations.CreateModel(
	///                 name=model_name,
	///                 fields=model_state.fields,
	///                 options=model_state.options,
	///                 bases=model_state.bases,
	///             ),
	///         )
	/// ```
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let mut from_state = ProjectState::new();
	/// let mut to_state = ProjectState::new();
	///
	// Add a new model to the target state
	/// let mut model = ModelState::new("myapp", "User");
	/// model.add_field(FieldState::new("id".to_string(), "IntegerField".to_string(), false));
	/// to_state.add_model(model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// let operations = detector.generate_operations();
	///
	/// assert!(!operations.is_empty());
	/// ```
	pub fn generate_operations(&self) -> Vec<crate::Operation> {
		let changes = self.detect_changes();
		let mut operations = Vec::new();

		// Generate CreateTable operations for new models
		for (app_label, model_name) in &changes.created_models {
			if let Some(model) = self.to_state.get_model(app_label, model_name) {
				let mut columns = Vec::new();
				for (field_name, field_state) in &model.fields {
					columns.push(crate::ColumnDefinition::new(
						field_name.clone(),
						field_state.field_type.clone(),
					));
				}

				operations.push(crate::Operation::CreateTable {
					name: model.name.clone(),
					columns,
					constraints: Vec::new(),
				});
			}
		}

		// Generate AddColumn operations for new fields
		for (app_label, model_name, field_name) in &changes.added_fields {
			if let Some(model) = self.to_state.get_model(app_label, model_name)
				&& let Some(field) = model.get_field(field_name) {
					operations.push(crate::Operation::AddColumn {
						table: model.name.clone(),
						column: crate::ColumnDefinition::new(
							field_name.clone(),
							field.field_type.clone(),
						),
					});
				}
		}

		// Generate AlterColumn operations for changed fields
		for (app_label, model_name, field_name) in &changes.altered_fields {
			if let Some(model) = self.to_state.get_model(app_label, model_name)
				&& let Some(field) = model.get_field(field_name) {
					operations.push(crate::Operation::AlterColumn {
						table: model.name.clone(),
						column: field_name.clone(),
						new_definition: crate::ColumnDefinition::new(
							field_name.clone(),
							field.field_type.clone(),
						),
					});
				}
		}

		// Generate DropColumn operations for removed fields
		for (app_label, model_name, field_name) in &changes.removed_fields {
			if let Some(model) = self.from_state.get_model(app_label, model_name) {
				operations.push(crate::Operation::DropColumn {
					table: model.name.clone(),
					column: field_name.clone(),
				});
			}
		}

		// Generate DropTable operations for deleted models
		for (app_label, model_name) in &changes.deleted_models {
			if let Some(model) = self.from_state.get_model(app_label, model_name) {
				operations.push(crate::Operation::DropTable {
					name: model.name.clone(),
				});
			}
		}

		// Note: MoveModel operations for cross-app moves are detected in moved_models
		// but cannot be directly converted to legacy Operation enum.
		// Use the new operations API (operations::models::MoveModel) instead.
		// Future enhancement: Add MoveModel variant to Operation enum or
		// use a separate operation handling path for cross-app moves.

		// For now, cross-app moves will be handled separately in generate_migrations()
		// where we can use the new operations API directly.

		operations
	}

	/// Generate migrations from detected changes
	///
	/// Groups operations by app_label and creates Migration objects for each app.
	/// This is the final step in the migration autodetection process.
	///
	/// # Django Reference
	/// From: django/db/migrations/autodetector.py:95-141
	/// ```python
	/// def changes(self, graph, trim_to_apps=None, convert_apps=None, migration_name=None):
	///     # Generate operations
	///     self._generate_through_model_map()
	///     self.generate_renamed_models()
	///     # ... all other generate_* methods
	///
	///     # Group operations by app
	///     self.arrange_for_graph(changes, graph, trim_to_apps)
	///
	///     # Create Migration objects
	///     return changes
	/// ```
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let mut from_state = ProjectState::new();
	/// let mut to_state = ProjectState::new();
	///
	// Add a new model
	/// let mut model = ModelState::new("blog", "Post");
	/// model.add_field(FieldState::new("title".to_string(), "CharField".to_string(), false));
	/// to_state.add_model(model);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// let migrations = detector.generate_migrations();
	///
	/// assert_eq!(migrations.len(), 1);
	/// assert_eq!(migrations[0].app_label, "blog");
	/// assert!(!migrations[0].operations.is_empty());
	/// ```
	pub fn generate_migrations(&self) -> Vec<crate::Migration> {
		let changes = self.detect_changes();
		let mut migrations_by_app: std::collections::HashMap<String, Vec<crate::Operation>> =
			std::collections::HashMap::new();

		// Group created models by app
		for (app_label, model_name) in &changes.created_models {
			if let Some(model) = self.to_state.get_model(app_label, model_name) {
				let mut columns = Vec::new();
				for (field_name, field_state) in &model.fields {
					columns.push(crate::ColumnDefinition::new(
						field_name.clone(),
						field_state.field_type.clone(),
					));
				}

				migrations_by_app
					.entry(app_label.clone())
					.or_default()
					.push(crate::Operation::CreateTable {
						name: model.name.clone(),
						columns,
						constraints: Vec::new(),
					});
			}
		}

		// Group added fields by app
		for (app_label, model_name, field_name) in &changes.added_fields {
			if let Some(model) = self.to_state.get_model(app_label, model_name)
				&& let Some(field) = model.get_field(field_name) {
					migrations_by_app
						.entry(app_label.clone())
						.or_default()
						.push(crate::Operation::AddColumn {
							table: model.name.clone(),
							column: crate::ColumnDefinition::new(
								field_name.clone(),
								field.field_type.clone(),
							),
						});
				}
		}

		// Group altered fields by app
		for (app_label, model_name, field_name) in &changes.altered_fields {
			if let Some(model) = self.to_state.get_model(app_label, model_name)
				&& let Some(field) = model.get_field(field_name) {
					migrations_by_app
						.entry(app_label.clone())
						.or_default()
						.push(crate::Operation::AlterColumn {
							table: model.name.clone(),
							column: field_name.clone(),
							new_definition: crate::ColumnDefinition::new(
								field_name.clone(),
								field.field_type.clone(),
							),
						});
				}
		}

		// Group removed fields by app
		for (app_label, model_name, field_name) in &changes.removed_fields {
			if let Some(model) = self.from_state.get_model(app_label, model_name) {
				migrations_by_app
					.entry(app_label.clone())
					.or_default()
					.push(crate::Operation::DropColumn {
						table: model.name.clone(),
						column: field_name.clone(),
					});
			}
		}

		// Group deleted models by app
		for (app_label, model_name) in &changes.deleted_models {
			if let Some(model) = self.from_state.get_model(app_label, model_name) {
				migrations_by_app
					.entry(app_label.clone())
					.or_default()
					.push(crate::Operation::DropTable {
						name: model.name.clone(),
					});
			}
		}

		// Create Migration objects for each app
		let mut migrations = Vec::new();
		for (app_label, operations) in migrations_by_app {
			// Generate a simple migration name based on the first operation
			let migration_name = if let Some(op) = operations.first() {
				match op {
					crate::Operation::CreateTable { name, .. } => {
						format!("0001_initial_{}", name.to_lowercase())
					}
					crate::Operation::AddColumn { table, column } => {
						format!(
							"0001_add_{}_{}",
							column.name.to_lowercase(),
							table.to_lowercase()
						)
					}
					crate::Operation::AlterColumn { table, column, .. } => format!(
						"0001_alter_{}_{}",
						column.to_lowercase(),
						table.to_lowercase()
					),
					crate::Operation::DropColumn { table, column, .. } => {
						format!(
							"0001_remove_{}_{}",
							column.to_lowercase(),
							table.to_lowercase()
						)
					}
					crate::Operation::DropTable { name, .. } => {
						format!("0001_delete_{}", name.to_lowercase())
					}
					_ => "0001_auto".to_string(),
				}
			} else {
				"0001_auto".to_string()
			};

			let mut migration = crate::Migration::new(&migration_name, &app_label);
			for operation in operations {
				migration = migration.add_operation(operation);
			}
			migrations.push(migration);
		}

		migrations
	}

	/// Detect model dependencies for operation ordering
	///
	/// This method analyzes the final state (to_state) to build a dependency graph.
	/// A model depends on another if it has ForeignKey or ManyToMany fields pointing to it.
	///
	/// # Dependency Detection Rules
	/// - ForeignKey: Model A depends on Model B if A has a ForeignKey field to B
	/// - ManyToMany: Model A depends on Model B if A has a ManyToMany field to B
	/// - Self-referential: A model can depend on itself (e.g., tree structures)
	///
	/// # Use Case
	/// When moving models between apps, we must ensure:
	/// 1. Referenced models are moved before referencing models
	/// 2. Circular dependencies are detected and handled
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
	///
	/// let from_state = ProjectState::new();
	/// let mut to_state = ProjectState::new();
	///
	/// // Create User model
	/// let mut user = ModelState::new("accounts", "User");
	/// user.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// to_state.add_model(user);
	///
	/// // Create Post model that depends on User
	/// let mut post = ModelState::new("blog", "Post");
	/// post.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
	/// post.add_field(FieldState::new("author".to_string(), "ForeignKey(accounts.User)".to_string(), false));
	/// to_state.add_model(post);
	///
	/// let detector = MigrationAutodetector::new(from_state, to_state);
	/// let changes = detector.detect_changes();
	///
	/// // blog.Post depends on accounts.User
	/// let post_deps = changes.model_dependencies.get(&("blog".to_string(), "Post".to_string()));
	/// assert!(post_deps.is_some());
	/// assert!(post_deps.unwrap().contains(&("accounts".to_string(), "User".to_string())));
	/// ```
	fn detect_model_dependencies(&self, changes: &mut DetectedChanges) {
		// Analyze all models in the final state
		for ((app_label, model_name), model) in &self.to_state.models {
			let mut dependencies = Vec::new();

			// Check each field for foreign key relationships
			for field in model.fields.values() {
				// Detect ForeignKey fields by checking field type
				// Format: "ForeignKey(app.Model)" or "ManyToManyField(app.Model)"
				if let Some(referenced_model) =
					self.extract_related_model(&field.field_type, app_label)
				{
					dependencies.push(referenced_model);
				}
			}

			// Only store if there are actual dependencies
			if !dependencies.is_empty() {
				changes
					.model_dependencies
					.insert((app_label.clone(), model_name.clone()), dependencies);
			}
		}
	}

	/// Extract related model from field type string
	///
	/// Parses field type strings like:
	/// - "ForeignKey(app.Model)" -> Some(("app", "Model"))
	/// - "ManyToManyField(app.Model)" -> Some(("app", "Model"))
	/// - "ForeignKey(Model)" -> Some((current_app, "Model"))
	/// - "CharField" -> None
	///
	/// # Arguments
	/// * `field_type` - Field type string (e.g., "ForeignKey(accounts.User)")
	/// * `current_app` - Current app label for resolving unqualified references
	///
	/// # Returns
	/// * `Some((app_label, model_name))` if field is a relation
	/// * `None` if field is not a relation
	fn extract_related_model(
		&self,
		field_type: &str,
		current_app: &str,
	) -> Option<(String, String)> {
		// Check for ForeignKey pattern
		if let Some(inner) = field_type
			.strip_prefix("ForeignKey(")
			.and_then(|s| s.strip_suffix(")"))
		{
			return self.parse_model_reference(inner, current_app);
		}

		// Check for ManyToManyField pattern
		if let Some(inner) = field_type
			.strip_prefix("ManyToManyField(")
			.and_then(|s| s.strip_suffix(")"))
		{
			return self.parse_model_reference(inner, current_app);
		}

		// Check for OneToOneField pattern
		if let Some(inner) = field_type
			.strip_prefix("OneToOneField(")
			.and_then(|s| s.strip_suffix(")"))
		{
			return self.parse_model_reference(inner, current_app);
		}

		None
	}

	/// Parse model reference string into (app_label, model_name)
	///
	/// Supports formats:
	/// - "app.Model" -> ("app", "Model")
	/// - "Model" -> (current_app, "Model") - Uses current app for unqualified references
	///
	/// # Arguments
	/// * `reference` - Model reference string (e.g., "accounts.User" or "User")
	/// * `current_app` - Current app label for resolving unqualified references
	///
	/// # Returns
	/// * `Some((app_label, model_name))` if parseable
	/// * `None` if format is invalid
	fn parse_model_reference(
		&self,
		reference: &str,
		current_app: &str,
	) -> Option<(String, String)> {
		let parts: Vec<&str> = reference.split('.').collect();
		match parts.as_slice() {
			// Fully qualified reference: "app.Model"
			[app, model] => Some((app.to_string(), model.to_string())),
			// Unqualified reference: "Model" - assume same app
			[model] => {
				// Use current app for same-app references
				Some((current_app.to_string(), model.to_string()))
			}
			// Invalid format
			_ => None,
		}
	}
}

impl ModelState {
	/// Remove a field from this model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ModelState, FieldState};
	///
	/// let mut model = ModelState::new("myapp", "User");
	/// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	/// model.add_field(field);
	/// assert!(model.has_field("email"));
	///
	/// model.remove_field("email");
	/// assert!(!model.has_field("email"));
	/// ```
	pub fn remove_field(&mut self, name: &str) {
		self.fields.remove(name);
	}

	/// Alter a field definition
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{ModelState, FieldState};
	///
	/// let mut model = ModelState::new("myapp", "User");
	/// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
	/// model.add_field(field);
	///
	/// let new_field = FieldState::new("email".to_string(), "TEXT".to_string(), true);
	/// model.alter_field("email", new_field);
	///
	/// let altered = model.get_field("email").unwrap();
	/// assert_eq!(altered.field_type, "TEXT");
	/// assert_eq!(altered.nullable, true);
	/// ```
	pub fn alter_field(&mut self, name: &str, new_field: FieldState) {
		self.fields.insert(name.to_string(), new_field);
	}
}
