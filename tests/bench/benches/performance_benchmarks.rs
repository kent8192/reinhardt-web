//! Performance benchmarks for Reinhardt testing utilities
//!
//! These benchmarks test the performance of various testing utilities
//! including database operations, serialization, template rendering,
//! and other critical paths.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use reinhardt_auth::{AllowAny, IsAuthenticated, JWT, Permission, PermissionContext};
use reinhardt_orm::{CascadeOption, LoadingStrategy, Relationship, RelationshipType};
use reinhardt_proxy::{AssociationProxy, CollectionProxy, ProxyTarget, ScalarProxy, ScalarValue};
use reinhardt_serializers::{CharField, EmailField, IntegerField, JsonSerializer, ModelSerializer};
use reinhardt_templates::TemplateLoader;
use reinhardt_test::{
	APIClient, APIRequestFactory, MockFunction, SettingsManager, Spy, TestCase, TestResponse,
	cleanup_fixture, generate_fixture_data, load_fixture,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::runtime::Runtime;

/// Test models for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkUser {
	id: Option<i64>,
	name: String,
	email: String,
	posts: Vec<BenchmarkPost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkPost {
	id: Option<i64>,
	user_id: i64,
	title: String,
	content: String,
	tags: Vec<String>,
}

impl reinhardt_orm::Model for BenchmarkUser {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"benchmark_users"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

impl reinhardt_orm::Model for BenchmarkPost {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"benchmark_posts"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

/// Benchmark API client operations
fn benchmark_api_client_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("api_client_create", |b| {
		b.iter(|| {
			black_box(APIClient::new("http://localhost:8000"));
		});
	});

	c.bench_function("api_client_authenticate", |b| {
		let client = APIClient::new("http://localhost:8000");
		b.iter(|| {
			rt.block_on(async {
				let _ = client.authenticate("test_token").await;
			});
		});
	});

	c.bench_function("api_client_get_request", |b| {
		let client = APIClient::new("http://localhost:8000");
		b.iter(|| {
			rt.block_on(async {
				let _ = client.get("/api/users").await;
			});
		});
	});

	c.bench_function("api_client_post_request", |b| {
		let client = APIClient::new("http://localhost:8000");
		let data = serde_json::json!({"name": "Test User", "email": "test@example.com"});
		b.iter(|| {
			rt.block_on(async {
				let _ = client.post("/api/users", &data).await;
			});
		});
	});
}

/// Benchmark request factory operations
fn benchmark_request_factory_operations(c: &mut Criterion) {
	c.bench_function("request_factory_create", |b| {
		b.iter(|| {
			black_box(APIRequestFactory::new());
		});
	});

	c.bench_function("request_factory_build_get", |b| {
		let factory = APIRequestFactory::new();
		b.iter(|| {
			black_box(factory.get("/api/users").build());
		});
	});

	c.bench_function("request_factory_build_post", |b| {
		let factory = APIRequestFactory::new();
		let data = serde_json::json!({"name": "Test User"});
		b.iter(|| {
			black_box(factory.post("/api/users", &data).build());
		});
	});

	c.bench_function("request_factory_with_headers", |b| {
		let factory = APIRequestFactory::new();
		b.iter(|| {
			black_box(
				factory
					.get("/api/users")
					.header("Authorization", "Bearer token")
					.header("Content-Type", "application/json")
					.build(),
			);
		});
	});
}

/// Benchmark mock and spy operations
fn benchmark_mock_spy_operations(c: &mut Criterion) {
	c.bench_function("mock_function_create", |b| {
		b.iter(|| {
			black_box(MockFunction::new(|_x: i32| x * 2));
		});
	});

	c.bench_function("mock_function_call", |b| {
		let mock = MockFunction::new(|x: i32| x * 2);
		b.iter(|| {
			black_box(mock.call(42));
		});
	});

	c.bench_function("spy_create", |b| {
		b.iter(|| {
			black_box(Spy::new());
		});
	});

	c.bench_function("spy_record_call", |b| {
		let spy = Spy::new();
		b.iter(|| {
			spy.record_call(vec![serde_json::Value::Number(42.into())]);
		});
	});

	c.bench_function("spy_get_calls", |b| {
		let spy = Spy::new();
		for i in 0..100 {
			spy.record_call(vec![serde_json::Value::Number(i.into())]);
		}
		b.iter(|| {
			black_box(spy.get_calls());
		});
	});
}

/// Benchmark test response operations
fn benchmark_test_response_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("test_response_create", |b| {
		b.iter(|| {
			black_box(TestResponse::new(200, "OK", "{}"));
		});
	});

	c.bench_function("test_response_assert_status", |b| {
		let response = TestResponse::new(200, "OK", "{}");
		b.iter(|| {
			black_box(response.assert_status(200));
		});
	});

	c.bench_function("test_response_assert_json", |b| {
		let response = TestResponse::new(200, "OK", r#"{"name": "Test User"}"#);
		b.iter(|| {
			rt.block_on(async {
				black_box(response.assert_json_field("name", "Test User").await);
			});
		});
	});

	c.bench_function("test_response_parse_json", |b| {
		let response = TestResponse::new(200, "OK", r#"{"name": "Test User", "age": 30}"#);
		b.iter(|| {
			rt.block_on(async {
				black_box(response.parse_json().await);
			});
		});
	});
}

/// Benchmark settings manager operations
fn benchmark_settings_manager_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("settings_manager_create", |b| {
		b.iter(|| {
			black_box(SettingsManager::new());
		});
	});

	c.bench_function("settings_manager_override", |b| {
		let mut manager = SettingsManager::new();
		b.iter(|| {
			rt.block_on(async {
				let _guard = manager.override_setting("TEST_SETTING", "test_value").await;
				black_box(());
			});
		});
	});

	c.bench_function("settings_manager_multiple_overrides", |b| {
		let mut manager = SettingsManager::new();
		b.iter(|| {
			rt.block_on(async {
				let _guard1 = manager.override_setting("SETTING1", "value1").await;
				let _guard2 = manager.override_setting("SETTING2", "value2").await;
				let _guard3 = manager.override_setting("SETTING3", "value3").await;
				black_box(());
			});
		});
	});
}

/// Benchmark fixture operations
fn benchmark_fixture_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("load_fixture_json", |b| {
		b.iter(|| {
			rt.block_on(async {
				// Create a temporary fixture file
				let temp_path = std::env::temp_dir().join("test_fixture.json");
				std::fs::write(&temp_path, r#"{"users": [{"id": 1, "name": "Test User"}]}"#)
					.unwrap();

				let result = load_fixture(&temp_path).await;
				std::fs::remove_file(&temp_path).unwrap();
				black_box(result);
			});
		});
	});

	c.bench_function("generate_fixture_data", |b| {
		b.iter(|| {
			rt.block_on(async {
				black_box(generate_fixture_data("users", 100).await);
			});
		});
	});

	c.bench_function("cleanup_fixture", |b| {
		b.iter(|| {
			rt.block_on(async {
				// Create a temporary fixture file
				let temp_path = std::env::temp_dir().join("test_fixture_cleanup.json");
				std::fs::write(&temp_path, r#"{"data": "test"}"#).unwrap();

				cleanup_fixture(&temp_path).await;
				black_box(());
			});
		});
	});
}

/// Benchmark proxy operations
fn benchmark_proxy_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("association_proxy_create", |b| {
		b.iter(|| {
			black_box(AssociationProxy::<(), ()>::new("posts", "title"));
		});
	});

	c.bench_function("collection_proxy_create", |b| {
		b.iter(|| {
			black_box(CollectionProxy::new("posts", "title"));
		});
	});

	c.bench_function("scalar_proxy_create", |b| {
		b.iter(|| {
			black_box(ScalarProxy::new("profile", "bio"));
		});
	});

	c.bench_function("scalar_value_conversion", |b| {
		b.iter(|| {
			let value = ScalarValue::String("test".to_string());
			black_box(value.as_string());
		});
	});

	c.bench_function("proxy_target_serialization", |b| {
		b.iter(|| {
			let target = ProxyTarget::Collection(vec![
				ScalarValue::String("test".to_string()),
				ScalarValue::Integer(42),
			]);
			black_box(serde_json::to_string(&target));
		});
	});
}

/// Benchmark ORM relationship operations
fn benchmark_orm_relationship_operations(c: &mut Criterion) {
	c.bench_function("relationship_create", |b| {
		b.iter(|| {
			black_box(
				Relationship::<BenchmarkUser, BenchmarkPost>::new(
					"posts",
					RelationshipType::OneToMany,
				)
				.with_foreign_key("user_id")
				.with_lazy(LoadingStrategy::Lazy),
			);
		});
	});

	c.bench_function("relationship_with_cascade", |b| {
		b.iter(|| {
			black_box(
				Relationship::<BenchmarkUser, BenchmarkPost>::new(
					"posts",
					RelationshipType::OneToMany,
				)
				.with_cascade("all, delete-orphan")
				.with_foreign_key("user_id"),
			);
		});
	});

	c.bench_function("relationship_sql_generation", |b| {
		let relationship =
			Relationship::<BenchmarkUser, BenchmarkPost>::new("posts", RelationshipType::OneToMany)
				.with_foreign_key("user_id")
				.with_lazy(LoadingStrategy::Lazy);

		b.iter(|| {
			black_box(relationship.load_sql("users.id"));
		});
	});

	c.bench_function("cascade_option_parsing", |b| {
		b.iter(|| {
			black_box(CascadeOption::parse("all, delete-orphan, save-update"));
		});
	});
}

/// Benchmark template operations
fn benchmark_template_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("template_loader_create", |b| {
		b.iter(|| {
			black_box(TemplateLoader::new());
		});
	});

	c.bench_function("template_register", |b| {
		let mut loader = TemplateLoader::new();
		b.iter(|| {
			loader.register("test_template", || "Hello {{ name }}!".to_string());
		});
	});

	c.bench_function("template_render", |b| {
		let mut loader = TemplateLoader::new();
		loader.register("test_template", || "Hello {{ name }}!".to_string());

		b.iter(|| {
			rt.block_on(async {
				black_box(loader.render("test_template").await);
			});
		});
	});
}

/// Benchmark authentication operations
fn benchmark_auth_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("jwt_create", |b| {
		b.iter(|| {
			black_box(JWT::new("secret_key"));
		});
	});

	c.bench_function("jwt_generate_token", |b| {
		let jwt = JWT::new("secret_key");
		b.iter(|| {
			rt.block_on(async {
				black_box(jwt.generate_token("user123", 3600).await);
			});
		});
	});

	c.bench_function("jwt_validate_token", |b| {
		let jwt = JWT::new("secret_key");
		let token = rt.block_on(async { jwt.generate_token("user123", 3600).await.unwrap() });

		b.iter(|| {
			rt.block_on(async {
				black_box(jwt.validate_token(&token).await);
			});
		});
	});

	c.bench_function("permission_check", |b| {
		let permission = IsAuthenticated;
		let context = PermissionContext {
			request: &reinhardt_apps::Request::new(),
			is_authenticated: true,
			is_admin: false,
			is_active: true,
		};

		b.iter(|| {
			rt.block_on(async {
				black_box(permission.has_permission(&context).await);
			});
		});
	});
}

