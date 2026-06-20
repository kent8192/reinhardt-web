//! Azure Blob Storage backend implementation.

#![allow(deprecated)] // Backend constructor keeps accepting legacy config during compatibility.

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Response, StatusCode};
use sha2::Sha256;
use std::collections::BTreeMap;

use crate::config::AzureConfig;
use crate::{Result, StorageBackend, StorageError};

type HmacSha256 = Hmac<Sha256>;

const AZURE_VERSION: &str = "2023-11-03";

/// Azure Blob Storage backend.
#[derive(Debug, Clone)]
pub struct AzureStorage {
	config: AzureConfig,
	http: reqwest::Client,
}

impl AzureStorage {
	/// Create a new Azure storage backend.
	pub async fn new(config: AzureConfig) -> Result<Self> {
		let storage = Self {
			config,
			http: reqwest::Client::new(),
		};
		storage.ensure_container().await?;
		Ok(storage)
	}

	fn blob_name(&self, name: &str) -> String {
		if let Some(prefix) = &self.config.prefix {
			if prefix.is_empty() {
				name.to_string()
			} else {
				format!("{}/{}", prefix.trim_end_matches('/'), name)
			}
		} else {
			name.to_string()
		}
	}

	fn endpoint(&self) -> String {
		self.config.endpoint.clone().unwrap_or_else(|| {
			format!(
				"https://{}.blob.core.windows.net",
				self.config.account.trim()
			)
		})
	}

	fn container_url(&self) -> String {
		format!(
			"{}/{}",
			self.endpoint().trim_end_matches('/'),
			self.config.container
		)
	}

	fn blob_url(&self, blob: &str) -> String {
		format!(
			"{}/{}",
			self.container_url(),
			utf8_percent_encode(blob, NON_ALPHANUMERIC)
		)
	}

	fn append_sas(&self, url: String) -> String {
		if self.config.access_key.is_some() {
			return url;
		}
		if let Some(sas) = &self.config.sas_token {
			let token = sas.expose_secret().trim_start_matches('?');
			if url.contains('?') {
				format!("{url}&{token}")
			} else {
				format!("{url}?{token}")
			}
		} else {
			url
		}
	}

	fn request_date() -> String {
		Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
	}

	fn access_key(&self) -> Result<&str> {
		self.config
			.access_key
			.as_ref()
			.map(|key| key.expose_secret())
			.ok_or_else(|| {
				StorageError::ConfigError(
					"Azure access_key or sas_token is required for blob operations".to_string(),
				)
			})
	}

	fn canonicalized_headers(headers: &HeaderMap) -> String {
		let mut values = BTreeMap::new();
		for (name, value) in headers {
			let name = name.as_str().to_ascii_lowercase();
			if name.starts_with("x-ms-") {
				let value = value.to_str().unwrap_or_default();
				let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
				values.insert(name, value);
			}
		}
		values
			.into_iter()
			.map(|(name, value)| format!("{name}:{value}\n"))
			.collect::<String>()
	}

	fn canonicalized_resource(&self, url: &str) -> Result<String> {
		let parsed =
			reqwest::Url::parse(url).map_err(|err| StorageError::ConfigError(err.to_string()))?;
		let mut resource = format!("/{}{}", self.config.account, parsed.path());

		let mut query: BTreeMap<String, Vec<String>> = BTreeMap::new();
		for (key, value) in parsed.query_pairs() {
			query
				.entry(key.to_ascii_lowercase())
				.or_default()
				.push(value.to_string());
		}
		for (key, mut values) in query {
			values.sort();
			resource.push('\n');
			resource.push_str(&key);
			resource.push(':');
			resource.push_str(&values.join(","));
		}
		Ok(resource)
	}

	fn sign_request(
		&self,
		method: &Method,
		url: &str,
		headers: &HeaderMap,
		content_length: Option<usize>,
		content_type: Option<&str>,
	) -> Result<String> {
		let content_length = match content_length {
			Some(0) | None => String::new(),
			Some(length) => length.to_string(),
		};
		let content_type = content_type.unwrap_or_default();
		let string_to_sign = [
			method.as_str().to_string(),
			String::new(),
			String::new(),
			content_length,
			String::new(),
			content_type.to_string(),
			String::new(),
			String::new(),
			String::new(),
			String::new(),
			String::new(),
			String::new(),
			format!(
				"{}{}",
				Self::canonicalized_headers(headers),
				self.canonicalized_resource(url)?
			),
		]
		.join("\n");

		let key = STANDARD
			.decode(self.access_key()?)
			.map_err(|err| StorageError::ConfigError(err.to_string()))?;
		let mut mac = HmacSha256::new_from_slice(&key)
			.map_err(|err| StorageError::ConfigError(err.to_string()))?;
		mac.update(string_to_sign.as_bytes());
		Ok(STANDARD.encode(mac.finalize().into_bytes()))
	}

