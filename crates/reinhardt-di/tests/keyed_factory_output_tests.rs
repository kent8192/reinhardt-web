#![cfg(feature = "testing")]

use reinhardt_di::{
	DependencyScope, Depends, FactoryOutput, InjectableKey, InjectionContext, SingletonScope,
	global_registry,
};
use serial_test::serial;
use std::sync::Arc;

struct PrimaryDb;

impl InjectableKey for PrimaryDb {}

#[derive(Clone, Debug, PartialEq)]
struct DatabaseConnection {
	url: String,
}

#[serial(di_registry)]
#[tokio::test]
async fn depends_resolves_factory_output_by_key_and_value_type() {
	let registry = global_registry();
	let _guard = registry.register_override::<FactoryOutput<PrimaryDb, DatabaseConnection>, _, _>(
		DependencyScope::Transient,
		|_ctx| async {
			Ok(FactoryOutput::new(DatabaseConnection {
				url: "postgres://primary".to_string(),
			}))
		},
	);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	let db = Depends::<PrimaryDb, DatabaseConnection>::resolve_from_registry(&ctx, true)
		.await
		.expect("keyed dependency must resolve from FactoryOutput");

	assert_eq!(db.url, "postgres://primary");
	assert_eq!(
		db.as_output().as_ref(),
		&DatabaseConnection {
			url: "postgres://primary".to_string(),
		}
	);
}

struct ReplicaDb;

impl InjectableKey for ReplicaDb {}

#[serial(di_registry)]
#[tokio::test]
async fn same_value_type_can_be_registered_under_different_keys() {
	let registry = global_registry();
	let _primary = registry
		.register_override::<FactoryOutput<PrimaryDb, DatabaseConnection>, _, _>(
			DependencyScope::Transient,
			|_ctx| async {
				Ok(FactoryOutput::new(DatabaseConnection {
					url: "postgres://primary".to_string(),
				}))
			},
		);
	let _replica = registry
		.register_override::<FactoryOutput<ReplicaDb, DatabaseConnection>, _, _>(
			DependencyScope::Transient,
			|_ctx| async {
				Ok(FactoryOutput::new(DatabaseConnection {
					url: "postgres://replica".to_string(),
				}))
			},
		);
	let scope = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(scope).build();

	let primary = Depends::<PrimaryDb, DatabaseConnection>::resolve_from_registry(&ctx, true)
		.await
		.expect("primary key must resolve");
	let replica = Depends::<ReplicaDb, DatabaseConnection>::resolve_from_registry(&ctx, true)
		.await
		.expect("replica key must resolve");

	assert_eq!(primary.url, "postgres://primary");
	assert_eq!(replica.url, "postgres://replica");
}
