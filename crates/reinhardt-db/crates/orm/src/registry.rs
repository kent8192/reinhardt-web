use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

/// Table metadata information
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub primary_key: Vec<String>,
}

impl TableInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            primary_key: Vec::new(),
        }
    }

    pub fn schema(self, _schema: &str) -> Self {
        // Schema information would be stored here
        self
    }

    pub fn add_column(mut self, column: ColumnInfo) -> Self {
        self.columns.push(column);
        self
    }
}

/// Column metadata information
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
    pub default: Option<String>,
}

impl ColumnInfo {
    pub fn new(name: impl Into<String>, column_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            column_type: column_type.into(),
            nullable: true,
            default: None,
        }
    }
}

/// Entity mapper (alias for EntityMapper)
pub type Mapper = EntityMapper;

/// Global registry instance
pub fn registry() -> &'static MapperRegistry {
    use once_cell::sync::Lazy;
    static REGISTRY: Lazy<MapperRegistry> = Lazy::new(MapperRegistry::new);
    &REGISTRY
}

/// Mapper registry for managing entity mappings
#[derive(Debug, Clone)]
pub struct MapperRegistry {
    mappers: Arc<RwLock<HashMap<String, EntityMapper>>>,
}

impl MapperRegistry {
    pub fn new() -> Self {
        Self {
            mappers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, name: impl Into<String>, mapper: EntityMapper) {
        let name = name.into();
        if let Ok(mut mappers) = self.mappers.write() {
            mappers.insert(name, mapper);
        }
    }

    pub fn get(&self, name: &str) -> Option<EntityMapper> {
        if let Ok(mappers) = self.mappers.read() {
            mappers.get(name).cloned()
        } else {
            None
        }
    }

    pub fn remove(&self, name: &str) -> bool {
        if let Ok(mut mappers) = self.mappers.write() {
            mappers.remove(name).is_some()
        } else {
            false
        }
    }

    pub fn clear(&self) {
        if let Ok(mut mappers) = self.mappers.write() {
            mappers.clear();
        }
    }

    pub fn count(&self) -> usize {
        if let Ok(mappers) = self.mappers.read() {
            mappers.len()
        } else {
            0
        }
    }
}

impl Default for MapperRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity mapper
#[derive(Debug, Clone)]
pub struct EntityMapper {
    pub table_name: String,
    pub columns: Vec<ColumnMapping>,
    pub primary_key: Vec<String>,
}

impl EntityMapper {
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            columns: Vec::new(),
            primary_key: Vec::new(),
        }
    }

    pub fn add_column(&mut self, mapping: ColumnMapping) {
        self.columns.push(mapping);
    }

    pub fn set_primary_key(&mut self, columns: Vec<String>) {
        self.primary_key = columns;
    }
}

/// Column mapping
#[derive(Debug, Clone)]
pub struct ColumnMapping {
    pub property_name: String,
    pub column_name: String,
    pub column_type: String,
    pub nullable: bool,
}

