//! Native coverage for generated content-type persistence arguments.
#![cfg(all(feature = "database", feature = "contenttypes", feature = "postgres"))]

use reinhardt_db::contenttypes::ContentType;
use reinhardt_db::contenttypes::persistence::{
	ContentTypePersistence, ContentTypePersistenceBackend,
};
use rstest::rstest;
use testcontainers::runners::AsyncRunner;

#[rstest]
#[tokio::test]
async fn postgres_content_types_preserve_generated_crud_bindings() {
	// Arrange: the container owns the database through every return and panic path.
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	sqlx::any::install_default_drivers();
	let persistence = ContentTypePersistence::new(&url).await.unwrap();
	persistence.create_table().await.unwrap();
	let app = "quoted' ? $1 app";
	let model = "quoted' ? $2 model";

	// Act / Assert: RETURNING and every lookup consume the renderer's own arguments.
	let mut saved = persistence
		.save(&ContentType::new(app, model))
		.await
		.unwrap();
	let id = saved.id.unwrap();
	assert!(id > 0);
	assert_eq!(
		persistence.get(app, model).await.unwrap(),
		Some(saved.clone())
	);
	assert_eq!(
		persistence.get_by_id(id).await.unwrap(),
		Some(saved.clone())
	);
	assert!(persistence.exists(app, model).await.unwrap());
	assert_eq!(persistence.load_all().await.unwrap(), vec![saved.clone()]);
	assert_eq!(persistence.get_or_create(app, model).await.unwrap(), saved);
	saved.model = "updated' ? $3 model".to_owned();
	assert_eq!(persistence.save(&saved).await.unwrap(), saved);
	assert_eq!(persistence.get_by_id(id).await.unwrap(), Some(saved));
	persistence.delete(id).await.unwrap();
	assert_eq!(persistence.get_by_id(id).await.unwrap(), None);
	assert!(!persistence.exists(app, model).await.unwrap());
}
