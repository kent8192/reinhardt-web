//! runserver CLI command
//!
//! Starts the development server.

use clap::Parser;
use colored::Colorize;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use reinhardt_conf::Settings;
use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::profile::Profile;
use reinhardt_conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
use reinhardt_utils::r#static::handler::{StaticError, StaticFileHandler};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use serde_json::Value;
use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

#[derive(Parser, Debug)]
#[command(name = "runserver")]
#[command(about = "Starts the development server", long_about = None)]
struct Args {
	/// Server address (default: 127.0.0.1:8000)
	#[arg(value_name = "ADDRESS", default_value = "127.0.0.1:8000")]
	address: String,

	/// Disable auto-reload
	#[arg(long)]
	noreload: bool,

	/// Watch delay in milliseconds for file change debouncing (default: 500)
	#[arg(long, default_value = "500")]
	watch_delay: u64,

	/// Disable threading
	#[arg(long)]
	nothreading: bool,

	/// Serve static files in production mode
	#[arg(long)]
	insecure: bool,

	/// Path to TLS certificate file (enables HTTPS)
	#[arg(long, value_name = "FILE")]
	cert: Option<PathBuf>,

	/// Path to TLS private key file (required with --cert)
	#[arg(long, value_name = "FILE")]
	key: Option<PathBuf>,

	/// Generate and use a self-signed certificate for development (enables HTTPS)
	#[arg(long)]
	self_signed: bool,
}

/// Load settings from TOML configuration files
///
/// Uses the same pattern as collectstatic command to ensure consistency
/// across all management commands.
fn load_settings() -> Result<Settings, Box<dyn std::error::Error>> {
	let profile_str = std::env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
	let profile = Profile::parse(&profile_str);

	let base_dir = std::env::current_dir()?;
	let settings_dir = base_dir.join("settings");

	let merged = SettingsBuilder::new()
		.profile(profile)
		.add_source(
			DefaultSource::new()
				.with_value(
					"base_dir",
					Value::String(
						base_dir
							.to_str()
							.ok_or("base_dir contains invalid UTF-8")?
							.to_string(),
					),
				)
				.with_value("debug", Value::Bool(true))
				.with_value(
					"secret_key",
					Value::String("insecure-dev-key-change-in-production".to_string()),
				)
				.with_value("allowed_hosts", Value::Array(vec![]))
				.with_value("installed_apps", Value::Array(vec![]))
				.with_value("middleware", Value::Array(vec![]))
				.with_value("root_urlconf", Value::String("config.urls".to_string()))
				.with_value("databases", serde_json::json!({}))
				.with_value("templates", Value::Array(vec![]))
				.with_value("static_url", Value::String("/static/".to_string()))
				.with_value(
					"static_root",
					Value::String(base_dir.join("staticfiles").to_string_lossy().to_string()),
				)
				.with_value("staticfiles_dirs", Value::Array(vec![]))
				.with_value("media_url", Value::String("/media/".to_string()))
				.with_value("language_code", Value::String("en-us".to_string()))
				.with_value("time_zone", Value::String("UTC".to_string()))
				.with_value("use_i18n", Value::Bool(false))
				.with_value("use_tz", Value::Bool(false))
				.with_value(
					"default_auto_field",
					Value::String("reinhardt.db.models.BigAutoField".to_string()),
				)
				.with_value("secure_ssl_redirect", Value::Bool(false))
				.with_value("secure_hsts_include_subdomains", Value::Bool(false))
				.with_value("secure_hsts_preload", Value::Bool(false))
				.with_value("session_cookie_secure", Value::Bool(false))
				.with_value("csrf_cookie_secure", Value::Bool(false))
				.with_value("append_slash", Value::Bool(false))
				.with_value("admins", Value::Array(vec![]))
				.with_value("managers", Value::Array(vec![])),
		)
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.add_source(TomlFileSource::new(settings_dir.join("base.toml")))
		.add_source(TomlFileSource::new(
			settings_dir.join(format!("{}.toml", profile_str)),
		))
		.build()?;

	Ok(merged.into_typed::<Settings>()?)
}