/// Benchmark serialization operations
fn benchmark_serialization_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("json_serializer_create", |b| {
		b.iter(|| {
			black_box(JsonSerializer::new());
		});
	});

	c.bench_function("char_field_validation", |b| {
		let field = CharField::new().max_length(100);
		b.iter(|| {
			rt.block_on(async {
				black_box(field.validate("test_string").await);
			});
		});
	});

	c.bench_function("integer_field_validation", |b| {
		let field = IntegerField::new().min_value(0).max_value(100);
		b.iter(|| {
			rt.block_on(async {
				black_box(field.validate(42).await);
			});
		});
	});

	c.bench_function("email_field_validation", |b| {
		let field = EmailField::new();
		b.iter(|| {
			rt.block_on(async {
				black_box(field.validate("test@example.com").await);
			});
		});
	});

	c.bench_function("model_serializer_serialize", |b| {
		let serializer = ModelSerializer::<BenchmarkUser>::new();
		let user = BenchmarkUser {
			id: Some(1),
			name: "Test User".to_string(),
			email: "test@example.com".to_string(),
			posts: vec![],
		};

		b.iter(|| {
			rt.block_on(async {
				black_box(serializer.serialize(&user).await);
			});
		});
	});
}

/// Benchmark concurrent operations
fn benchmark_concurrent_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	c.bench_function("concurrent_api_requests", |b| {
		let client = APIClient::new("http://localhost:8000");
		b.iter(|| {
			rt.block_on(async {
				let handles: Vec<_> = (0..10)
					.map(|i| {
						let client = client.clone();
						tokio::spawn(async move { client.get(&format!("/api/users/{}", i)).await })
					})
					.collect();

				for handle in handles {
					let _ = handle.await;
				}
			});
		});
	});

	c.bench_function("concurrent_mock_calls", |b| {
		let mock = MockFunction::new(|x: i32| x * 2);
		b.iter(|| {
			rt.block_on(async {
				let handles: Vec<_> = (0..100)
					.map(|i| {
						let mock = mock.clone();
						tokio::spawn(async move { mock.call(i) })
					})
					.collect();

				for handle in handles {
					let _ = handle.await;
				}
			});
		});
	});
}

/// Benchmark memory usage
fn benchmark_memory_usage(c: &mut Criterion) {
	c.bench_function("large_fixture_generation", |b| {
		b.iter(|| {
			let rt = Runtime::new().unwrap();
			rt.block_on(async {
				black_box(generate_fixture_data("users", 10000).await);
			});
		});
	});

	c.bench_function("large_collection_proxy", |b| {
		b.iter(|| {
			let proxy =
				CollectionProxy::new("large_collection", "data").with_memory_limit(1024 * 1024); // 1MB limit
			black_box(proxy);
		});
	});
}

criterion_group!(
	benches,
	benchmark_api_client_operations,
	benchmark_request_factory_operations,
	benchmark_mock_spy_operations,
	benchmark_test_response_operations,
	benchmark_settings_manager_operations,
	benchmark_fixture_operations,
	benchmark_proxy_operations,
	benchmark_orm_relationship_operations,
	benchmark_template_operations,
	benchmark_auth_operations,
	benchmark_serialization_operations,
	benchmark_concurrent_operations,
	benchmark_memory_usage
);

criterion_main!(benches);
