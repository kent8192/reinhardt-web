//! Type-safe JOIN conditions using the FieldLookup system
//!
//! This module provides compile-time type safety for JOIN operations
//! by leveraging the existing Field<M, T> type system.

use crate::Model;
use crate::query_fields::Field;
use crate::sqlalchemy_query::JoinType;
use std::marker::PhantomData;

/// Type-safe JOIN condition between two models
///
/// The generic parameters L (Left) and R (Right) represent the models being joined.
/// This structure ensures that joined fields have compatible types at compile time.
///
/// # Example
///
/// ```rust,ignore
// This compiles: both fields are i64
/// TypedJoin::on(User::id(), Post::user_id())
///
// This fails: i64 vs String type mismatch
/// TypedJoin::on(User::id(), Post::title())
/// ```
pub struct TypedJoin<L: Model, R: Model> {
    right_table: &'static str,
    left_field_path: Vec<&'static str>,
    right_field_path: Vec<&'static str>,
    join_type: JoinType,
    _phantom: PhantomData<(L, R)>,
}

impl<L: Model, R: Model> TypedJoin<L, R> {
    /// Create an INNER JOIN between two fields of the same type
    ///
    /// # Type Safety
    ///
    /// The compiler enforces that both fields have the same type T.
    /// This prevents joining incompatible columns like integers with strings.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// TypedJoin::on(User::id(), Post::user_id())
    /// ```
    pub fn on<T>(left: Field<L, T>, right: Field<R, T>) -> Self {
        Self {
            right_table: R::table_name(),
            left_field_path: left.path().to_vec(),
            right_field_path: right.path().to_vec(),
            join_type: JoinType::Inner,
            _phantom: PhantomData,
        }
    }

    /// Create a LEFT OUTER JOIN between two fields of the same type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// TypedJoin::left_on(User::id(), Post::user_id())
    /// ```
    pub fn left_on<T>(left: Field<L, T>, right: Field<R, T>) -> Self {
        Self {
            right_table: R::table_name(),
            left_field_path: left.path().to_vec(),
            right_field_path: right.path().to_vec(),
            join_type: JoinType::Left,
            _phantom: PhantomData,
        }
    }

    /// Create a RIGHT OUTER JOIN between two fields of the same type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// TypedJoin::right_on(User::id(), Post::user_id())
    /// ```
    pub fn right_on<T>(left: Field<L, T>, right: Field<R, T>) -> Self {
        Self {
            right_table: R::table_name(),
            left_field_path: left.path().to_vec(),
            right_field_path: right.path().to_vec(),
            join_type: JoinType::Right,
            _phantom: PhantomData,
        }
    }

    /// Create a FULL OUTER JOIN between two fields of the same type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// TypedJoin::full_on(User::id(), Post::user_id())
    /// ```
    pub fn full_on<T>(left: Field<L, T>, right: Field<R, T>) -> Self {
        Self {
            right_table: R::table_name(),
            left_field_path: left.path().to_vec(),
            right_field_path: right.path().to_vec(),
            join_type: JoinType::Full,
            _phantom: PhantomData,
        }
    }

    /// Convert the typed join into SQL components
    ///
    /// Returns a tuple of (table_name, join_type, condition)
    /// suitable for use in SelectQuery.
    pub fn to_sql(&self) -> (String, JoinType, String) {
        let table = self.right_table.to_string();

        // Build the join condition: left_table.left_field = right_table.right_field
        let left_field = self.left_field_path.join(".");
        let right_field = self.right_field_path.join(".");

        let condition = format!(
            "{}.{} = {}.{}",
            L::table_name(),
            left_field,
            self.right_table,
            right_field
        );

        (table, self.join_type, condition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_validators::TableName;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct User {
        id: i64,
        email: String,
    }

    const USER_TABLE: TableName = TableName::new_const("users");

    impl Model for User {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            USER_TABLE.as_str()
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            Some(&self.id)
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = value;
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Post {
        id: i64,
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
            Some(&self.id)
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = value;
        }
    }

    #[test]
    fn test_typed_join_compiles() {
        let join = TypedJoin::on(
            Field::<User, i64>::new(vec!["id"]),
            Field::<Post, i64>::new(vec!["user_id"]),
        );

        let (table, join_type, condition) = join.to_sql();
        assert_eq!(table, "posts");
        assert_eq!(join_type, JoinType::Inner);
        assert_eq!(condition, "users.id = posts.user_id");
    }

    #[test]
    fn test_left_join() {
        let join = TypedJoin::left_on(
            Field::<User, i64>::new(vec!["id"]),
            Field::<Post, i64>::new(vec!["user_id"]),
        );

        let (_, join_type, _) = join.to_sql();
        assert_eq!(join_type, JoinType::Left);
    }

    // This test won't compile if uncommented - demonstrating type safety
    // #[test]
    // fn test_incompatible_types() {
    //     let join = TypedJoin::on(
    //         Field::<User, i64>::new(vec!["id"]),
    //         Field::<Post, String>::new(vec!["title"]),  // Type mismatch!
    //     );
    // }
}
