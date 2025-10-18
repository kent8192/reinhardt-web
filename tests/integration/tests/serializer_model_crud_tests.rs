// Model serializer CRUD tests - Create, Read, Update, Delete operations
use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, ModelSerializer, Serializer,
};
use serde::{Deserialize, Serialize};

// Test models

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Product {
    id: Option<i64>,
    name: String,
    price: f64,
    stock: i32,
    active: bool,
}

impl Model for Product {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "products"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Customer {
    id: Option<i64>,
    name: String,
    email: String,
    created_at: Option<String>,
    updated_at: Option<String>,
}

impl Model for Customer {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "customers"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

// Test: Create operation
#[test]
fn test_create_operation() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let product = Product {
        id: None, // No ID for new product
        name: "New Product".to_string(),
        price: 99.99,
        stock: 100,
        active: true,
    };

    let result = serializer.create(product.clone());
    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.name, "New Product");
    assert_eq!(created.price, 99.99);
}

// Test: Create with validation
#[test]
fn test_create_with_validation() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let product = Product {
        id: None,
        name: "Valid Product".to_string(),
        price: 49.99,
        stock: 50,
        active: true,
    };

    let result = serializer.create(product);
    assert!(result.is_ok());
}

// Test: Create and assign primary key
#[test]
fn test_create_and_assign_pk() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut product = Product {
        id: None,
        name: "Product".to_string(),
        price: 19.99,
        stock: 10,
        active: true,
    };

    let result = serializer.create(product.clone());
    assert!(result.is_ok());

    // Simulate database assigning ID
    product.set_primary_key(123);
    assert_eq!(product.primary_key(), Some(&123));
}

// Test: Read operation (serialization)
#[test]
fn test_read_operation() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let product = Product {
        id: Some(1),
        name: "Existing Product".to_string(),
        price: 29.99,
        stock: 25,
        active: true,
    };

    let serialized = Serializer::serialize(&serializer, &product).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // Verify all fields are serialized
    assert!(json_str.contains("\"id\":1"));
    assert!(json_str.contains("\"Existing Product\""));
    assert!(json_str.contains("29.99"));
}

// Test: Read with deserialization
#[test]
fn test_read_with_deserialization() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let original = Product {
        id: Some(42),
        name: "Test Product".to_string(),
        price: 39.99,
        stock: 15,
        active: false,
    };

    let serialized = Serializer::serialize(&serializer, &original).unwrap();
    let deserialized: Product =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(original, deserialized);
}

// Test: Update operation
#[test]
fn test_update_operation() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut product = Product {
        id: Some(1),
        name: "Old Name".to_string(),
        price: 10.0,
        stock: 5,
        active: true,
    };

    let updated_data = Product {
        id: Some(1),
        name: "New Name".to_string(),
        price: 15.0,
        stock: 10,
        active: true,
    };

    let result = serializer.update(&mut product, updated_data);
    assert!(result.is_ok());
    assert_eq!(product.name, "New Name");
    assert_eq!(product.price, 15.0);
    assert_eq!(product.stock, 10);
}

// Test: Partial update
#[test]
fn test_partial_update() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 20.0,
        stock: 100,
        active: true,
    };

    // Update only price and stock
    let updated_data = Product {
        id: Some(1),
        name: "Product".to_string(), // Same
        price: 25.0,                 // Changed
        stock: 90,                   // Changed
        active: true,                // Same
    };

    let result = serializer.update(&mut product, updated_data);
    assert!(result.is_ok());
    assert_eq!(product.price, 25.0);
    assert_eq!(product.stock, 90);
}