/// Development static file server that wraps StaticFileHandler
///
/// This adapter provides hyper-compatible responses and supports
/// multiple static file directories (STATICFILES_DIRS).
struct DevStaticFileServer {
	handlers: Vec<(PathBuf, StaticFileHandler)>,
	static_url: String,
	debug: bool,
}

impl DevStaticFileServer {
	fn new(settings: &Settings) -> Self {
		let mut handlers = Vec::new();

		// Create a handler for each directory in staticfiles_dirs
		for dir in &settings.staticfiles_dirs {
			if dir.exists() {
				handlers.push((dir.clone(), StaticFileHandler::new(dir.clone())));
			}
		}

		// Also add static_root if it exists (for collected static files)
		if let Some(ref static_root) = settings.static_root
			&& static_root.exists()
		{
			handlers.push((
				static_root.clone(),
				StaticFileHandler::new(static_root.clone()),
			));
		}

		Self {
			handlers,
			static_url: settings.static_url.clone(),
			debug: settings.debug,
		}
	}

	/// Check for conflicting files across static directories
	///
	/// Returns a list of directories that contain the same file.
	/// This helps developers identify potential issues with their
	/// static file configuration.
	async fn check_conflicts(&self, relative_path: &str) -> Vec<PathBuf> {
		let mut found_in = Vec::new();

		for (dir, handler) in &self.handlers {
			if handler.resolve_path(relative_path).await.is_ok() {
				found_in.push(dir.clone());
			}
		}

		found_in
	}

	/// Serve a static file request
	///
	/// Searches through all configured static directories and returns
	/// the first matching file. Uses StaticFileHandler for proper
	/// path traversal prevention and MIME type detection.
	async fn serve(&self, path: &str) -> Result<Response<Full<Bytes>>, Infallible> {
		// Only serve in debug mode
		if !self.debug {
			return Ok(Response::builder()
				.status(StatusCode::NOT_FOUND)
				.header("Content-Type", "text/plain; charset=utf-8")
				.body(Full::new(Bytes::from(
					"Static files are only served in debug mode",
				)))
				.unwrap());
		}

		// Extract relative path from the request
		let relative_path = path
			.strip_prefix(&self.static_url)
			.unwrap_or(path)
			.trim_start_matches('/');

		if relative_path.is_empty() {
			return Ok(Response::builder()
				.status(StatusCode::NOT_FOUND)
				.header("Content-Type", "text/plain; charset=utf-8")
				.body(Full::new(Bytes::from("File not found")))
				.unwrap());
		}

		// Check for conflicts (multiple directories with same file)
		let conflicts = self.check_conflicts(relative_path).await;
		if conflicts.len() > 1 {
			eprintln!(
				"{}",
				format!(
					"Warning: Static file '{}' found in multiple directories: {:?}",
					relative_path, conflicts
				)
				.yellow()
			);
		}

		// Try to serve from each handler
		for (_, handler) in &self.handlers {
			match handler.serve(relative_path).await {
				Ok(static_file) => {
					let etag = static_file.etag();
					return Ok(Response::builder()
						.status(StatusCode::OK)
						.header("Content-Type", &static_file.mime_type)
						.header("ETag", &etag)
						.header("Cache-Control", "no-cache")
						.body(Full::new(Bytes::from(static_file.content)))
						.unwrap());
				}
				Err(StaticError::DirectoryTraversal(path)) => {
					eprintln!(
						"{}",
						format!("Blocked directory traversal attempt: {}", path).red()
					);
					return Ok(Response::builder()
						.status(StatusCode::FORBIDDEN)
						.header("Content-Type", "text/plain; charset=utf-8")
						.body(Full::new(Bytes::from("Forbidden")))
						.unwrap());
				}
				Err(StaticError::NotFound(_)) => {
					// Continue to next handler
					continue;
				}
				Err(StaticError::Io(e)) => {
					eprintln!("{}", format!("IO error serving static file: {}", e).red());
					return Ok(Response::builder()
						.status(StatusCode::INTERNAL_SERVER_ERROR)
						.header("Content-Type", "text/plain; charset=utf-8")
						.body(Full::new(Bytes::from("Internal server error")))
						.unwrap());
				}
			}
		}

		// File not found in any directory
		Ok(Response::builder()
			.status(StatusCode::NOT_FOUND)
			.header("Content-Type", "text/plain; charset=utf-8")
			.body(Full::new(Bytes::from("File not found")))
			.unwrap())
	}
}

