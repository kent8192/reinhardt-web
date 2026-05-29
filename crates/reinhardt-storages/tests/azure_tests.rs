//! Integration tests for Azure Blob Storage using Azurite.

mod fixtures;
mod utils;

use fixtures::azure_fixture;
use reinhardt_storages::StorageError;
use serial_test::serial;
use utils::{assert_azure_signed_url, assert_file_size, assert_storage_not_exists};

#[tokio::test]
#[serial(azure)]
async fn azure_save_open_delete_roundtrip() {
	let fixture = azure_fixture().await;
	let name = "path/to/file.bin";
	let content = vec![0, 1, 2, 3, 254, 255];

	assert_eq!(fixture.backend.save(name, &content).await.unwrap(), name);
	assert!(fixture.backend.exists(name).await.unwrap());
	assert_eq!(fixture.backend.open(name).await.unwrap(), content);
	assert_file_size(&*fixture.backend, name, 6).await.unwrap();

	let modified = fixture.backend.get_modified_time(name).await.unwrap();
	assert!(modified.timestamp() > 0);

	fixture.backend.delete(name).await.unwrap();
	assert_storage_not_exists(&*fixture.backend, name)
		.await
		.unwrap();
}

#[tokio::test]
#[serial(azure)]
async fn azure_overwrites_and_handles_empty_files() {
	let fixture = azure_fixture().await;
	let name = "empty.txt";

	fixture.backend.save(name, b"first").await.unwrap();
	fixture.backend.save(name, b"").await.unwrap();

	assert_eq!(fixture.backend.open(name).await.unwrap(), Vec::<u8>::new());
	assert_eq!(fixture.backend.size(name).await.unwrap(), 0);

	// Cleanup
	fixture.backend.delete(name).await.unwrap();
}

#[tokio::test]
#[serial(azure)]
async fn azure_missing_object_errors_are_not_found() {
	let fixture = azure_fixture().await;
	let name = "missing.txt";

	assert!(!fixture.backend.exists(name).await.unwrap());
	assert!(matches!(
		fixture.backend.open(name).await,
		Err(StorageError::NotFound(_))
	));
	assert!(matches!(
		fixture.backend.delete(name).await,
		Err(StorageError::NotFound(_))
	));
	assert!(matches!(
		fixture.backend.url(name, 60).await,
		Err(StorageError::NotFound(_))
	));
	assert!(matches!(
		fixture.backend.size(name).await,
		Err(StorageError::NotFound(_))
	));
	assert!(matches!(
		fixture.backend.get_modified_time(name).await,
		Err(StorageError::NotFound(_))
	));
}

#[tokio::test]
#[serial(azure)]
async fn azure_generates_sas_url_shape() {
	let fixture = azure_fixture().await;
	let name = "signed.txt";
	fixture.backend.save(name, b"signed content").await.unwrap();

	let url = fixture.backend.url(name, 300).await.unwrap();

	assert_azure_signed_url(&url).unwrap();
	let response = reqwest::get(url).await.unwrap();
	assert!(
		response.status().is_success(),
		"Azurite accepts the generated SAS URL: {}",
		response.status()
	);
	assert_eq!(response.bytes().await.unwrap().as_ref(), b"signed content");

	// Cleanup
	fixture.backend.delete(name).await.unwrap();
}