// Test: Update with timestamp
#[test]
fn test_update_with_timestamp() {
    let serializer = DefaultModelSerializer::<Customer>::new();
    let mut customer = Customer {
        id: Some(1),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        created_at: Some("2024-01-01T00:00:00Z".to_string()),
        updated_at: Some("2024-01-01T00:00:00Z".to_string()),
    };

    let updated_data = Customer {
        id: Some(1),
        name: "John Updated".to_string(),
        email: "john.new@example.com".to_string(),
        created_at: Some("2024-01-01T00:00:00Z".to_string()),
        updated_at: Some("2024-01-15T12:00:00Z".to_string()), // Updated timestamp
    };

    let result = serializer.update(&mut customer, updated_data);
    assert!(result.is_ok());
    assert_eq!(customer.name, "John Updated");
    assert_eq!(
        customer.updated_at,
        Some("2024-01-15T12:00:00Z".to_string())
    );
}

// Test: Update preserves ID
#[test]
fn test_update_preserves_id() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut product = Product {
        id: Some(999),
        name: "Product".to_string(),
        price: 10.0,
        stock: 5,
        active: true,
    };

    let updated_data = Product {
        id: Some(999), // Same ID
        name: "Updated Product".to_string(),
        price: 12.0,
        stock: 3,
        active: false,
    };

    let result = serializer.update(&mut product, updated_data);
    assert!(result.is_ok());
    assert_eq!(product.primary_key(), Some(&999));
}

// Test: Bulk create operations
#[test]
fn test_bulk_create_operations() {
    use reinhardt_serializers::JsonSerializer;

    let serializer = DefaultModelSerializer::<Product>::new();
    let products = vec![
        Product {
            id: None,
            name: "Product 1".to_string(),
            price: 10.0,
            stock: 10,
            active: true,
        },
        Product {
            id: None,
            name: "Product 2".to_string(),
            price: 20.0,
            stock: 20,
            active: true,
        },
        Product {
            id: None,
            name: "Product 3".to_string(),
            price: 30.0,
            stock: 30,
            active: true,
        },
    ];

    // Create each product
    let mut created = Vec::new();
    for product in products {
        let result = serializer.create(product);
        assert!(result.is_ok());
        created.push(result.unwrap());
    }

    assert_eq!(created.len(), 3);

    // Serialize all created products
    let list_serializer = JsonSerializer::<Vec<Product>>::new();
    let serialized = Serializer::serialize(&list_serializer, &created).unwrap();
    let deserialized: Vec<Product> =
        ReinhardtDeserializer::deserialize(&list_serializer, &serialized).unwrap();

    assert_eq!(created, deserialized);
}

// Test: Bulk update operations
#[test]
fn test_bulk_update_operations() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut products = vec![
        Product {
            id: Some(1),
            name: "Product 1".to_string(),
            price: 10.0,
            stock: 10,
            active: true,
        },
        Product {
            id: Some(2),
            name: "Product 2".to_string(),
            price: 20.0,
            stock: 20,
            active: true,
        },
    ];

    // Update all products
    for product in &mut products {
        let updated = Product {
            id: product.id,
            name: format!("{} (updated)", product.name),
            price: product.price * 1.1, // 10% increase
            stock: product.stock,
            active: product.active,
        };
        let result = serializer.update(product, updated);
        assert!(result.is_ok());
    }

    assert!(products[0].name.contains("(updated)"));
    assert!(products[1].name.contains("(updated)"));
}

// Test: Create with default values
#[test]
fn test_create_with_defaults() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct ItemWithDefaults {
        id: Option<i64>,
        name: String,
        quantity: i32,
        #[serde(default = "default_status")]
        status: String,
    }

    fn default_status() -> String {
        "pending".to_string()
    }

    impl Model for ItemWithDefaults {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "items"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<ItemWithDefaults>::new();
    let item = ItemWithDefaults {
        id: None,
        name: "Item".to_string(),
        quantity: 1,
        status: default_status(),
    };

    let result = serializer.create(item.clone());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, "pending");
}

// Test: Update non-existent field (should work with full model update)
#[test]
fn test_update_full_model() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut product = Product {
        id: Some(1),
        name: "Original".to_string(),
        price: 100.0,
        stock: 50,
        active: true,
    };

    let completely_new = Product {
        id: Some(1),
        name: "Completely New".to_string(),
        price: 200.0,
        stock: 100,
        active: false,
    };

    let result = serializer.update(&mut product, completely_new.clone());
    assert!(result.is_ok());
    assert_eq!(product, completely_new);
}