/// Render the welcome page
fn serve_welcome_page() -> Response<Full<Bytes>> {
	let template_str = include_str!("../../templates/welcome.tpl");

	let mut tera = Tera::default();
	tera.add_raw_template("welcome.tpl", template_str)
		.unwrap_or_else(|e| {
			eprintln!("Error adding template: {}", e);
		});

	let mut context = Context::new();
	context.insert("version", env!("CARGO_PKG_VERSION"));

	let html = tera.render("welcome.tpl", &context).unwrap_or_else(|e| {
		format!(
			"<html><body><h1>Error rendering template: {}</h1></body></html>",
			e
		)
	});

	Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", "text/html; charset=utf-8")
		.body(Full::new(Bytes::from(html)))
		.unwrap()
}

async fn handle_request(
	req: Request<Incoming>,
	static_server: Arc<DevStaticFileServer>,
	settings: Arc<Settings>,
) -> Result<Response<Full<Bytes>>, Infallible> {
	let path = req.uri().path();

	// In debug mode, serve static files
	if settings.debug && path.starts_with(&settings.static_url) {
		let relative_path = path
			.strip_prefix(&settings.static_url)
			.unwrap_or(path)
			.trim_start_matches('/');

		if !relative_path.is_empty() {
			return static_server.serve(path).await;
		}
	}

	Ok(serve_welcome_page())
}

/// Load TLS configuration from certificate and key files
fn load_tls_config(
	cert_path: &PathBuf,
	key_path: &PathBuf,
) -> Result<ServerConfig, Box<dyn std::error::Error>> {
	// Load certificate chain
	let cert_file = File::open(cert_path)?;
	let mut cert_reader = BufReader::new(cert_file);
	let cert_chain: Vec<_> = certs(&mut cert_reader).collect::<Result<_, _>>()?;

	// Load private key
	let key_file = File::open(key_path)?;
	let mut key_reader = BufReader::new(key_file);
	let private_key = private_key(&mut key_reader)?.ok_or("No private key found in key file")?;

	// Build TLS configuration
	let config = ServerConfig::builder()
		.with_no_client_auth()
		.with_single_cert(cert_chain, private_key)?;

	Ok(config)
}

/// Generate a self-signed certificate for development
fn generate_self_signed_cert() -> Result<
	(
		Vec<rustls::pki_types::CertificateDer<'static>>,
		rustls::pki_types::PrivateKeyDer<'static>,
	),
	Box<dyn std::error::Error>,
