//! Polls Component WASM Tests with Mocking
//!
//! Layer 2 tests for polls components that interact with server functions.
//! These tests verify that actual polls components from
//! `src/apps/polls/client/components.rs` render correctly and can interact
//! with mocked server function responses.
//!
//! **Test Categories:**
//! - Pure rendering tests (no server_fn interaction)
//! - Structure validation tests (list elements, form elements)
//! - Mock infrastructure integration tests
//!
//! **Run with**: `cargo make wasm-test`

#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Import actual components from the application
use examples_tutorial_basis::apps::polls::client::components::{
	polls_detail, polls_index, polls_results,
};
use examples_tutorial_basis::apps::polls::server_fn::{
	get_question_detail, get_question_results, get_questions, vote,
};
use examples_tutorial_basis::apps::users::server_fn::current_user;
use examples_tutorial_basis::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};
use gloo_timers::future::TimeoutFuture;
use reinhardt::pages::component::{Page, PageExt};
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::pages::{Element as DomElement, document};
use reinhardt::test::msw::MockServiceWorker;
use wasm_bindgen::JsCast;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create a mock QuestionInfo for testing
fn mock_question() -> QuestionInfo {
	QuestionInfo {
		id: 1,
		question_text: "What is your favorite programming language?".to_string(),
		pub_date: chrono::Utc::now(),
		author_id: 1,
	}
}

/// Create a mock list of QuestionInfo for testing
fn mock_questions_list() -> Vec<QuestionInfo> {
	vec![
		QuestionInfo {
			id: 1,
			question_text: "What is your favorite programming language?".to_string(),
			pub_date: chrono::Utc::now(),
			author_id: 1,
		},
		QuestionInfo {
			id: 2,
			question_text: "Which web framework do you prefer?".to_string(),
			pub_date: chrono::Utc::now(),
			author_id: 1,
		},
		QuestionInfo {
			id: 3,
			question_text: "What database do you use most?".to_string(),
			pub_date: chrono::Utc::now(),
			author_id: 1,
		},
	]
}

/// Create a mock list of ChoiceInfo for testing
fn mock_choices() -> Vec<ChoiceInfo> {
	vec![
		ChoiceInfo {
			id: 1,
			question_id: 1,
			choice_text: "Rust".to_string(),
			votes: 42,
		},
		ChoiceInfo {
			id: 2,
			question_id: 1,
			choice_text: "Python".to_string(),
			votes: 30,
		},
		ChoiceInfo {
			id: 3,
			question_id: 1,
			choice_text: "JavaScript".to_string(),
			votes: 25,
		},
	]
}

/// Create a mock VoteRequest for testing
#[allow(dead_code)] // For future mock tests
fn mock_vote_request() -> VoteRequest {
	VoteRequest {
		question_id: 1,
		choice_id: 1,
	}
}

fn install_poll_test_root() -> DomElement {
	let doc = document();
	if let Some(prev) = doc
		.query_selector("#polls-test-root")
		.expect("query test root")
	{
		prev.as_web_sys().remove();
	}

	let root = doc.create_element("div").expect("create test root");
	root.set_attribute("id", "polls-test-root")
		.expect("set test root id");
	doc.body()
		.expect("document body")
		.as_web_sys()
		.append_child(root.as_web_sys())
		.expect("append test root");
	root
}

async fn await_selector(selector: &str) -> DomElement {
	let doc = document();
	for _ in 0..100 {
		if let Some(element) = doc.query_selector(selector).expect("query selector") {
			return element;
		}
		TimeoutFuture::new(50).await;
	}

	let root_html = doc
		.query_selector("#polls-test-root")
		.expect("query test root")
		.map(|root| root.as_web_sys().inner_html())
		.unwrap_or_else(|| "<missing #polls-test-root>".to_string());
	panic!("timed out waiting for selector `{selector}` in #polls-test-root DOM: {root_html}");
}

// ============================================================================
// Polls Index Rendering Tests
// ============================================================================

/// Test polls index renders as a Page::Element
#[wasm_bindgen_test]
fn test_polls_index_renders() {
	let view = polls_index();
	assert!(matches!(view, Page::Element(_)));
}

