/// DDL reference objects for schema operations
///
/// This module provides reference types for database schema objects,
/// inspired by Django's ddl_references module.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::backends::schema::ddl_references::{Table, Columns};
///
/// let table = Table::new("users", Some("public"));
/// assert_eq!(table.name(), "users");
/// assert_eq!(table.schema(), Some("public"));
///
/// let columns = Columns::new("users", &["id", "name"]);
/// assert_eq!(columns.columns(), &["id", "name"]);
/// ```
use std::fmt;

/// Reference to a database table
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Table {
    name: String,
    schema: Option<String>,
}

impl Table {
    /// Create a new table reference
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_db::backends::schema::ddl_references::Table;
    ///
    /// let table = Table::new("users", None::<String>);
    /// assert_eq!(table.name(), "users");
    /// assert_eq!(table.schema(), None);
    ///
    /// let schema_table = Table::new("posts", Some("blog"));
    /// assert_eq!(schema_table.schema(), Some("blog"));
    /// ```
    pub fn new(name: impl Into<String>, schema: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            schema: schema.map(|s| s.into()),
        }
    }

    /// Get the table name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the schema name
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Get the fully qualified table name
    pub fn qualified_name(&self) -> String {
        if let Some(schema) = &self.schema {
            format!("{}.{}", schema, self.name)
        } else {
            self.name.clone()
        }
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.qualified_name())
    }
}

/// Reference to table columns
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Columns {
    table: String,
    columns: Vec<String>,
}

impl Columns {
    /// Create a new columns reference
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_db::backends::schema::ddl_references::Columns;
    ///
    /// let columns = Columns::new("users", &["id", "name", "email"]);
    /// assert_eq!(columns.table(), "users");
    /// assert_eq!(columns.columns(), &["id", "name", "email"]);
    /// ```
    pub fn new(table: impl Into<String>, columns: &[impl AsRef<str>]) -> Self {
        Self {
            table: table.into(),
            columns: columns.iter().map(|c| c.as_ref().to_string()).collect(),
        }
    }

    /// Get the table name
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Get the column names
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Get the column names as a comma-separated string
    pub fn columns_str(&self) -> String {
        self.columns.join(", ")
    }
}

impl fmt::Display for Columns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.columns_str())
    }
}

/// Reference to an index name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexName {
    table: String,
    columns: Vec<String>,
    suffix: String,
}

impl IndexName {
    /// Create a new index name reference
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_db::backends::schema::ddl_references::IndexName;
    ///
    /// let idx = IndexName::new("users", &["email"], "idx");
    /// assert_eq!(idx.table(), "users");
    /// assert_eq!(idx.suffix(), "idx");
    /// ```
    pub fn new(
        table: impl Into<String>,
        columns: &[impl AsRef<str>],
        suffix: impl Into<String>,
    ) -> Self {
        Self {
            table: table.into(),
            columns: columns.iter().map(|c| c.as_ref().to_string()).collect(),
            suffix: suffix.into(),
        }
    }

    /// Get the table name
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Get the column names
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Get the suffix
    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    /// Generate the index name
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_db::backends::schema::ddl_references::IndexName;
    ///
    /// let idx = IndexName::new("users", &["email"], "idx");
    /// let name = idx.generate_name();
    /// assert!(name.starts_with("users"));
    /// assert!(name.contains("email"));
    /// assert!(name.contains("idx"));
    /// ```
    pub fn generate_name(&self) -> String {
        let cols = self.columns.join("_");
        format!(
            "{}_{}_{}_{}",
            self.table,
            cols,
            self.suffix,
            hash_name(&cols)
        )
    }
}

impl fmt::Display for IndexName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.generate_name())
    }
}

/// Reference to a foreign key constraint name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForeignKeyName {
    from_table: String,
    from_columns: Vec<String>,
    to_table: String,
    to_columns: Vec<String>,
    suffix: String,
}

