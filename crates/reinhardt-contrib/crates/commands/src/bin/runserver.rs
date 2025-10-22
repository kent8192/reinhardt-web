//! runserver CLI command
//!
//! Starts the development server.

use askama::Template;
use clap::Parser;
use colored::Colorize;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeTemplate {
    version: &'static str,
}

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

async fn handle_request(_req: Request<Incoming>) -> Result<Response<String>, Infallible> {
    // Render the welcome page template
    let template = WelcomeTemplate {
        version: env!("CARGO_PKG_VERSION"),
    };
    let html = template.render().unwrap_or_else(|e| {
        format!(
            "<html><body><h1>Error rendering template: {}</h1></body></html>",
            e
        )
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
        .unwrap())
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

        if let Some(ref acceptor) = tls_acceptor {
            // HTTPS connection
            let acceptor = acceptor.clone();
            tokio::task::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let io = TokioIo::new(tls_stream);
                        if let Err(err) = http1::Builder::new()
                            .serve_connection(io, service_fn(handle_request))
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
                    .serve_connection(io, service_fn(handle_request))
                    .await
                {
                    eprintln!("Error serving HTTP connection: {:?}", err);
                }
            });
        }
    }
}
