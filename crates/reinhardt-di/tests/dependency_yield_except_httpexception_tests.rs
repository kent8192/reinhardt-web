//! FastAPI dependency yield except HTTPException tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_yield_except_httpexception.py
//!
//! These tests verify that:
//! 1. Dependencies can catch HTTPException in except block after yield
//! 2. Finally blocks run regardless of exceptions
//! 3. Database changes are rolled back when exception occurs

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct HttpException {
	status_code: u16,
	detail: String,
}

// Simulated database with transactional behavior
#[derive(Clone)]
struct Database {
	data: Arc<Mutex<HashMap<String, String>>>,
	temp_data: HashMap<String, String>,
	exception_caught: Arc<AtomicBool>,
	finally_ran: Arc<AtomicBool>,
}

impl Drop for Database {
	fn drop(&mut self) {
		// This simulates the finally block
		self.finally_ran.store(true, Ordering::SeqCst);
	}
}

impl Database {
	fn new(
		data: Arc<Mutex<HashMap<String, String>>>,
		exception_caught: Arc<AtomicBool>,
		finally_ran: Arc<AtomicBool>,
	) -> Self {
		let temp_data = data.lock().unwrap().clone();
		Database {
			data,
			temp_data,
			exception_caught,
			finally_ran,
		}
	}

	fn set(&mut self, key: String, value: String) {
		self.temp_data.insert(key, value);
	}

	fn commit(&self) -> DiResult<()> {
		let mut data = self.data.lock().unwrap();
		*data = self.temp_data.clone();
		Ok(())
	}

	fn rollback_on_exception(&self) {
		// Simulate catching HTTPException
		self.exception_caught.store(true, Ordering::SeqCst);
		// Don't commit changes - rollback
	}
}

#[async_trait::async_trait]
impl Injectable for Database {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Get shared state
		let data = if let Some(d) = ctx.get_singleton::<Arc<Mutex<HashMap<String, String>>>>() {
			(*d).clone()
		} else {
			let mut initial = HashMap::new();
			initial.insert("rick".to_string(), "Rick Sanchez".to_string());
			let data = Arc::new(Mutex::new(initial));
			ctx.set_singleton(data.clone());
			data
		};

		let exception_caught = if let Some(e) = ctx.get_request::<Arc<AtomicBool>>() {
			(*e).clone()
		} else {
			let e = Arc::new(AtomicBool::new(false));
			ctx.set_request(e.clone());
			e
		};

		let finally_ran = if let Some(f) = ctx.get_singleton::<Arc<AtomicBool>>() {
			(*f).clone()
		} else {
			let f = Arc::new(AtomicBool::new(false));
			ctx.set_singleton(f.clone());
			f
		};

		Ok(Database::new(data, exception_caught, finally_ran))
	}
}

#[tokio::test]
async fn test_dependency_gets_exception() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton.clone());

	// Inject database
	let mut db = Database::inject(&ctx).await.unwrap();

	// Get references to state trackers
	let exception_caught = db.exception_caught.clone();
	let finally_ran = db.finally_ran.clone();

	// Simulate request that raises HTTPException
	db.set("rick".to_string(), "Morty".to_string());

	// Simulate HTTPException raised in endpoint
	db.rollback_on_exception();

	// Drop database - triggers finally
	drop(db);

	// Verify state
	assert!(exception_caught.load(Ordering::SeqCst));
	assert!(finally_ran.load(Ordering::SeqCst));

	// Verify database was not updated
	let data = ctx
		.get_singleton::<Arc<Mutex<HashMap<String, String>>>>()
		.unwrap();
	let data_guard = data.lock().unwrap();
	assert_eq!(data_guard.get("rick").unwrap(), "Rick Sanchez");
}

#[tokio::test]
async fn test_dependency_no_exception() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton.clone());

	// Inject database
	let mut db = Database::inject(&ctx).await.unwrap();

	// Get references to state trackers
	let exception_caught = db.exception_caught.clone();
	let finally_ran = db.finally_ran.clone();

	// Simulate successful request
	db.set("rick".to_string(), "Morty".to_string());

	// Commit changes
	db.commit().unwrap();

	// Drop database - triggers finally
	drop(db);

	// Verify state
	assert!(!exception_caught.load(Ordering::SeqCst));
	assert!(finally_ran.load(Ordering::SeqCst));

	// Verify database was updated
	let data = ctx
		.get_singleton::<Arc<Mutex<HashMap<String, String>>>>()
		.unwrap();
	let data_guard = data.lock().unwrap();
	assert_eq!(data_guard.get("rick").unwrap(), "Morty");
}