	async fn send(
		&self,
		method: Method,
		url: String,
		body: Option<Vec<u8>>,
		content_type: Option<&str>,
		blob_request: bool,
	) -> Result<Response> {
		let mut headers = HeaderMap::new();
		headers.insert(
			HeaderName::from_static("x-ms-date"),
			HeaderValue::from_str(&Self::request_date())
				.map_err(|err| StorageError::ConfigError(err.to_string()))?,
		);
		headers.insert(
			HeaderName::from_static("x-ms-version"),
			HeaderValue::from_static(AZURE_VERSION),
		);
		if blob_request {
			headers.insert(
				HeaderName::from_static("x-ms-blob-type"),
				HeaderValue::from_static("BlockBlob"),
			);
		}

		let content_length = body.as_ref().map(Vec::len);
		let url = self.append_sas(url);
		let mut request = self
			.http
			.request(method.clone(), &url)
			.headers(headers.clone());
		if let Some(content_type) = content_type {
			request = request.header("content-type", content_type);
		}
		if let Some(length) = content_length {
			request = request.header("content-length", length);
		}
		if self.config.access_key.is_some() {
			let signature =
				self.sign_request(&method, &url, &headers, content_length, content_type)?;
			request = request.header(
				"authorization",
				format!("SharedKey {}:{signature}", self.config.account),
			);
		} else if self.config.sas_token.is_none() {
			return Err(StorageError::ConfigError(
				"Azure access_key or sas_token is required for blob operations".to_string(),
			));
		}
		if let Some(body) = body {
			request = request.body(body);
		}
		request
			.send()
			.await
			.map_err(|err| StorageError::NetworkError(err.to_string()))
	}

	async fn ensure_container(&self) -> Result<()> {
		let url = format!("{}?restype=container", self.container_url());
		let response = self.send(Method::PUT, url, None, None, false).await?;
		let status = response.status();
		if status.is_success() || status == StatusCode::CONFLICT {
			Ok(())
		} else {
			Err(Self::map_status(status, &self.config.container))
		}
	}

	fn map_status(status: StatusCode, name: &str) -> StorageError {
		if status == StatusCode::NOT_FOUND {
			StorageError::NotFound(format!("Azure blob not found: {name}"))
		} else if status == StatusCode::FORBIDDEN {
			StorageError::PermissionDenied(format!("Azure permission denied for blob: {name}"))
		} else if status.is_server_error() {
			StorageError::NetworkError(format!("Azure service error {status} for blob: {name}"))
		} else {
			StorageError::Other(format!(
				"Azure request failed with status {status} for blob: {name}"
			))
		}
	}

	fn sas_url(&self, blob: &str, expiry_secs: u64) -> Result<String> {
		let expiry = Utc::now() + Duration::seconds(i64::try_from(expiry_secs).unwrap_or(i64::MAX));
		let se = expiry.format("%Y-%m-%dT%H:%M:%SZ").to_string();
		let sp = "r";
		let sv = AZURE_VERSION;
		let sr = "b";
		let canonicalized_resource = format!(
			"/blob/{}/{}/{}",
			self.config.account, self.config.container, blob
		);
		let string_to_sign = [
			sp,
			"",
			&se,
			&canonicalized_resource,
			"",
			"",
			"",
			sv,
			sr,
			"",
			"",
			"",
			"",
			"",
			"",
			"",
		]
		.join("\n");
		let access_key = self.config.access_key.as_ref().ok_or_else(|| {
			StorageError::ConfigError(
				"Azure access_key is required to generate temporary URLs".to_string(),
			)
		})?;
		let key = STANDARD
			.decode(access_key.expose_secret())
			.map_err(|err| StorageError::ConfigError(err.to_string()))?;
		let mut mac = HmacSha256::new_from_slice(&key)
			.map_err(|err| StorageError::ConfigError(err.to_string()))?;
		mac.update(string_to_sign.as_bytes());
		let sig = STANDARD.encode(mac.finalize().into_bytes());
		Ok(format!(
			"{}?sv={}&se={}&sr={}&sp={}&sig={}",
			self.blob_url(blob),
			sv,
			utf8_percent_encode(&se, NON_ALPHANUMERIC),
			sr,
			sp,
			utf8_percent_encode(&sig, NON_ALPHANUMERIC)
		))
	}
}