/// Test polls index contains title
#[wasm_bindgen_test]
fn test_polls_index_has_title() {
	let view = polls_index();
	let html = view.render_to_string();
	assert!(html.contains("Polls"), "Should have 'Polls' title");
	assert!(html.contains("<h1"), "Title should be in h1 tag");
}

/// Test polls index has container class
#[wasm_bindgen_test]
fn test_polls_index_has_container() {
	let view = polls_index();
	let html = view.render_to_string();
	assert!(
		html.contains("max-w-4xl") && html.contains("mx-auto"),
		"Should have the current centered page container classes"
	);
}

// ============================================================================
// Polls Detail Rendering Tests
// ============================================================================

/// Test polls detail renders as a Page::Element
#[wasm_bindgen_test]
fn test_polls_detail_renders() {
	let view = polls_detail(1);
	assert!(matches!(view, Page::Element(_)));
}

/// Test polls detail renders for different question IDs
#[wasm_bindgen_test]
fn test_polls_detail_different_ids() {
	// Test with various question IDs
	for id in [1, 2, 10, 100] {
		let view = polls_detail(id);
		assert!(
			matches!(view, Page::Element(_)),
			"Should render for question_id = {}",
			id
		);
	}
}

// ============================================================================
// Polls Results Rendering Tests
// ============================================================================

/// Test polls results renders as a Page::Element
#[wasm_bindgen_test]
fn test_polls_results_renders() {
	let view = polls_results(1);
	assert!(matches!(view, Page::Element(_)));
}

/// Test polls results renders for different question IDs
#[wasm_bindgen_test]
fn test_polls_results_different_ids() {
	// Test with various question IDs
	for id in [1, 2, 10, 100] {
		let view = polls_results(id);
		assert!(
			matches!(view, Page::Element(_)),
			"Should render for question_id = {}",
			id
		);
	}
}

// ============================================================================
// Mock Infrastructure Tests
// ============================================================================

// ============================================================================
// Real server_fn round-trip tests via MSW
//
// These tests don't just verify that the mock can be registered — they
// actually invoke the application's `#[server_fn]` functions, let MSW
// intercept the underlying `window.fetch()` call, deserialize the typed
// response, and assert on the payload that the client code would see.
// ============================================================================

/// `get_questions()` returns the list mocked by MSW (success path).
#[wasm_bindgen_test]
async fn test_get_questions_returns_mocked_list() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_questions::marker>(|_args| Ok(mock_questions_list()));
	worker.start().await;

	let questions = get_questions().await.expect("server_fn should succeed");

	assert_eq!(questions.len(), 3);
	assert_eq!(
		questions[0].question_text,
		"What is your favorite programming language?"
	);
	assert_eq!(questions[1].id, 2);

	worker
		.calls_to_server_fn::<get_questions::marker>()
		.assert_called();
}

/// `get_questions()` surfaces a server-side error from MSW (error path).
#[wasm_bindgen_test]
async fn test_get_questions_surfaces_server_error() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_questions::marker>(|_args| {
		Err(ServerFnError::server(500, "Internal server error"))
	});
	worker.start().await;

	let err = get_questions().await.expect_err("expected server error");
	match err {
		ServerFnError::Server { status, message } => {
			assert_eq!(status, 500, "expected HTTP 500 status");
			assert_eq!(
				message, "Internal server error",
				"expected mocked server message to propagate verbatim"
			);
		}
		other => panic!("expected ServerFnError::Server, got: {other:?}"),
	}
}

/// `get_question_detail(qid)` round-trips the `(QuestionInfo, Vec<ChoiceInfo>)` tuple
/// through MSW.
#[wasm_bindgen_test]
async fn test_get_question_detail_round_trip() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_detail::marker>(|_args| {
		Ok((mock_question(), mock_choices()))
	});
	worker.start().await;

	let (question, choices) = get_question_detail(1)
		.await
		.expect("server_fn should succeed");

	assert_eq!(question.id, 1);
	assert_eq!(
		question.question_text,
		"What is your favorite programming language?"
	);
	assert_eq!(choices.len(), 3);
	assert_eq!(choices[0].choice_text, "Rust");
	assert_eq!(choices[0].votes, 42);
}