impl ColumnMapping {
    pub fn new(
        property_name: impl Into<String>,
        column_name: impl Into<String>,
        column_type: impl Into<String>,
    ) -> Self {
        Self {
            property_name: property_name.into(),
            column_name: column_name.into(),
            column_type: column_type.into(),
            nullable: true,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapper_registry_new() {
        let registry = MapperRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_mapper_registry_register() {
        let registry = MapperRegistry::new();
        let mapper = EntityMapper::new("users");
        registry.register("User", mapper);
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_mapper_registry_get() {
        let registry = MapperRegistry::new();
        let mapper = EntityMapper::new("users");
        registry.register("User", mapper);

        let retrieved = registry.get("User");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().table_name, "users");
    }

    #[test]
    fn test_mapper_registry_remove() {
        let registry = MapperRegistry::new();
        let mapper = EntityMapper::new("users");
        registry.register("User", mapper);

        assert!(registry.remove("User"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_mapper_registry_clear() {
        let registry = MapperRegistry::new();
        registry.register("User", EntityMapper::new("users"));
        registry.register("Post", EntityMapper::new("posts"));

        registry.clear();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_entity_mapper_new() {
        let mapper = EntityMapper::new("users");
        assert_eq!(mapper.table_name, "users");
        assert_eq!(mapper.columns.len(), 0);
        assert_eq!(mapper.primary_key.len(), 0);
    }

    #[test]
    fn test_entity_mapper_add_column() {
        let mut mapper = EntityMapper::new("users");
        let column = ColumnMapping::new("id", "id", "INTEGER");
        mapper.add_column(column);

        assert_eq!(mapper.columns.len(), 1);
        assert_eq!(mapper.columns[0].property_name, "id");
    }

    #[test]
    fn test_entity_mapper_set_primary_key() {
        let mut mapper = EntityMapper::new("users");
        mapper.set_primary_key(vec!["id".to_string()]);

        assert_eq!(mapper.primary_key.len(), 1);
        assert_eq!(mapper.primary_key[0], "id");
    }

    #[test]
    fn test_column_mapping_new() {
        let column = ColumnMapping::new("username", "user_name", "VARCHAR");
        assert_eq!(column.property_name, "username");
        assert_eq!(column.column_name, "user_name");
        assert_eq!(column.column_type, "VARCHAR");
        assert!(column.nullable);
    }

    #[test]
    fn test_column_mapping_not_null() {
        let column = ColumnMapping::new("id", "id", "INTEGER").not_null();
        assert!(!column.nullable);
    }
}

// ============================================================================
// Type-safe entity registry (compile-time checked)
// ============================================================================

/// Trait for entities that can be registered in the ORM
///
/// Implement this trait for each entity/model in your application.
/// The compiler will ensure that only valid entity types can be used.
///
/// # Example
///
/// ```rust
/// use reinhardt_orm::registry::EntityType;
///
/// pub struct User {
///     pub id: i64,
///     pub name: String,
/// }
///
/// impl EntityType for User {
///     const NAME: &'static str = "User";
///     const TABLE_NAME: &'static str = "users";
/// }
/// ```
pub trait EntityType {
    /// The entity name
    const NAME: &'static str;

    /// The table name
    const TABLE_NAME: &'static str;
}

impl MapperRegistry {
    /// Type-safe get method
    ///
    /// Get an entity mapper using compile-time verified entity types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::registry::{MapperRegistry, EntityType};
    ///
    /// pub struct User;
    /// impl EntityType for User {
    ///     const NAME: &'static str = "User";
    ///     const TABLE_NAME: &'static str = "users";
    /// }
    ///
    /// let registry = MapperRegistry::new();
    /// let mapper = registry.get_typed::<User>();
    /// ```
    pub fn get_typed<E: EntityType>(&self) -> Option<EntityMapper> {
        self.get(E::NAME)
    }

    /// Type-safe register method
    ///
    /// Register an entity mapper using compile-time verified entity types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::registry::{MapperRegistry, EntityType, EntityMapper};
    ///
    /// pub struct Post;
    /// impl EntityType for Post {
    ///     const NAME: &'static str = "Post";
    ///     const TABLE_NAME: &'static str = "posts";
    /// }
    ///
    /// let registry = MapperRegistry::new();
    /// let mapper = EntityMapper::new(Post::TABLE_NAME);
    /// registry.register_typed::<Post>(mapper);
    /// ```
    pub fn register_typed<E: EntityType>(&self, mapper: EntityMapper) {
        self.register(E::NAME, mapper);
    }

    /// Type-safe remove method
    ///
    /// Remove an entity mapper using compile-time verified entity types.
    pub fn remove_typed<E: EntityType>(&self) -> bool {
        self.remove(E::NAME)
    }
}

#[cfg(test)]
mod typed_tests {
    use super::*;

    // Test entity types
    struct User;
    impl EntityType for User {
        const NAME: &'static str = "User";
        const TABLE_NAME: &'static str = "users";
    }

    struct Post;
    impl EntityType for Post {
        const NAME: &'static str = "Post";
        const TABLE_NAME: &'static str = "posts";
    }

    struct Comment;
    impl EntityType for Comment {
        const NAME: &'static str = "Comment";
        const TABLE_NAME: &'static str = "comments";
    }

    #[test]
    fn test_registry_typed_register() {
        let registry = MapperRegistry::new();
        let mapper = EntityMapper::new(User::TABLE_NAME);
        registry.register_typed::<User>(mapper);

        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_registry_typed_get() {
        let registry = MapperRegistry::new();
        let mapper = EntityMapper::new(Post::TABLE_NAME);
        registry.register_typed::<Post>(mapper);

        let retrieved = registry.get_typed::<Post>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().table_name, "posts");
    }

    #[test]
    fn test_registry_typed_get_not_found() {
        let registry = MapperRegistry::new();

        let result = registry.get_typed::<Comment>();
        assert!(result.is_none());
    }

    #[test]
    fn test_typed_remove() {
        let registry = MapperRegistry::new();
        let mapper = EntityMapper::new(User::TABLE_NAME);
        registry.register_typed::<User>(mapper);

        assert_eq!(registry.count(), 1);

        let removed = registry.remove_typed::<User>();
        assert!(removed);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_typed_and_regular_mixed() {
        let registry = MapperRegistry::new();

        // Register using typed method
        let mapper = EntityMapper::new(User::TABLE_NAME);
        registry.register_typed::<User>(mapper);

        // Can access using both methods
        let typed = registry.get_typed::<User>();
        let regular = registry.get("User");

        assert!(typed.is_some());
        assert!(regular.is_some());
        assert_eq!(typed.unwrap().table_name, regular.unwrap().table_name);
    }
}