#[async_trait]
impl StorageBackend for AzureStorage {
	async fn save(&self, name: &str, content: &[u8]) -> Result<String> {
		let blob = self.blob_name(name);
		let response = self
			.send(
				Method::PUT,
				self.blob_url(&blob),
				Some(content.to_vec()),
				Some("application/octet-stream"),
				true,
			)
			.await?;
		let status = response.status();
		if !status.is_success() {
			return Err(Self::map_status(status, &blob));
		}
		Ok(blob)
	}

	async fn open(&self, name: &str) -> Result<Vec<u8>> {
		let blob = self.blob_name(name);
		let response = self
			.send(Method::GET, self.blob_url(&blob), None, None, false)
			.await?;
		let status = response.status();
		if !status.is_success() {
			return Err(Self::map_status(status, &blob));
		}
		response
			.bytes()
			.await
			.map(|bytes| bytes.to_vec())
			.map_err(|err| StorageError::NetworkError(err.to_string()))
	}

	async fn delete(&self, name: &str) -> Result<()> {
		let blob = self.blob_name(name);
		if !self.exists(name).await? {
			return Err(StorageError::NotFound(format!(
				"Azure blob not found: {blob}"
			)));
		}
		let response = self
			.send(Method::DELETE, self.blob_url(&blob), None, None, false)
			.await?;
		let status = response.status();
		if !status.is_success() {
			return Err(Self::map_status(status, &blob));
		}
		Ok(())
	}

	async fn exists(&self, name: &str) -> Result<bool> {
		let blob = self.blob_name(name);
		let response = self
			.send(Method::HEAD, self.blob_url(&blob), None, None, false)
			.await?;
		let status = response.status();
		if status.is_success() {
			Ok(true)
		} else if status == StatusCode::NOT_FOUND {
			Ok(false)
		} else {
			Err(Self::map_status(status, &blob))
		}
	}

	async fn url(&self, name: &str, expiry_secs: u64) -> Result<String> {
		let blob = self.blob_name(name);
		if !self.exists(name).await? {
			return Err(StorageError::NotFound(format!(
				"Azure blob not found: {blob}"
			)));
		}
		self.sas_url(&blob, expiry_secs)
	}

	async fn size(&self, name: &str) -> Result<u64> {
		let blob = self.blob_name(name);
		let response = self
			.send(Method::HEAD, self.blob_url(&blob), None, None, false)
			.await?;
		let status = response.status();
		if !status.is_success() {
			return Err(Self::map_status(status, &blob));
		}
		response
			.headers()
			.get("content-length")
			.and_then(|value| value.to_str().ok())
			.ok_or_else(|| {
				StorageError::Other(
					"Azure blob metadata did not include content-length".to_string(),
				)
			})?
			.parse::<u64>()
			.map_err(|err| StorageError::Other(err.to_string()))
	}

	async fn get_modified_time(&self, name: &str) -> Result<DateTime<Utc>> {
		let blob = self.blob_name(name);
		let response = self
			.send(Method::HEAD, self.blob_url(&blob), None, None, false)
			.await?;
		let status = response.status();
		if !status.is_success() {
			return Err(Self::map_status(status, &blob));
		}
		let last_modified = response
			.headers()
			.get("last-modified")
			.and_then(|value| value.to_str().ok())
			.ok_or_else(|| {
				StorageError::Other("Azure blob metadata did not include last-modified".to_string())
			})?;
		DateTime::parse_from_rfc2822(last_modified)
			.map(|time| time.with_timezone(&Utc))
			.map_err(|err| StorageError::Other(err.to_string()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sas_url_requires_access_key_instead_of_exposing_configured_sas_token() {
		let storage = AzureStorage {
			config: AzureConfig {
				account: "testaccount".to_string(),
				container: "testcontainer".to_string(),
				prefix: None,
				endpoint: Some("https://example.test/testaccount".to_string()),
				access_key: None,
				sas_token: Some("sp=rwdlac&sig=SECRET_CONTAINER_SIGNATURE".into()),
				connection_string: None,
			},
			http: reqwest::Client::new(),
		};

		let err = storage
			.sas_url("private.txt", 60)
			.expect_err("temporary URLs must not expose configured SAS credentials");

		assert!(
			matches!(err, StorageError::ConfigError(message) if message == "Azure access_key is required to generate temporary URLs")
		);
	}
}
