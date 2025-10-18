//! API Views Integration Tests
//!
//! Tests for DRF-style API views (ListAPIView, CreateAPIView, etc.)
//! Inspired by Django REST Framework test_views.py

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_orm::Model;
use reinhardt_serializers::JsonSerializer;
use reinhardt_views::{
    CreateAPIView, DestroyAPIView, ListAPIView, ListCreateAPIView, RetrieveDestroyAPIView,
    RetrieveUpdateAPIView, RetrieveUpdateDestroyAPIView, UpdateAPIView, View,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Product {
    id: Option<i64>,
    name: String,
    price: f64,
    in_stock: bool,
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

// ============================================================================
// ListAPIView Tests (DRF: test_views.py - list views)
// ============================================================================

#[tokio::test]
async fn test_list_api_view_get_success() {
    let products = vec![
        Product {
            id: Some(1),
            name: "Product 1".to_string(),
            price: 10.0,
            in_stock: true,
        },
        Product {
            id: Some(2),
            name: "Product 2".to_string(),
            price: 20.0,
            in_stock: false,
        },
    ];

    let view = ListAPIView::<Product, JsonSerializer<Product>>::new().with_objects(products);

    let request = Request::new(
        Method::GET,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);
}

#[tokio::test]
async fn test_list_api_view_post_not_allowed() {
    let view = ListAPIView::<Product, JsonSerializer<Product>>::new().with_objects(vec![]);

    let request = Request::new(
        Method::POST,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_list_api_view_empty_list() {
    let view = ListAPIView::<Product, JsonSerializer<Product>>::new().with_objects(vec![]);

    let request = Request::new(
        Method::GET,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert_eq!(resp.status, 200);

    let body: Vec<serde_json::Value> = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(body.len(), 0);
}

// ============================================================================
// CreateAPIView Tests (DRF: test_views.py - create views)
// ============================================================================

#[tokio::test]
async fn test_create_api_view_post_success() {
    let view = CreateAPIView::<Product, JsonSerializer<Product>>::new();

    let product_json = r#"{"id":1,"name":"New Product","price":15.5,"in_stock":true}"#;
    let request = Request::new(
        Method::POST,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(product_json),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    let resp = response.unwrap();
    // CreateAPIView should return 201 Created
    assert_eq!(resp.status, 201);
}

#[tokio::test]
async fn test_create_api_view_get_not_allowed() {
    let view = CreateAPIView::<Product, JsonSerializer<Product>>::new();

    let request = Request::new(
        Method::GET,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_create_api_view_invalid_json() {
    let view = CreateAPIView::<Product, JsonSerializer<Product>>::new();

    let request = Request::new(
        Method::POST,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from("invalid json"),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

// ============================================================================
// UpdateAPIView Tests (DRF: test_views.py - update views)
// ============================================================================

#[tokio::test]
async fn test_update_api_view_put_success() {
    let product = Product {
        id: Some(1),
        name: "Original Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view = UpdateAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let updated_json = r#"{"id":1,"name":"Updated Product","price":12.0,"in_stock":false}"#;
    let request = Request::new(
        Method::PUT,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(updated_json),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);
}

#[tokio::test]
async fn test_update_api_view_patch_success() {
    let product = Product {
        id: Some(1),
        name: "Original Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view = UpdateAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let patch_json = r#"{"price":15.0}"#;
    let request = Request::new(
        Method::PATCH,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(patch_json),
    );

    let response = view.dispatch(request).await;
    // PATCH now supports partial updates by merging with existing object
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert_eq!(resp.status, 200);
}

#[tokio::test]
async fn test_update_api_view_get_not_allowed() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view = UpdateAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let request = Request::new(
        Method::GET,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

// ============================================================================
// DestroyAPIView Tests (DRF: test_views.py - destroy views)
// ============================================================================

#[tokio::test]
async fn test_destroy_api_view_delete_success() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view = DestroyAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let request = Request::new(
        Method::DELETE,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 204); // No Content
}

#[tokio::test]
async fn test_destroy_api_view_get_not_allowed() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view = DestroyAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let request = Request::new(
        Method::GET,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

// ============================================================================
// Combined Views Tests (DRF: test_views.py - combined views)
// ============================================================================

#[tokio::test]
async fn test_list_create_api_view_get() {
    let products = vec![Product {
        id: Some(1),
        name: "Product 1".to_string(),
        price: 10.0,
        in_stock: true,
    }];

    let view = ListCreateAPIView::<Product, JsonSerializer<Product>>::new().with_objects(products);

    let request = Request::new(
        Method::GET,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);
}

#[tokio::test]
async fn test_list_create_api_view_post() {
    let view = ListCreateAPIView::<Product, JsonSerializer<Product>>::new().with_objects(vec![]);

    let product_json = r#"{"id":2,"name":"New Product","price":20.0,"in_stock":true}"#;
    let request = Request::new(
        Method::POST,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(product_json),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 201);
}

#[tokio::test]
async fn test_retrieve_update_api_view_get() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view =
        RetrieveUpdateAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let request = Request::new(
        Method::GET,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);
}

#[tokio::test]
async fn test_retrieve_update_api_view_put() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view =
        RetrieveUpdateAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let updated_json = r#"{"id":1,"name":"Updated","price":15.0,"in_stock":false}"#;
    let request = Request::new(
        Method::PUT,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(updated_json),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);
}

#[tokio::test]
async fn test_retrieve_destroy_api_view_get() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view =
        RetrieveDestroyAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let request = Request::new(
        Method::GET,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);
}

#[tokio::test]
async fn test_retrieve_destroy_api_view_delete() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view =
        RetrieveDestroyAPIView::<Product, JsonSerializer<Product>>::new().with_object(product);

    let request = Request::new(
        Method::DELETE,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 204);
}

#[tokio::test]
async fn test_retrieve_update_destroy_api_view_all_methods() {
    let product = Product {
        id: Some(1),
        name: "Product".to_string(),
        price: 10.0,
        in_stock: true,
    };

    let view = RetrieveUpdateDestroyAPIView::<Product, JsonSerializer<Product>>::new()
        .with_object(product.clone());

    // Test GET
    let get_request = Request::new(
        Method::GET,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let response = view.dispatch(get_request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);

    // Test PUT
    let view_put = RetrieveUpdateDestroyAPIView::<Product, JsonSerializer<Product>>::new()
        .with_object(product.clone());
    let updated_json = r#"{"id":1,"name":"Updated","price":15.0,"in_stock":false}"#;
    let put_request = Request::new(
        Method::PUT,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(updated_json),
    );
    let response = view_put.dispatch(put_request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 200);

    // Test DELETE
    let view_delete = RetrieveUpdateDestroyAPIView::<Product, JsonSerializer<Product>>::new()
        .with_object(product);
    let delete_request = Request::new(
        Method::DELETE,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let response = view_delete.dispatch(delete_request).await;
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, 204);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[tokio::test]
async fn test_create_api_view_empty_body() {
    let view = CreateAPIView::<Product, JsonSerializer<Product>>::new();

    let request = Request::new(
        Method::POST,
        "/api/products/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_update_api_view_without_object() {
    let view = UpdateAPIView::<Product, JsonSerializer<Product>>::new();

    let updated_json = r#"{"id":1,"name":"Updated","price":15.0,"in_stock":false}"#;
    let request = Request::new(
        Method::PUT,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(updated_json),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_destroy_api_view_without_object() {
    let view = DestroyAPIView::<Product, JsonSerializer<Product>>::new();

    let request = Request::new(
        Method::DELETE,
        "/api/products/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}