/// `get_question_detail(qid)` surfaces a 404 NotFound from MSW.
#[wasm_bindgen_test]
async fn test_get_question_detail_surfaces_not_found() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_detail::marker>(|_args| {
		Err(ServerFnError::server(404, "Question not found"))
	});
	worker.start().await;

	let err = get_question_detail(99)
		.await
		.expect_err("expected not-found error");
	match err {
		ServerFnError::Server { status, message } => {
			assert_eq!(status, 404, "expected HTTP 404 status");
			assert_eq!(
				message, "Question not found",
				"expected mocked not-found message to propagate verbatim"
			);
		}
		other => panic!("expected ServerFnError::Server, got: {other:?}"),
	}
}

/// `get_question_results(qid)` round-trips the `(QuestionInfo, Vec<ChoiceInfo>, i32)`
/// tuple — used by `polls_results` for the bar-chart rendering.
#[wasm_bindgen_test]
async fn test_get_question_results_round_trip() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_results::marker>(|_args| {
		Ok((mock_question(), mock_choices(), 97_i32))
	});
	worker.start().await;

	let (question, choices, total) = get_question_results(1)
		.await
		.expect("server_fn should succeed");

	assert_eq!(question.id, 1);
	assert_eq!(choices.len(), 3);
	assert_eq!(total, 97);
	assert_eq!(total, choices.iter().map(|c| c.votes).sum::<i32>());
}

/// `vote(VoteRequest)` returns the chosen `ChoiceInfo` when MSW supplies one.
#[wasm_bindgen_test]
async fn test_vote_succeeds() {
	let worker = MockServiceWorker::new();
	let mocked_choice = mock_choices()[0].clone(); // "Rust", 42 votes
	worker.handle_server_fn::<vote::marker>({
		let c = mocked_choice.clone();
		move |_args| Ok(c.clone())
	});
	worker.start().await;

	let choice = vote(VoteRequest {
		question_id: 1,
		choice_id: 1,
	})
	.await
	.expect("vote should succeed");

	assert_eq!(choice.id, mocked_choice.id);
	assert_eq!(choice.choice_text, "Rust");
	worker.calls_to_server_fn::<vote::marker>().assert_called();
}

/// `vote(...)` surfaces an application error from MSW (e.g., invalid choice).
#[wasm_bindgen_test]
async fn test_vote_surfaces_invalid_choice() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<vote::marker>(|_args| {
		Err(ServerFnError::server(400, "Invalid choice"))
	});
	worker.start().await;

	let err = vote(VoteRequest {
		question_id: 1,
		choice_id: 999,
	})
	.await
	.expect_err("expected invalid-choice error");
	match err {
		ServerFnError::Server { status, message } => {
			assert_eq!(status, 400, "expected HTTP 400 status");
			assert_eq!(
				message, "Invalid choice",
				"expected mocked invalid-choice message to propagate verbatim"
			);
		}
		other => panic!("expected ServerFnError::Server, got: {other:?}"),
	}
}

// ============================================================================
// Component-with-MSW smoke tests
//
// These exercise the real polls component renders while MSW is active, so
// they cover the path from `use_action` dispatch through MSW interception
// to typed response deserialization. They don't await reactive re-renders
// (mounting + scheduler flush is outside this file's scope), but they
// prove that the component constructs without panicking when MSW is in
// place — exactly the regression class chased through cloud's SPA work.
// ============================================================================

/// `polls_index()` constructs cleanly with MSW intercepting `get_questions`.
#[wasm_bindgen_test]
async fn test_polls_index_with_msw_active() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_questions::marker>(|_args| Ok(mock_questions_list()));
	worker.start().await;

	let view = polls_index();
	assert!(matches!(view, Page::Element(_)));
}

/// `polls_detail(qid)` constructs cleanly with MSW intercepting
/// `get_question_detail`.
#[wasm_bindgen_test]
async fn test_polls_detail_with_msw_active() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_detail::marker>(|_args| {
		Ok((mock_question(), mock_choices()))
	});
	worker.start().await;

	let view = polls_detail(1);
	assert!(matches!(view, Page::Element(_)));
}