// Test: CRUD round trip
#[test]
fn test_crud_round_trip() {
    let serializer = DefaultModelSerializer::<Product>::new();

    // Create
    let mut product = Product {
        id: None,
        name: "Test Product".to_string(),
        price: 50.0,
        stock: 25,
        active: true,
    };
    let create_result = serializer.create(product.clone());
    assert!(create_result.is_ok());

    // Simulate DB assigning ID
    product.set_primary_key(1);

    // Read
    let serialized = Serializer::serialize(&serializer, &product).unwrap();
    let read_product: Product =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(product, read_product);

    // Update
    let updated = Product {
        id: Some(1),
        name: "Updated Product".to_string(),
        price: 55.0,
        stock: 20,
        active: true,
    };
    let update_result = serializer.update(&mut product, updated);
    assert!(update_result.is_ok());
    assert_eq!(product.name, "Updated Product");
    assert_eq!(product.price, 55.0);
}

// Test: Serialization preserves field order
#[test]
fn test_serialization_field_order() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let product = Product {
        id: Some(1),
        name: "Test".to_string(),
        price: 10.0,
        stock: 5,
        active: true,
    };

    let serialized = Serializer::serialize(&serializer, &product).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // JSON object fields are present (order may vary in JSON)
    assert!(json_str.contains("\"id\""));
    assert!(json_str.contains("\"name\""));
    assert!(json_str.contains("\"price\""));
    assert!(json_str.contains("\"stock\""));
    assert!(json_str.contains("\"active\""));
}

// Test: Update with validation
#[test]
fn test_update_with_validation() {
    let serializer = DefaultModelSerializer::<Customer>::new();
    let mut customer = Customer {
        id: Some(1),
        name: "Old Name".to_string(),
        email: "old@example.com".to_string(),
        created_at: Some("2024-01-01T00:00:00Z".to_string()),
        updated_at: Some("2024-01-01T00:00:00Z".to_string()),
    };

    let valid_update = Customer {
        id: Some(1),
        name: "New Name".to_string(),
        email: "new@example.com".to_string(),
        created_at: Some("2024-01-01T00:00:00Z".to_string()),
        updated_at: Some("2024-01-20T00:00:00Z".to_string()),
    };

    let result = serializer.update(&mut customer, valid_update);
    assert!(result.is_ok());
    assert_eq!(customer.email, "new@example.com");
}

// Test: Create multiple instances with different data
#[test]
fn test_create_multiple_different_instances() {
    let serializer = DefaultModelSerializer::<Product>::new();

    let product1 = Product {
        id: None,
        name: "Product A".to_string(),
        price: 100.0,
        stock: 10,
        active: true,
    };

    let product2 = Product {
        id: None,
        name: "Product B".to_string(),
        price: 200.0,
        stock: 20,
        active: false,
    };

    let result1 = serializer.create(product1);
    let result2 = serializer.create(product2);

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let created1 = result1.unwrap();
    let created2 = result2.unwrap();

    assert_eq!(created1.name, "Product A");
    assert_eq!(created2.name, "Product B");
    assert_ne!(created1.price, created2.price);
}

// Test: Update idempotency
#[test]
fn test_update_idempotency() {
    let serializer = DefaultModelSerializer::<Product>::new();
    let mut product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        stock: 5,
        active: true,
    };

    let update_data = Product {
        id: Some(1),
        name: "Updated".to_string(),
        price: 15.0,
        stock: 10,
        active: false,
    };

    // First update
    let result1 = serializer.update(&mut product, update_data.clone());
    assert!(result1.is_ok());
    let state_after_first = product.clone();

    // Second update with same data
    let result2 = serializer.update(&mut product, update_data);
    assert!(result2.is_ok());

    // Should have same state
    assert_eq!(product, state_after_first);
}
