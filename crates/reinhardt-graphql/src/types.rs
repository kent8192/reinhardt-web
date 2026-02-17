//! GraphQL type definitions

/// GraphQL type marker
pub trait GraphQLType {}

/// GraphQL field marker
pub trait GraphQLField {}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	struct TestType {
		id: String,
		value: i32,
	}

	impl GraphQLType for TestType {}

	struct TestField {
		name: String,
	}

	impl GraphQLField for TestField {}

	#[rstest]
	fn test_graphql_type_marker() {
		let test_type = TestType {
			id: "test-1".to_string(),
			value: 42,
		};

		// Verify the type implements GraphQLType
		fn assert_graphql_type<T: GraphQLType>(_: &T) {}
		assert_graphql_type(&test_type);

		// Verify data is accessible
		assert_eq!(test_type.id, "test-1");
		assert_eq!(test_type.value, 42);
	}

	#[rstest]
	fn test_graphql_field_marker() {
		let test_field = TestField {
			name: "test_field".to_string(),
		};

		// Verify the type implements GraphQLField
		fn assert_graphql_field<T: GraphQLField>(_: &T) {}
		assert_graphql_field(&test_field);

		// Verify data is accessible
		assert_eq!(test_field.name, "test_field");
	}

	// Test that multiple types can implement the markers
	struct AnotherType;
	impl GraphQLType for AnotherType {}

	struct AnotherField;
	impl GraphQLField for AnotherField {}

	#[rstest]
	fn test_multiple_implementations() {
		let type1 = TestType {
			id: "1".to_string(),
			value: 1,
		};
		let type2 = AnotherType;

		fn assert_graphql_type<T: GraphQLType>(_: &T) {}
		assert_graphql_type(&type1);
		assert_graphql_type(&type2);

		let field1 = TestField {
			name: "field1".to_string(),
		};
		let field2 = AnotherField;

		fn assert_graphql_field<T: GraphQLField>(_: &T) {}
		assert_graphql_field(&field1);
		assert_graphql_field(&field2);
	}
}