/// `polls_detail(qid)` renders one radio input for each loaded choice.
#[wasm_bindgen_test]
async fn test_polls_detail_renders_loaded_choice_radios() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_detail::marker>(|_args| {
		Ok((mock_question(), mock_choices()))
	});
	worker.handle_server_fn::<current_user::marker>(|_args| Ok(None));
	worker.start().await;

	let root = install_poll_test_root();
	polls_detail(1).mount(&root).expect("mount polls detail");

	await_selector("input[type='radio'][name='choice_id'][value='1']").await;
	await_selector("input[type='radio'][name='choice_id'][value='2']").await;
	await_selector("input[type='radio'][name='choice_id'][value='3']").await;

	worker
		.calls_to_server_fn::<get_question_detail::marker>()
		.assert_called();
}

/// Selecting a poll choice must survive the reactive re-render triggered by the
/// form field signal update.
#[wasm_bindgen_test]
async fn test_polls_detail_keeps_clicked_radio_checked() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_detail::marker>(|_args| {
		Ok((mock_question(), mock_choices()))
	});
	worker.handle_server_fn::<current_user::marker>(|_args| Ok(None));
	worker.start().await;

	let root = install_poll_test_root();
	polls_detail(1).mount(&root).expect("mount polls detail");

	let selector = "input[type='radio'][name='choice_id'][value='1']";
	let input = await_selector(selector)
		.await
		.as_web_sys()
		.clone()
		.dyn_into::<web_sys::HtmlInputElement>()
		.expect("choice radio should be an input element");
	input.click();

	TimeoutFuture::new(50).await;

	let current_input = document()
		.query_selector(selector)
		.expect("query selected radio")
		.expect("selected radio should still exist")
		.as_web_sys()
		.clone()
		.dyn_into::<web_sys::HtmlInputElement>()
		.expect("current choice radio should be an input element");
	assert!(
		current_input.checked(),
		"clicked choice radio should stay checked after reactive update"
	);
}

/// `polls_results(qid)` constructs cleanly with MSW intercepting
/// `get_question_results`.
#[wasm_bindgen_test]
async fn test_polls_results_with_msw_active() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<get_question_results::marker>(|_args| {
		Ok((mock_question(), mock_choices(), 97_i32))
	});
	worker.start().await;

	let view = polls_results(1);
	assert!(matches!(view, Page::Element(_)));
}

// ============================================================================
// Shared Types Serialization Tests
// ============================================================================

/// Test QuestionInfo serialization
#[wasm_bindgen_test]
fn test_question_info_serialization() {
	let question = mock_question();

	let json = serde_json::to_string(&question).expect("Should serialize QuestionInfo");
	assert!(json.contains("favorite programming language"));
	assert!(json.contains("question_text"));
	assert!(json.contains("pub_date"));
}

/// Test QuestionInfo deserialization
#[wasm_bindgen_test]
fn test_question_info_deserialization() {
	let json = r#"{
	        "id": 1,
	        "question_text": "What is Rust?",
	        "pub_date": "2025-01-01T12:00:00Z",
	        "author_id": 1
	    }"#;

	let question: QuestionInfo =
		serde_json::from_str(json).expect("Should deserialize QuestionInfo");
	assert_eq!(question.id, 1);
	assert_eq!(question.question_text, "What is Rust?");
}

/// Test ChoiceInfo serialization
#[wasm_bindgen_test]
fn test_choice_info_serialization() {
	let choice = ChoiceInfo {
		id: 1,
		question_id: 1,
		choice_text: "Rust".to_string(),
		votes: 42,
	};

	let json = serde_json::to_string(&choice).expect("Should serialize ChoiceInfo");
	assert!(json.contains("Rust"));
	assert!(json.contains("42"));
	assert!(json.contains("question_id"));
}

/// Test ChoiceInfo deserialization
#[wasm_bindgen_test]
fn test_choice_info_deserialization() {
	let json = r#"{
        "id": 1,
        "question_id": 1,
        "choice_text": "Python",
        "votes": 30
    }"#;

	let choice: ChoiceInfo = serde_json::from_str(json).expect("Should deserialize ChoiceInfo");
	assert_eq!(choice.id, 1);
	assert_eq!(choice.choice_text, "Python");
	assert_eq!(choice.votes, 30);
}

