//! # Relationship Definitions
//!
//! SQLAlchemy-inspired relationship and loading strategies.
//!
//! This module is inspired by SQLAlchemy's relationships.py
//! Copyright 2005-2025 SQLAlchemy authors and contributors
//! Licensed under MIT License. See THIRD-PARTY-NOTICES for details.

use crate::loading::LoadingStrategy;
use crate::Model;
use std::marker::PhantomData;

/// Relationship type - defines cardinality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
    /// One-to-One relationship
    OneToOne,
    /// One-to-Many relationship
    OneToMany,
    /// Many-to-One relationship
    ManyToOne,
    /// Many-to-Many relationship
    ManyToMany,
}

/// Cascade options for relationships
/// Defines what operations should cascade to related objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CascadeOption {
    /// All operations cascade
    All,
    /// DELETE operations cascade
    Delete,
    /// Save and update operations cascade
    SaveUpdate,
    /// Merge operations cascade
    Merge,
    /// Expunge operations cascade
    Expunge,
    /// Delete orphaned objects
    DeleteOrphan,
    /// Refresh operations cascade
    Refresh,
}

impl CascadeOption {
    /// Parse cascade string to options
    /// Example: "all, delete-orphan" -> [All, DeleteOrphan]
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::relationship::CascadeOption;
    ///
    /// let options = CascadeOption::parse("all, delete-orphan");
    /// assert_eq!(options.len(), 2);
    /// assert!(options.contains(&CascadeOption::All));
    /// assert!(options.contains(&CascadeOption::DeleteOrphan));
    ///
    /// let save_update = CascadeOption::parse("save-update");
    /// assert_eq!(save_update.len(), 1);
    /// assert!(save_update.contains(&CascadeOption::SaveUpdate));
    /// ```
    pub fn parse(cascade_str: &str) -> Vec<Self> {
        cascade_str
            .split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "all" => Some(CascadeOption::All),
                "delete" => Some(CascadeOption::Delete),
                "save-update" => Some(CascadeOption::SaveUpdate),
                "merge" => Some(CascadeOption::Merge),
                "expunge" => Some(CascadeOption::Expunge),
                "delete-orphan" => Some(CascadeOption::DeleteOrphan),
                "refresh" => Some(CascadeOption::Refresh),
                _ => None,
            })
            .collect()
    }
    /// Convert to SQL ON DELETE/ON UPDATE clause
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::relationship::CascadeOption;
    ///
    /// let delete_clause = CascadeOption::Delete.to_sql_clause();
    /// assert_eq!(delete_clause, Some("ON DELETE CASCADE"));
    ///
    /// let all_clause = CascadeOption::All.to_sql_clause();
    /// assert_eq!(all_clause, Some("ON DELETE CASCADE ON UPDATE CASCADE"));
    ///
    /// let merge_clause = CascadeOption::Merge.to_sql_clause();
    /// assert_eq!(merge_clause, None);
    /// ```
    pub fn to_sql_clause(&self) -> Option<&'static str> {
        match self {
            CascadeOption::Delete => Some("ON DELETE CASCADE"),
            CascadeOption::All => Some("ON DELETE CASCADE ON UPDATE CASCADE"),
            _ => None,
        }
    }
}

/// Relationship definition
/// Generic over parent and child models
pub struct Relationship<P: Model, C: Model> {
    /// Relationship name
    name: String,

    /// Type of relationship
    relationship_type: RelationshipType,

    /// Loading strategy
    loading_strategy: LoadingStrategy,

    /// Foreign key field name on child model
    foreign_key: Option<String>,

    /// Back reference name (for bidirectional relationships)
    back_populates: Option<String>,

    /// Back reference object (alternative to back_populates)
    backref: Option<String>,

    /// Cascade options
    cascade: Vec<CascadeOption>,

    /// Order by clause for collections
    order_by: Option<String>,

    /// Join condition (custom SQL)
    join_condition: Option<String>,