impl ForeignKeyName {
    /// Create a new foreign key name reference
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_db::backends::schema::ddl_references::ForeignKeyName;
    ///
    /// let fk = ForeignKeyName::new(
    ///     "posts",
    ///     &["user_id"],
    ///     "users",
    ///     &["id"],
    ///     "fk"
    /// );
    /// assert_eq!(fk.from_table(), "posts");
    /// assert_eq!(fk.to_table(), "users");
    /// ```
    pub fn new(
        from_table: impl Into<String>,
        from_columns: &[impl AsRef<str>],
        to_table: impl Into<String>,
        to_columns: &[impl AsRef<str>],
        suffix: impl Into<String>,
    ) -> Self {
        Self {
            from_table: from_table.into(),
            from_columns: from_columns
                .iter()
                .map(|c| c.as_ref().to_string())
                .collect(),
            to_table: to_table.into(),
            to_columns: to_columns.iter().map(|c| c.as_ref().to_string()).collect(),
            suffix: suffix.into(),
        }
    }

    /// Get the source table name
    pub fn from_table(&self) -> &str {
        &self.from_table
    }

    /// Get the target table name
    pub fn to_table(&self) -> &str {
        &self.to_table
    }

    /// Generate the foreign key constraint name
    pub fn generate_name(&self) -> String {
        let from_cols = self.from_columns.join("_");
        let to_cols = self.to_columns.join("_");
        format!(
            "{}_{}_{}_{}_{}_{}",
            self.from_table,
            from_cols,
            self.to_table,
            to_cols,
            self.suffix,
            hash_name(&format!("{}_{}", from_cols, to_cols))
        )
    }
}

impl fmt::Display for ForeignKeyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.generate_name())
    }
}

/// SQL statement wrapper
#[derive(Debug, Clone)]
pub struct Statement {
    template: String,
    params: Vec<(String, String)>,
}

impl Statement {
    /// Create a new SQL statement
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_db::backends::schema::ddl_references::Statement;
    ///
    /// let stmt = Statement::new("CREATE TABLE %(table)s (%(definition)s)");
    /// assert!(stmt.template().contains("CREATE TABLE"));
    /// ```
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            params: Vec::new(),
        }
    }

    /// Get the template string
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Set a parameter value
    pub fn set_param(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.push((key.into(), value.into()));
    }

    /// Render the statement with parameters
    pub fn render(&self) -> String {
        let mut result = self.template.clone();
        for (key, value) in &self.params {
            let placeholder = format!("%({})s", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Generate a short hash for name generation
fn hash_name(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash & 0xFFFF) // Use last 4 hex digits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_reference() {
        let table = Table::new("users", None::<String>);
        assert_eq!(table.name(), "users");
        assert_eq!(table.schema(), None);
        assert_eq!(table.qualified_name(), "users");

        let schema_table = Table::new("posts", Some("blog"));
        assert_eq!(schema_table.schema(), Some("blog"));
        assert_eq!(schema_table.qualified_name(), "blog.posts");
    }

    #[test]
    fn test_columns_reference() {
        let columns = Columns::new("users", &["id", "name", "email"]);
        assert_eq!(columns.table(), "users");
        assert_eq!(columns.columns(), &["id", "name", "email"]);
        assert_eq!(columns.columns_str(), "id, name, email");
    }

    #[test]
    fn test_index_name() {
        let idx = IndexName::new("users", &["email"], "idx");
        let name = idx.generate_name();

        assert!(name.starts_with("users"));
        assert!(name.contains("email"));
        assert!(name.contains("idx"));
    }

    #[test]
    fn test_foreign_key_name() {
        let fk = ForeignKeyName::new("posts", &["user_id"], "users", &["id"], "fk");
        assert_eq!(fk.from_table(), "posts");
        assert_eq!(fk.to_table(), "users");

        let name = fk.generate_name();
        assert!(name.contains("posts"));
        assert!(name.contains("user_id"));
        assert!(name.contains("users"));
    }

    #[test]
    fn test_statement() {
        let mut stmt = Statement::new("CREATE TABLE %(table)s (%(definition)s)");
        stmt.set_param("table", "users");
        stmt.set_param("definition", "id INTEGER");

        let rendered = stmt.render();
        assert_eq!(rendered, "CREATE TABLE users (id INTEGER)");
    }
}
