//! Automatic schema inference from Rust types

use crate::fields::{FieldInfo, FieldInfoBuilder};
use crate::schema::OpenApiSchema;
use crate::types::FieldType;
use std::collections::HashMap;

/// Schema inferencer for automatic type detection
///
/// # Examples
///
/// ```
/// use reinhardt_metadata::{SchemaInferencer, FieldType};
///
/// let inferencer = SchemaInferencer::new();
/// let schema = inferencer.infer_from_type_name("String");
/// assert_eq!(schema.field_type, FieldType::String);
/// ```
#[derive(Debug, Clone)]
pub struct SchemaInferencer {
    type_mappings: HashMap<String, FieldType>,
}

impl Default for SchemaInferencer {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaInferencer {
    /// Creates a new schema inferencer with default type mappings
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_metadata::SchemaInferencer;
    ///
    /// let inferencer = SchemaInferencer::new();
    /// ```
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // Primitive types
        type_mappings.insert("bool".to_string(), FieldType::Boolean);
        type_mappings.insert("String".to_string(), FieldType::String);
        type_mappings.insert("str".to_string(), FieldType::String);

        // Integer types
        type_mappings.insert("i8".to_string(), FieldType::Integer);
        type_mappings.insert("i16".to_string(), FieldType::Integer);
        type_mappings.insert("i32".to_string(), FieldType::Integer);
        type_mappings.insert("i64".to_string(), FieldType::Integer);
        type_mappings.insert("i128".to_string(), FieldType::Integer);
        type_mappings.insert("isize".to_string(), FieldType::Integer);
        type_mappings.insert("u8".to_string(), FieldType::Integer);
        type_mappings.insert("u16".to_string(), FieldType::Integer);
        type_mappings.insert("u32".to_string(), FieldType::Integer);
        type_mappings.insert("u64".to_string(), FieldType::Integer);
        type_mappings.insert("u128".to_string(), FieldType::Integer);
        type_mappings.insert("usize".to_string(), FieldType::Integer);

        // Float types
        type_mappings.insert("f32".to_string(), FieldType::Float);
        type_mappings.insert("f64".to_string(), FieldType::Float);

        // Date/Time types
        type_mappings.insert("NaiveDate".to_string(), FieldType::Date);
        type_mappings.insert("NaiveDateTime".to_string(), FieldType::DateTime);
        type_mappings.insert("DateTime".to_string(), FieldType::DateTime);
        type_mappings.insert("NaiveTime".to_string(), FieldType::Time);
        type_mappings.insert("Duration".to_string(), FieldType::Duration);

        // Special types
        type_mappings.insert("Uuid".to_string(), FieldType::Uuid);

        Self { type_mappings }
    }

    /// Infers field info from a Rust type name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_metadata::{SchemaInferencer, FieldType};
    ///
    /// let inferencer = SchemaInferencer::new();
    ///
    /// // Primitive types
    /// let field = inferencer.infer_from_type_name("String");
    /// assert_eq!(field.field_type, FieldType::String);
    ///
    /// let field = inferencer.infer_from_type_name("i64");
    /// assert_eq!(field.field_type, FieldType::Integer);
    ///
    /// let field = inferencer.infer_from_type_name("bool");
    /// assert_eq!(field.field_type, FieldType::Boolean);
    /// ```
    pub fn infer_from_type_name(&self, type_name: &str) -> FieldInfo {
        // Handle Vec<T>
        if let Some(inner_type) = self.extract_vec_type(type_name) {
            let child = self.infer_from_type_name(inner_type);
            return FieldInfoBuilder::new(FieldType::List).child(child).build();
        }

        // Handle Option<T>
        if let Some(inner_type) = self.extract_option_type(type_name) {
            let mut field = self.infer_from_type_name(inner_type);
            field.required = false;
            return field;
        }

        // Look up in type mappings
        let field_type = self
            .type_mappings
            .get(type_name)
            .cloned()
            .unwrap_or(FieldType::Field);

        FieldInfoBuilder::new(field_type).build()
    }

    /// Registers a custom type mapping
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_metadata::{SchemaInferencer, FieldType};
    ///
    /// let mut inferencer = SchemaInferencer::new();
    /// inferencer.register_type("UserId", FieldType::Uuid);
    ///
    /// let field = inferencer.infer_from_type_name("UserId");
    /// assert_eq!(field.field_type, FieldType::Uuid);
    /// ```
    pub fn register_type(&mut self, type_name: impl Into<String>, field_type: FieldType) {
        self.type_mappings.insert(type_name.into(), field_type);
    }

