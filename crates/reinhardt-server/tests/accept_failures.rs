//! A failed `accept` goes to the server's accept error handler instead of
//! ending the server. Running out of file descriptors is the real way to get
//! one, and it affects the whole process, so these tests have a binary to
//! themselves and run one at a time.
#![cfg(unix)]

use reinhardt_http::Handler;
use reinhardt_server::{
	AcceptErrorAction, AcceptErrorHandler, HttpServer, Request, Response, ShutdownCoordinator,
};
use rstest::rstest;
use serial_test::serial;
use std::{
	fs::File,
	io,
	net::SocketAddr,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::Duration,
};
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::{TcpListener, TcpSocket, TcpStream},
	task::JoinHandle,
};

/// Low enough that filling the table is quick, and enough to run a server.
const DESCRIPTORS: libc::rlim_t = 256;
/// Several retry delays, so accept fails more than once.
const OUT_OF_DESCRIPTORS: Duration = Duration::from_millis(500);
const RETRY_AFTER: Duration = Duration::from_millis(100);

struct Ok200;

#[async_trait::async_trait]
impl Handler for Ok200 {
	async fn handle(&self, _request: Request) -> reinhardt_http::Result<Response> {
		Ok(Response::ok().with_body("served"))
	}
}

/// Counts what the server reports and answers with a fixed action.
struct Recording {
	action: AcceptErrorAction,
	errors: AtomicUsize,
	out_of_descriptors: AtomicUsize,
	accepted: AtomicUsize,
}

impl Recording {
	fn new(action: AcceptErrorAction) -> Arc<Self> {
		Arc::new(Self {
			action,
			errors: AtomicUsize::new(0),
			out_of_descriptors: AtomicUsize::new(0),
			accepted: AtomicUsize::new(0),
		})
	}
}

impl AcceptErrorHandler for Recording {
	fn on_error(&self, error: &io::Error) -> AcceptErrorAction {
		self.errors.fetch_add(1, Ordering::SeqCst);
		if error.raw_os_error() == Some(libc::EMFILE) {
			self.out_of_descriptors.fetch_add(1, Ordering::SeqCst);
		}
		self.action
	}

	fn on_accepted(&self) {
		self.accepted.fetch_add(1, Ordering::SeqCst);
	}
}

fn lower_descriptor_limit() {
	let mut limit = libc::rlimit {
		rlim_cur: 0,
		rlim_max: 0,
	};
	// SAFETY: both calls only read or write the local struct passed to them.
	unsafe {
		assert_eq!(libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit), 0);
		limit.rlim_cur = DESCRIPTORS.min(limit.rlim_max);
		assert_eq!(libc::setrlimit(libc::RLIMIT_NOFILE, &limit), 0);
	}
}

/// Open files until the process may open no more, and hold them.
fn fill_descriptor_table() -> Vec<File> {
	let mut held = Vec::new();
	loop {
		match File::open("/dev/null") {
			Ok(file) => held.push(file),
			Err(error) if error.raw_os_error() == Some(libc::EMFILE) => return held,
			Err(error) => panic!("expected to run out of descriptors: {error}"),
		}
	}
}

async fn status_line(mut stream: TcpStream) -> String {
	stream
		.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
		.await
		.unwrap();
	let mut response = Vec::new();
	tokio::time::timeout(Duration::from_secs(10), stream.read_to_end(&mut response))
		.await
		.expect("the server should answer")
		.unwrap();
	let response = String::from_utf8_lossy(&response);
	response.lines().next().unwrap_or_default().to_owned()
}

/// Stops the server if a test ends before it does.
struct Serving {
	address: SocketAddr,
	coordinator: ShutdownCoordinator,
	task: Option<JoinHandle<Result<(), String>>>,
}

impl Serving {
	fn is_finished(&self) -> bool {
		self.task.as_ref().is_some_and(JoinHandle::is_finished)
	}

	/// Wait for the server to return.
	async fn finished(&mut self) -> Result<(), String> {
		self.task
			.take()
			.expect("the server task is owned")
			.await
			.unwrap()
	}
}

impl Drop for Serving {
	fn drop(&mut self) {
		self.coordinator.shutdown();
		if let Some(task) = &self.task {
			task.abort();
		}
	}
}

async fn serve(handler: Arc<Recording>) -> Serving {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let address = listener.local_addr().unwrap();
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(1));
	let server = HttpServer::new(Ok200).with_accept_error_handler(handler);
	let serving = coordinator.clone();
	let task = tokio::spawn(async move {
		server
			.listen_on_with_shutdown(listener, serving)
			.await
			.map_err(|error| error.to_string())
	});
	// An answer means the server has opened all it opens at startup.
	let first = TcpStream::connect(address).await.unwrap();
	assert_eq!(status_line(first).await, "HTTP/1.1 200 OK");
	Serving {
		address,
		coordinator,
		task: Some(task),
	}
}

#[rstest]
#[serial(descriptors)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retrying_outlives_running_out_of_descriptors() {
	// Arrange
	lower_descriptor_limit();
	let handler = Recording::new(AcceptErrorAction::RetryAfter(RETRY_AFTER));
	let mut serving = serve(Arc::clone(&handler)).await;
	// A socket made now can connect later without needing a new descriptor.
	let socket = TcpSocket::new_v4().unwrap();
	let held = fill_descriptor_table();

	// Act
	let waiting = socket.connect(serving.address).await.unwrap();
	tokio::time::sleep(OUT_OF_DESCRIPTORS).await;

	// Assert: it kept going, and answers again once descriptors are free.
	if serving.is_finished() {
		panic!(
			"the server stopped on a failed accept: {:?}",
			serving.finished().await
		);
	}
	drop(held);
	assert!(handler.out_of_descriptors.load(Ordering::SeqCst) >= 1);
	assert_eq!(
		handler.errors.load(Ordering::SeqCst),
		handler.out_of_descriptors.load(Ordering::SeqCst)
	);
	// A failed accept leaves the connection queued on Linux; macOS drops it.
	if cfg!(target_os = "linux") {
		assert_eq!(status_line(waiting).await, "HTTP/1.1 200 OK");
	}
	let accepted_before = handler.accepted.load(Ordering::SeqCst);
	let after = TcpStream::connect(serving.address).await.unwrap();
	assert_eq!(status_line(after).await, "HTTP/1.1 200 OK");
	assert!(handler.accepted.load(Ordering::SeqCst) > accepted_before);
	serving.coordinator.shutdown();
	assert_eq!(serving.finished().await, Ok(()));
}

#[rstest]
#[serial(descriptors)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stopping_returns_the_accept_error() {
	// Arrange
	lower_descriptor_limit();
	let handler = Recording::new(AcceptErrorAction::Stop);
	let mut serving = serve(Arc::clone(&handler)).await;
	let socket = TcpSocket::new_v4().unwrap();
	let held = fill_descriptor_table();

	// Act
	let _waiting = socket.connect(serving.address).await.unwrap();
	let result = tokio::time::timeout(Duration::from_secs(5), serving.finished())
		.await
		.expect("the server should stop on the first failed accept");
	drop(held);

	// Assert
	let error = result.expect_err("the server should return the accept error");
	assert!(
		error.contains("Too many open files"),
		"unexpected error: {error}"
	);
	assert_eq!(handler.errors.load(Ordering::SeqCst), 1);
	assert_eq!(handler.out_of_descriptors.load(Ordering::SeqCst), 1);
}