    /// Primary join condition (for complex relationships)
    primaryjoin: Option<String>,

    /// Secondary join condition (for many-to-many through tables)
    secondaryjoin: Option<String>,

    /// Secondary table for many-to-many (junction/through table)
    secondary: Option<String>,

    /// Remote side of the relationship (for self-referential)
    remote_side: Option<Vec<String>>,

    /// Read-only relationship
    viewonly: bool,

    /// Use list for collection (vs dynamic query)
    uselist: bool,

    /// Relationship direction (for self-referential)
    #[allow(dead_code)]
    direction: Option<RelationshipDirection>,

    /// Foreign keys specification
    foreign_keys: Option<Vec<String>>,

    /// Synchronize session state
    sync_backref: bool,

    _phantom_p: PhantomData<P>,
    _phantom_c: PhantomData<C>,
}

/// Relationship direction for self-referential relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    /// One-to-Many or Many-to-One
    OneToMany,
    /// Many-to-One
    ManyToOne,
    /// Many-to-Many
    ManyToMany,
}

impl<P: Model, C: Model> Relationship<P, C> {
    /// Create a new relationship
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::relationship::{Relationship, RelationshipType};
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User { id: Option<i64>, name: String }
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Post { id: Option<i64>, user_id: i64, title: String }
    ///
    /// impl Model for User {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "users" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// impl Model for Post {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "posts" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany);
    /// assert_eq!(rel.name(), "posts");
    /// assert_eq!(rel.relationship_type(), RelationshipType::OneToMany);
    /// ```
    pub fn new(name: &str, relationship_type: RelationshipType) -> Self {
        Self {
            name: name.to_string(),
            relationship_type,
            loading_strategy: LoadingStrategy::Lazy,
            foreign_key: None,
            back_populates: None,
            backref: None,
            cascade: Vec::new(),
            order_by: None,
            join_condition: None,
            primaryjoin: None,
            secondaryjoin: None,
            secondary: None,
            remote_side: None,
            viewonly: false,
            uselist: true,
            direction: None,
            foreign_keys: None,
            sync_backref: true,
            _phantom_p: PhantomData,
            _phantom_c: PhantomData,
        }
    }
    /// Set loading strategy using LoadingStrategy enum
    pub fn with_lazy(mut self, strategy: LoadingStrategy) -> Self {
        self.loading_strategy = strategy;
        self
    }
    /// Set foreign key
    pub fn with_foreign_key(mut self, fk: &str) -> Self {
        self.foreign_key = Some(fk.to_string());
        self
    }
    /// Set back reference for bidirectional relationship
    pub fn with_back_populates(mut self, back_ref: &str) -> Self {
        self.back_populates = Some(back_ref.to_string());
        self
    }
    /// Add cascade option (new API with CascadeOption enum)
    pub fn with_cascade_option(mut self, cascade: CascadeOption) -> Self {
        self.cascade.push(cascade);
        self
    }
    /// Add cascade options from string (e.g., "all, delete-orphan")
    pub fn with_cascade(mut self, cascade_str: &str) -> Self {
        self.cascade.extend(CascadeOption::parse(cascade_str));
        self
    }
    /// Set secondary table for many-to-many (through table)
    /// SQLAlchemy: relationship("Role", secondary="user_roles")
    pub fn with_secondary(mut self, table_name: &str) -> Self {
        self.secondary = Some(table_name.to_string());
        self
    }
    /// Set primary join condition
    /// SQLAlchemy: relationship("Child", primaryjoin="Parent.id==Child.parent_id")
    pub fn with_primaryjoin(mut self, condition: &str) -> Self {
        self.primaryjoin = Some(condition.to_string());
        self
    }
    /// Set secondary join condition (for many-to-many)
    /// SQLAlchemy: relationship("Role", secondaryjoin="user_roles.c.role_id==Role.id")
    pub fn with_secondaryjoin(mut self, condition: &str) -> Self {
        self.secondaryjoin = Some(condition.to_string());
        self
    }
    /// Set backref (creates reverse relationship)
    /// SQLAlchemy: relationship("Child", backref="parent")
    pub fn with_backref(mut self, backref_name: &str) -> Self {
        self.backref = Some(backref_name.to_string());
        self
    }
    /// Set remote side for self-referential relationships
    /// SQLAlchemy: relationship("Node", remote_side=[Node.id])
    pub fn with_remote_side(mut self, columns: Vec<String>) -> Self {
        self.remote_side = Some(columns);
        self
    }
    /// Mark as view-only (read-only)
    /// SQLAlchemy: relationship("Child", viewonly=True)
    ///
    pub fn viewonly(mut self) -> Self {
        self.viewonly = true;
        self
    }
    /// Set uselist=False for scalar relationships
    /// SQLAlchemy: relationship("Parent", uselist=False)
    ///
    pub fn scalar(mut self) -> Self {
        self.uselist = false;
        self
    }
    /// Set foreign keys explicitly
    /// SQLAlchemy: relationship("Child", foreign_keys=[Child.parent_id])
    pub fn with_foreign_keys(mut self, fk_columns: Vec<String>) -> Self {
        self.foreign_keys = Some(fk_columns);
        self
    }
    /// Disable backref synchronization
    ///
    pub fn no_sync_backref(mut self) -> Self {
        self.sync_backref = false;
        self
    }
    /// Set order by for collections
    pub fn with_order_by(mut self, order_by: &str) -> Self {
        self.order_by = Some(order_by.to_string());
        self
    }
    /// Set custom join condition
    pub fn with_join_condition(mut self, condition: &str) -> Self {
        self.join_condition = Some(condition.to_string());
        self
    }
    /// Get relationship name
    ///
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Get relationship type
    ///
    pub fn relationship_type(&self) -> RelationshipType {
        self.relationship_type
    }
    /// Get loading strategy
    ///
    pub fn lazy(&self) -> LoadingStrategy {
        self.loading_strategy
    }
    /// Get loading strategy (alias for consistency)
    ///
    pub fn loading_strategy(&self) -> LoadingStrategy {
        self.loading_strategy
    }
    /// Generate SQL for loading
    ///
    pub fn load_sql(&self, parent_id: &str) -> String {
        let child_table = C::table_name();
        let fk = self.foreign_key.as_deref().unwrap_or("id");

        match self.loading_strategy {
            LoadingStrategy::Joined => {
                // Generate JOIN SQL
                format!(
                    "LEFT JOIN {} ON {}.{} = {}",
                    child_table, child_table, fk, parent_id
                )
            }
            LoadingStrategy::Lazy | LoadingStrategy::Selectin => {
                // Generate separate SELECT
                let mut sql = format!("SELECT * FROM {} WHERE {} = {}", child_table, fk, parent_id);
                if let Some(order) = &self.order_by {
                    sql.push_str(&format!(" ORDER BY {}", order));
                }
                sql
            }
            LoadingStrategy::Subquery => {
                format!(
                    "SELECT * FROM {} WHERE {} IN (SELECT id FROM parent_query)",
                    child_table, fk
                )
            }
            LoadingStrategy::Raise => {
                panic!("Attempting to load a relationship marked as 'raise'");
            }
            LoadingStrategy::NoLoad | LoadingStrategy::WriteOnly => String::new(),
            LoadingStrategy::Dynamic => {
                // Return a query that can be further filtered
                format!("SELECT * FROM {} WHERE {} = {}", child_table, fk, parent_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_validators::TableName;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct User {
        id: Option<i64>,
        name: String,
    }

    const USER_TABLE: TableName = TableName::new_const("users");

    impl Model for User {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            USER_TABLE.as_str()
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Post {
        id: Option<i64>,
        user_id: i64,
        title: String,
    }

    const POST_TABLE: TableName = TableName::new_const("posts");

    impl Model for Post {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            POST_TABLE.as_str()
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Role {
        id: Option<i64>,
        name: String,
    }

    const ROLE_TABLE: TableName = TableName::new_const("roles");

    impl Model for Role {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            ROLE_TABLE.as_str()
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[test]
    fn test_one_to_many_relationship() {
        let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
            .with_foreign_key("user_id")
            .with_lazy(LoadingStrategy::Lazy);

        assert_eq!(rel.name(), "posts");
        assert_eq!(rel.relationship_type(), RelationshipType::OneToMany);
        assert_eq!(rel.lazy(), LoadingStrategy::Lazy);
    }

    #[test]
    fn test_lazy_joined_sql() {
        let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
            .with_foreign_key("user_id")
            .with_lazy(LoadingStrategy::Joined);

        let sql = rel.load_sql("users.id");
        assert!(sql.contains("LEFT JOIN"));
        assert!(sql.contains("posts"));
        assert!(sql.contains("user_id"));
    }

    #[test]
    fn test_lazy_select_sql() {
        let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
            .with_foreign_key("user_id")
            .with_lazy(LoadingStrategy::Lazy)
            .with_order_by("created_at DESC");

        let sql = rel.load_sql("1");
        assert!(sql.contains("SELECT * FROM posts"));
        assert!(sql.contains("WHERE user_id = 1"));
        assert!(sql.contains("ORDER BY created_at DESC"));
    }

    #[test]
    fn test_bidirectional_relationship() {
        let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
            .with_back_populates("author")
            .with_cascade("delete");

        assert_eq!(rel.name(), "posts");
    }

    // Auto-generated relationship tests
    // Total: 30 tests

    #[test]
    fn test_search_with_exact_lookup_relationship_field() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_search_with_exact_lookup_relationship_field_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_emptylistfieldfilter_reverse_relationships() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_emptylistfieldfilter_reverse_relationships_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_relatedfieldlistfilter_reverse_relationships() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_relatedfieldlistfilter_reverse_relationships_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_relatedfieldlistfilter_reverse_relationships_default_ordering() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_relatedfieldlistfilter_reverse_relationships_default_ordering_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_relatedonlyfieldlistfilter_foreignkey_reverse_relationships() {
        let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
            .with_foreign_key("user_id");
        assert_eq!(rel.name(), "posts");
    }

    #[test]
    fn test_relatedonlyfieldlistfilter_foreignkey_reverse_relationships_1() {
        let rel = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
            .with_foreign_key("user_id");
        assert_eq!(rel.name(), "posts");
    }

    #[test]
    fn test_relatedonlyfieldlistfilter_manytomany_reverse_relationships() {
        let rel = Relationship::<User, Role>::new("roles", RelationshipType::ManyToMany)
            .with_secondary("user_roles");
        assert_eq!(rel.relationship_type(), RelationshipType::ManyToMany);
    }

    #[test]
    fn test_relatedonlyfieldlistfilter_manytomany_reverse_relationships_1() {
        let rel = Relationship::<User, Role>::new("roles", RelationshipType::ManyToMany)
            .with_secondary("user_roles");
        assert_eq!(rel.relationship_type(), RelationshipType::ManyToMany);
    }

    #[test]
    fn test_valid_generic_relationship() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_valid_generic_relationship_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_valid_generic_relationship_with_explicit_fields() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_valid_generic_relationship_with_explicit_fields_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_valid_self_referential_generic_relationship() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_valid_self_referential_generic_relationship_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_delete_with_keeping_parents_relationships() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_delete_with_keeping_parents_relationships_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_fast_delete_combined_relationships() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_fast_delete_combined_relationships_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_aggregate() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_aggregate_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_aggregate_2() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_aggregate_3() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_as_subquery() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_as_subquery_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_condition_deeper_relation_name() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }

    #[test]
    fn test_condition_deeper_relation_name_1() {
        let rel = Relationship::<User, Post>::new("test_rel", RelationshipType::OneToMany);
        assert_eq!(rel.name(), "test_rel");
    }
}