    /// Extracts the inner type from Vec<T>
    fn extract_vec_type<'a>(&self, type_name: &'a str) -> Option<&'a str> {
        if type_name.starts_with("Vec<") && type_name.ends_with('>') {
            Some(&type_name[4..type_name.len() - 1])
        } else {
            None
        }
    }

    /// Extracts the inner type from Option<T>
    fn extract_option_type<'a>(&self, type_name: &'a str) -> Option<&'a str> {
        if type_name.starts_with("Option<") && type_name.ends_with('>') {
            Some(&type_name[7..type_name.len() - 1])
        } else {
            None
        }
    }

    /// Infers an OpenAPI schema from a Rust type name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_metadata::SchemaInferencer;
    ///
    /// let inferencer = SchemaInferencer::new();
    /// let schema = inferencer.infer_openapi_schema("String");
    /// assert_eq!(schema.schema_type, Some("string".to_string()));
    /// ```
    pub fn infer_openapi_schema(&self, type_name: &str) -> OpenApiSchema {
        let field = self.infer_from_type_name(type_name);
        crate::schema::generate_field_schema(&field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_primitive_types() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("bool");
        assert_eq!(field.field_type, FieldType::Boolean);

        let field = inferencer.infer_from_type_name("String");
        assert_eq!(field.field_type, FieldType::String);

        let field = inferencer.infer_from_type_name("i64");
        assert_eq!(field.field_type, FieldType::Integer);

        let field = inferencer.infer_from_type_name("f64");
        assert_eq!(field.field_type, FieldType::Float);
    }

    #[test]
    fn test_infer_all_integer_types() {
        let inferencer = SchemaInferencer::new();

        for int_type in &["i8", "i16", "i32", "i64", "i128", "isize"] {
            let field = inferencer.infer_from_type_name(int_type);
            assert_eq!(
                field.field_type,
                FieldType::Integer,
                "Failed for type: {}",
                int_type
            );
        }

        for uint_type in &["u8", "u16", "u32", "u64", "u128", "usize"] {
            let field = inferencer.infer_from_type_name(uint_type);
            assert_eq!(
                field.field_type,
                FieldType::Integer,
                "Failed for type: {}",
                uint_type
            );
        }
    }

    #[test]
    fn test_infer_datetime_types() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("NaiveDate");
        assert_eq!(field.field_type, FieldType::Date);

        let field = inferencer.infer_from_type_name("DateTime");
        assert_eq!(field.field_type, FieldType::DateTime);

        let field = inferencer.infer_from_type_name("NaiveTime");
        assert_eq!(field.field_type, FieldType::Time);

        let field = inferencer.infer_from_type_name("Duration");
        assert_eq!(field.field_type, FieldType::Duration);
    }

    #[test]
    fn test_infer_uuid_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("Uuid");
        assert_eq!(field.field_type, FieldType::Uuid);
    }

    #[test]
    fn test_infer_vec_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("Vec<String>");
        assert_eq!(field.field_type, FieldType::List);
        assert!(field.child.is_some());

        let child = field.child.unwrap();
        assert_eq!(child.field_type, FieldType::String);
    }

    #[test]
    fn test_infer_nested_vec_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("Vec<Vec<i32>>");
        assert_eq!(field.field_type, FieldType::List);
        assert!(field.child.is_some());

        let child = field.child.unwrap();
        assert_eq!(child.field_type, FieldType::List);
        assert!(child.child.is_some());

        let inner_child = child.child.unwrap();
        assert_eq!(inner_child.field_type, FieldType::Integer);
    }

    #[test]
    fn test_infer_option_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("Option<String>");
        assert_eq!(field.field_type, FieldType::String);
        assert!(!field.required);
    }

    #[test]
    fn test_infer_option_vec_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("Option<Vec<i64>>");
        assert_eq!(field.field_type, FieldType::List);
        assert!(!field.required);

        let child = field.child.as_ref().unwrap();
        assert_eq!(child.field_type, FieldType::Integer);
    }

    #[test]
    fn test_infer_vec_option_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("Vec<Option<String>>");
        assert_eq!(field.field_type, FieldType::List);
        assert!(!field.required); // Default is not required

        let child = field.child.as_ref().unwrap();
        assert_eq!(child.field_type, FieldType::String);
        assert!(!child.required); // Option<String> is not required
    }

    #[test]
    fn test_register_custom_type() {
        let mut inferencer = SchemaInferencer::new();
        inferencer.register_type("UserId", FieldType::Uuid);
        inferencer.register_type("Email", FieldType::Email);

        let field = inferencer.infer_from_type_name("UserId");
        assert_eq!(field.field_type, FieldType::Uuid);

        let field = inferencer.infer_from_type_name("Email");
        assert_eq!(field.field_type, FieldType::Email);
    }

    #[test]
    fn test_infer_unknown_type() {
        let inferencer = SchemaInferencer::new();

        let field = inferencer.infer_from_type_name("UnknownType");
        assert_eq!(field.field_type, FieldType::Field);
    }

    #[test]
    fn test_infer_openapi_schema_string() {
        let inferencer = SchemaInferencer::new();
        let schema = inferencer.infer_openapi_schema("String");

        assert_eq!(schema.schema_type, Some("string".to_string()));
    }

    #[test]
    fn test_infer_openapi_schema_integer() {
        let inferencer = SchemaInferencer::new();
        let schema = inferencer.infer_openapi_schema("i64");

        assert_eq!(schema.schema_type, Some("integer".to_string()));
        assert_eq!(schema.format, Some("int64".to_string()));
    }

    #[test]
    fn test_infer_openapi_schema_list() {
        let inferencer = SchemaInferencer::new();
        let schema = inferencer.infer_openapi_schema("Vec<String>");

        assert_eq!(schema.schema_type, Some("array".to_string()));
        assert!(schema.items.is_some());

        let items = schema.items.unwrap();
        assert_eq!(items.schema_type, Some("string".to_string()));
    }

    #[test]
    fn test_infer_openapi_schema_datetime() {
        let inferencer = SchemaInferencer::new();
        let schema = inferencer.infer_openapi_schema("DateTime");

        assert_eq!(schema.schema_type, Some("string".to_string()));
        assert_eq!(schema.format, Some("date-time".to_string()));
    }

    #[test]
    fn test_infer_openapi_schema_uuid() {
        let inferencer = SchemaInferencer::new();
        let schema = inferencer.infer_openapi_schema("Uuid");

        assert_eq!(schema.schema_type, Some("string".to_string()));
        assert_eq!(schema.format, Some("uuid".to_string()));
    }
}
