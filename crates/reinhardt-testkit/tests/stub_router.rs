use hyper::Method;
use reinhardt_testkit::{http::Response, stub::StubRouter};
use rstest::rstest;

#[rstest]
#[should_panic(expected = "duplicate stub route: POST /webhook")]
fn duplicate_path_and_method_panics_at_registration() {
	// Arrange
	let router = StubRouter::new().post("/webhook", |_request| async { Ok(Response::ok()) });

	// Act
	// Assert: registration must panic before any server is started.
	let _router = router.route("/webhook", Method::POST, |_request| async {
		Ok(Response::ok())
	});
}