> {
	use rcgen::{CertificateParams, DistinguishedName, KeyPair};

	let mut params = CertificateParams::new(vec!["localhost".to_string()])?;
	let mut distinguished_name = DistinguishedName::new();
	distinguished_name.push(rcgen::DnType::CommonName, "Reinhardt Development Server");
	params.distinguished_name = distinguished_name;

	let key_pair = KeyPair::generate()?;
	let cert = params.self_signed(&key_pair)?;
	let cert_der = cert.der().to_vec();
	let key_der = key_pair.serialize_der();

	Ok((
		vec![rustls::pki_types::CertificateDer::from(cert_der)],
		rustls::pki_types::PrivateKeyDer::try_from(key_der)?,
	))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	// Validate TLS arguments
	if args.cert.is_some() && args.key.is_none() {
		return Err("--key is required when --cert is specified".into());
	}
	if args.key.is_some() && args.cert.is_none() {
		return Err("--cert is required when --key is specified".into());
	}
	if args.cert.is_some() && args.self_signed {
		return Err("Cannot use both --cert/--key and --self-signed".into());
	}

	// Load settings from TOML configuration
	let settings = Arc::new(load_settings()?);

	// Create static file server
	let static_server = Arc::new(DevStaticFileServer::new(&settings));

	// Parse the address
	let addr: SocketAddr = args
		.address
		.parse()
		.map_err(|_| format!("Invalid address: {}", args.address))?;

	// Determine if HTTPS is enabled
	let use_https = args.cert.is_some() || args.self_signed;
	let scheme = if use_https { "https" } else { "http" };

	println!(
		"{}",
		format!("Starting development server at {}://{}", scheme, addr)
			.cyan()
			.bold()
	);

	if settings.debug {
		println!(
			"{}",
			format!(
				"Static files URL: {} (serving from {} directories)",
				settings.static_url,
				static_server.handlers.len()
			)
			.cyan()
		);
	}

	if !args.noreload {
		println!("{}", "Auto-reload enabled".green());
	}

	if args.insecure {
		println!(
			"{}",
			"Running with --insecure: Static files will be served".yellow()
		);
	}

	// Load or generate TLS configuration if needed
	let tls_acceptor = if use_https {
		let tls_config = if args.self_signed {
			println!(
				"{}",
				"Using self-signed certificate for development".yellow()
			);
			let (certs, key) = generate_self_signed_cert()?;
			Arc::new(
				ServerConfig::builder()
					.with_no_client_auth()
					.with_single_cert(certs, key)?,
			)
		} else {
			let cert_path = args.cert.as_ref().unwrap();
			let key_path = args.key.as_ref().unwrap();
			println!(
				"{}",
				format!(
					"Loading TLS certificate from {:?} and key from {:?}",
					cert_path, key_path
				)
				.cyan()
			);
			Arc::new(load_tls_config(cert_path, key_path)?)
		};
		Some(TlsAcceptor::from(tls_config))
	} else {
		None
	};

	println!("{}", "Quit the server with CTRL-C".dimmed());
	println!();

	// Create TCP listener
	let listener = TcpListener::bind(addr).await?;

	println!("{}", format!("Listening on {}", addr).green().bold());

	// Accept connections in a loop
	loop {
		let (stream, _) = listener.accept().await?;
		let static_server = Arc::clone(&static_server);
		let settings = Arc::clone(&settings);

		if let Some(ref acceptor) = tls_acceptor {
			// HTTPS connection
			let acceptor = acceptor.clone();
			tokio::task::spawn(async move {
				match acceptor.accept(stream).await {
					Ok(tls_stream) => {
						let io = TokioIo::new(tls_stream);
						if let Err(err) = http1::Builder::new()
							.serve_connection(
								io,
								service_fn(move |req| {
									let ss = Arc::clone(&static_server);
									let s = Arc::clone(&settings);
									async move { handle_request(req, ss, s).await }
								}),
							)
							.await
						{
							eprintln!("Error serving HTTPS connection: {:?}", err);
						}
					}
					Err(err) => {
						eprintln!("TLS handshake error: {:?}", err);
					}
				}
			});
		} else {
			// HTTP connection
			let io = TokioIo::new(stream);
			tokio::task::spawn(async move {
				if let Err(err) = http1::Builder::new()
					.serve_connection(
						io,
						service_fn(move |req| {
							let ss = Arc::clone(&static_server);
							let s = Arc::clone(&settings);
							async move { handle_request(req, ss, s).await }
						}),
					)
					.await
				{
					eprintln!("Error serving HTTP connection: {:?}", err);
				}
			});
		}
	}
}