/// Test VoteRequest serialization
#[wasm_bindgen_test]
fn test_vote_request_serialization() {
	let request = VoteRequest {
		question_id: 1,
		choice_id: 2,
	};

	let json = serde_json::to_string(&request).expect("Should serialize VoteRequest");
	assert!(json.contains("question_id"));
	assert!(json.contains("choice_id"));
}

/// Test QuestionInfo roundtrip serialization
#[wasm_bindgen_test]
fn test_question_info_roundtrip() {
	let original = mock_question();
	let json = serde_json::to_string(&original).expect("Should serialize");
	let deserialized: QuestionInfo = serde_json::from_str(&json).expect("Should deserialize");

	assert_eq!(original.id, deserialized.id);
	assert_eq!(original.question_text, deserialized.question_text);
}

/// Test ChoiceInfo roundtrip serialization
#[wasm_bindgen_test]
fn test_choice_info_roundtrip() {
	let original = mock_choices()[0].clone();
	let json = serde_json::to_string(&original).expect("Should serialize");
	let deserialized: ChoiceInfo = serde_json::from_str(&json).expect("Should deserialize");

	assert_eq!(original.id, deserialized.id);
	assert_eq!(original.choice_text, deserialized.choice_text);
	assert_eq!(original.votes, deserialized.votes);
}

/// Test questions list serialization
#[wasm_bindgen_test]
fn test_questions_list_serialization() {
	let questions = mock_questions_list();

	let json = serde_json::to_string(&questions).expect("Should serialize questions list");
	let deserialized: Vec<QuestionInfo> =
		serde_json::from_str(&json).expect("Should deserialize questions list");

	assert_eq!(questions.len(), deserialized.len());
	assert_eq!(questions[0].id, deserialized[0].id);
	assert_eq!(questions[1].question_text, deserialized[1].question_text);
}

/// Test choices list serialization
#[wasm_bindgen_test]
fn test_choices_list_serialization() {
	let choices = mock_choices();

	let json = serde_json::to_string(&choices).expect("Should serialize choices list");
	let deserialized: Vec<ChoiceInfo> =
		serde_json::from_str(&json).expect("Should deserialize choices list");

	assert_eq!(choices.len(), deserialized.len());
	assert_eq!(choices[0].choice_text, deserialized[0].choice_text);
}

// ============================================================================
// VoteRequest Tests
// ============================================================================

/// Test VoteRequest with various values
#[wasm_bindgen_test]
fn test_vote_request_various_values() {
	let requests = vec![
		VoteRequest {
			question_id: 1,
			choice_id: 1,
		},
		VoteRequest {
			question_id: 100,
			choice_id: 500,
		},
		VoteRequest {
			question_id: 0,
			choice_id: 0,
		},
	];

	for request in requests {
		let json = serde_json::to_string(&request).expect("Should serialize");
		let deserialized: VoteRequest = serde_json::from_str(&json).expect("Should deserialize");
		assert_eq!(request.question_id, deserialized.question_id);
		assert_eq!(request.choice_id, deserialized.choice_id);
	}
}

// ============================================================================
// ChoiceInfo Vote Count Tests
// ============================================================================

/// Test ChoiceInfo with zero votes
#[wasm_bindgen_test]
fn test_choice_info_zero_votes() {
	let choice = ChoiceInfo {
		id: 1,
		question_id: 1,
		choice_text: "New Choice".to_string(),
		votes: 0,
	};

	assert_eq!(choice.votes, 0);

	let json = serde_json::to_string(&choice).expect("Should serialize");
	let deserialized: ChoiceInfo = serde_json::from_str(&json).expect("Should deserialize");
	assert_eq!(deserialized.votes, 0);
}

/// Test ChoiceInfo with high vote count
#[wasm_bindgen_test]
fn test_choice_info_high_votes() {
	let choice = ChoiceInfo {
		id: 1,
		question_id: 1,
		choice_text: "Popular Choice".to_string(),
		votes: 1_000_000,
	};

	let json = serde_json::to_string(&choice).expect("Should serialize");
	let deserialized: ChoiceInfo = serde_json::from_str(&json).expect("Should deserialize");
	assert_eq!(deserialized.votes, 1_000_000);
}
