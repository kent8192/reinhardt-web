//! Scoped N+1 query detection for ORM workloads.

use std::collections::{BTreeSet, HashMap, HashSet, hash_map::RandomState};
use std::future::Future;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use parking_lot::Mutex;

/// Action taken when a scoped N+1 detector finds suspicious query groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NPlusOneMode {
	/// Log findings without interrupting execution.
	Warn,
	/// Panic when findings exist.
	Fail,
}

/// Configuration for scoped N+1 query detection.
#[derive(Debug, Clone)]
pub struct NPlusOneConfig {
	/// Minimum executions for a repeated query shape before reporting.
	pub threshold: usize,
	/// Minimum distinct bind or literal signatures before reporting.
	pub min_distinct_params: usize,
	/// Maximum query samples stored in one scope.
	pub max_records_per_scope: usize,
	/// Normalized fingerprints that should never be reported.
	pub ignored_fingerprints: HashSet<String>,
	/// Reporting mode used when the scope completes.
	pub mode: NPlusOneMode,
}

impl Default for NPlusOneConfig {
	fn default() -> Self {
		Self {
			threshold: 10,
			min_distinct_params: 3,
			max_records_per_scope: 1024,
			ignored_fingerprints: HashSet::new(),
			mode: NPlusOneMode::Warn,
		}
	}
}

/// A suspicious repeated query shape found by an N+1 scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NPlusOneFinding {
	/// Normalized query fingerprint.
	pub fingerprint: String,
	/// Number of executions for the query shape.
	pub execution_count: usize,
	/// Number of distinct bind or inline literal signatures.
	pub distinct_bind_signature_count: usize,
	/// Normalized SQL statement for this fingerprint with literal values redacted.
	pub representative_sql: String,
	/// Bounded, masked bind or literal samples.
	pub representative_bind_samples: Vec<String>,
	/// Cumulative duration across the query shape.
	pub cumulative_duration: Duration,
	/// Generic remediation guidance.
	pub suggested_action: String,
}

/// Report produced when an N+1 detector scope completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NPlusOneReport {
	/// Developer-supplied label for the scoped workload.
	pub label: String,
	/// Findings sorted by execution count and cumulative duration.
	pub findings: Vec<NPlusOneFinding>,
	/// Number of samples retained in the scope.
	pub total_recorded_queries: usize,
	/// Number of samples dropped due to `max_records_per_scope`.
	pub dropped_sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct QueryFingerprint {
	normalized: String,
	literal_signature: String,
	literal_samples: Vec<String>,
}

impl QueryFingerprint {
	fn from_sql(sql: &str) -> Self {
		let mut normalized = String::with_capacity(sql.len());
		let mut literal_samples = Vec::new();
		let mut chars = sql.chars().peekable();

		while let Some(ch) = chars.next() {
			match ch {
				'\'' => {
					let literal = consume_quoted_literal(ch, &mut chars);
					normalized.push('?');
					literal_samples.push(mask_value(&literal));
				}
				'$' if chars.peek().is_some_and(|next| next.is_ascii_digit()) => {
					normalized.push('?');
					consume_ascii_digits(&mut chars);
				}
				':' if chars
					.peek()
					.is_some_and(|next| next.is_ascii_alphabetic() || *next == '_') =>
				{
					normalized.push('?');
					consume_identifier(&mut chars);
				}
				'?' => normalized.push('?'),
				c if c.is_ascii_digit() => {
					let literal = consume_number_literal(c, &mut chars);
					normalized.push('?');
					literal_samples.push(mask_value(&literal));
				}
				c if c.is_whitespace() => normalized.push(' '),
				c => normalized.push(c),
			}
		}

		let literal_signature = if literal_samples.is_empty() {
			"no-bind".to_string()
		} else {
			literal_samples.join(",")
		};

		Self {
			normalized: collapse_whitespace(&normalized),
			literal_signature,
			literal_samples,
		}
	}
}

#[derive(Debug)]
struct QueryAggregate {
	representative_sql: String,
	bind_signatures: BTreeSet<String>,
	representative_bind_samples: Vec<String>,
	execution_count: usize,
	cumulative_duration: Duration,
}

impl QueryAggregate {
	fn new(representative_sql: String) -> Self {
		Self {
			representative_sql,
			bind_signatures: BTreeSet::new(),
			representative_bind_samples: Vec::new(),
			execution_count: 0,
			cumulative_duration: Duration::ZERO,
		}
	}
}

#[derive(Debug)]
struct ScopeState {
	label: String,
	config: NPlusOneConfig,
	aggregates: HashMap<String, QueryAggregate>,
	total_recorded_queries: usize,
	dropped_sample_count: usize,
}

impl ScopeState {
	fn new(label: String, config: NPlusOneConfig) -> Self {
		Self {
			label,
			config,
			aggregates: HashMap::new(),
			total_recorded_queries: 0,
			dropped_sample_count: 0,
		}
	}

	fn record_query(&mut self, query: &str, params: &[String], duration: Duration) {
		let fingerprint = QueryFingerprint::from_sql(query);
		if self
			.config
			.ignored_fingerprints
			.contains(&fingerprint.normalized)
		{
			return;
		}

		if self.total_recorded_queries >= self.config.max_records_per_scope {
			self.dropped_sample_count += 1;
			return;
		}

		self.total_recorded_queries += 1;

		let bind_signature = bind_signature(params, &fingerprint);
		let bind_samples = bind_samples(params, &fingerprint);
		let normalized = fingerprint.normalized;

		let aggregate = self
			.aggregates
			.entry(normalized.clone())
			.or_insert_with(|| QueryAggregate::new(normalized));

		aggregate.execution_count += 1;
		aggregate.cumulative_duration += duration;
		aggregate.bind_signatures.insert(bind_signature);

		for sample in bind_samples {
			if aggregate.representative_bind_samples.len() >= 3 {
				break;
			}
			if !aggregate.representative_bind_samples.contains(&sample) {
				aggregate.representative_bind_samples.push(sample);
			}
		}
	}

	fn finish_report(&self) -> NPlusOneReport {
		let mut findings = self
			.aggregates
			.iter()
			.filter(|(fingerprint, aggregate)| {
				aggregate.execution_count >= self.config.threshold
					&& aggregate.bind_signatures.len() >= self.config.min_distinct_params
					&& !self.config.ignored_fingerprints.contains(*fingerprint)
			})
			.map(|(fingerprint, aggregate)| NPlusOneFinding {
				fingerprint: fingerprint.clone(),
				execution_count: aggregate.execution_count,
				distinct_bind_signature_count: aggregate.bind_signatures.len(),
				representative_sql: aggregate.representative_sql.clone(),
				representative_bind_samples: aggregate.representative_bind_samples.clone(),
				cumulative_duration: aggregate.cumulative_duration,
				suggested_action: suggested_action(),
			})
			.collect::<Vec<_>>();

		findings.sort_by(|left, right| {
			right
				.execution_count
				.cmp(&left.execution_count)
				.then_with(|| right.cumulative_duration.cmp(&left.cumulative_duration))
		});

		NPlusOneReport {
			label: self.label.clone(),
			findings,
			total_recorded_queries: self.total_recorded_queries,
			dropped_sample_count: self.dropped_sample_count,
		}
	}
}

tokio::task_local! {
	static CURRENT_N_PLUS_ONE_SCOPE: Arc<Mutex<ScopeState>>;
}

static MASK_HASH_STATE: OnceLock<RandomState> = OnceLock::new();

/// Scoped N+1 query detector.
#[derive(Debug, Clone)]
pub struct NPlusOneScope {
	label: String,
	config: NPlusOneConfig,
}

impl NPlusOneScope {
	/// Creates a scope that logs findings when the scoped future completes.
	pub fn warn(label: impl Into<String>, mut config: NPlusOneConfig) -> Self {
		config.mode = NPlusOneMode::Warn;
		Self {
			label: label.into(),
			config,
		}
	}

	/// Creates a scope that panics when findings exist.
	pub fn fail(label: impl Into<String>, mut config: NPlusOneConfig) -> Self {
		config.mode = NPlusOneMode::Fail;
		Self {
			label: label.into(),
			config,
		}
	}

	/// Runs a future inside this scope and returns the future output.
	pub async fn run<F, T>(self, future: F) -> T
	where
		F: Future<Output = T>,
	{
		let (output, _) = self.run_with_report(future).await;
		output
	}

	/// Runs a future inside this scope and returns the output plus detector report.
	pub async fn run_with_report<F, T>(self, future: F) -> (T, NPlusOneReport)
	where
		F: Future<Output = T>,
	{
		let mode = self.config.mode;
		let state = Arc::new(Mutex::new(ScopeState::new(self.label, self.config)));
		let output = CURRENT_N_PLUS_ONE_SCOPE.scope(state.clone(), future).await;
		let report = state.lock().finish_report();

		match mode {
			NPlusOneMode::Warn => warn_for_findings(&report),
			NPlusOneMode::Fail => fail_for_findings(&report),
		}

		(output, report)
	}

	/// Spawns a task that inherits the active N+1 scope when one exists.
	pub fn spawn<F, T>(future: F) -> tokio::task::JoinHandle<T>
	where
		F: Future<Output = T> + Send + 'static,
		T: Send + 'static,
	{
		match CURRENT_N_PLUS_ONE_SCOPE.try_with(Arc::clone) {
			Ok(state) => tokio::spawn(CURRENT_N_PLUS_ONE_SCOPE.scope(state, future)),
			Err(_) => tokio::spawn(future),
		}
	}
}

pub(crate) fn record_query(query: &str, params: &[String], duration: Duration) -> bool {
	CURRENT_N_PLUS_ONE_SCOPE
		.try_with(|state| {
			state.lock().record_query(query, params, duration);
		})
		.is_ok()
}

fn bind_signature(params: &[String], fingerprint: &QueryFingerprint) -> String {
	if params.is_empty() {
		return fingerprint.literal_signature.clone();
	}

	params
		.iter()
		.map(|value| mask_value(value))
		.collect::<Vec<_>>()
		.join(",")
}

fn bind_samples(params: &[String], fingerprint: &QueryFingerprint) -> Vec<String> {
	if params.is_empty() {
		return fingerprint.literal_samples.clone();
	}

	params.iter().map(|value| mask_value(value)).collect()
}

fn mask_value(value: &str) -> String {
	let class = classify_value(value);
	let mut hasher = MASK_HASH_STATE.get_or_init(RandomState::new).build_hasher();
	value.hash(&mut hasher);
	format!("{class}#{:016x}", hasher.finish())
}

fn classify_value(value: &str) -> &'static str {
	if value.parse::<i128>().is_ok() {
		"integer"
	} else if value.parse::<f64>().is_ok() {
		"number"
	} else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
		"boolean"
	} else {
		"string"
	}
}

fn consume_quoted_literal<I>(quote: char, chars: &mut std::iter::Peekable<I>) -> String
where
	I: Iterator<Item = char>,
{
	let mut literal = String::new();

	while let Some(ch) = chars.next() {
		if ch == quote {
			if chars.peek().is_some_and(|next| *next == quote) {
				literal.push(ch);
				let _ = chars.next();
				continue;
			}
			break;
		}
		if ch == '\\' {
			if let Some(escaped) = chars.next() {
				literal.push(escaped);
			}
		} else {
			literal.push(ch);
		}
	}

	literal
}

fn consume_ascii_digits<I>(chars: &mut std::iter::Peekable<I>)
where
	I: Iterator<Item = char>,
{
	while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
		let _ = chars.next();
	}
}

fn consume_identifier<I>(chars: &mut std::iter::Peekable<I>)
where
	I: Iterator<Item = char>,
{
	while chars
		.peek()
		.is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
	{
		let _ = chars.next();
	}
}

fn consume_number_literal<I>(first: char, chars: &mut std::iter::Peekable<I>) -> String
where
	I: Iterator<Item = char>,
{
	let mut literal = String::from(first);
	let mut allow_exponent_sign = false;

	while let Some(ch) = chars.peek().copied() {
		if ch.is_ascii_digit() || ch == '_' || ch == '.' {
			literal.push(ch);
			let _ = chars.next();
			allow_exponent_sign = false;
		} else if matches!(ch, 'e' | 'E') {
			literal.push(ch);
			let _ = chars.next();
			allow_exponent_sign = true;
		} else if allow_exponent_sign && matches!(ch, '+' | '-') {
			literal.push(ch);
			let _ = chars.next();
			allow_exponent_sign = false;
		} else {
			break;
		}
	}

	literal
}

fn collapse_whitespace(input: &str) -> String {
	input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn suggested_action() -> String {
	"Repeated query shape detected. If this is a single-object relationship, consider select_related(). If this is a collection relationship, consider prefetch_related() or a batch query.".to_string()
}

fn warn_for_findings(report: &NPlusOneReport) {
	for finding in &report.findings {
		tracing::warn!(
			scope = %report.label,
			fingerprint = %finding.fingerprint,
			execution_count = finding.execution_count,
			distinct_bind_signature_count = finding.distinct_bind_signature_count,
			cumulative_duration_ms = finding.cumulative_duration.as_millis(),
			dropped_sample_count = report.dropped_sample_count,
			suggestion = %finding.suggested_action,
			"N+1 query pattern detected"
		);
	}
}

fn fail_for_findings(report: &NPlusOneReport) {
	if report.findings.is_empty() {
		return;
	}

	panic!("{}", format_report(report));
}

fn format_report(report: &NPlusOneReport) -> String {
	let mut message = format!(
		"N+1 query pattern detected in scope '{}': {} finding(s)",
		report.label,
		report.findings.len()
	);

	for finding in &report.findings {
		message.push_str(&format!(
			"\n- count={} distinct_bind_signatures={} fingerprint={} suggestion={}",
			finding.execution_count,
			finding.distinct_bind_signature_count,
			finding.fingerprint,
			finding.suggested_action
		));
	}

	message
}

#[cfg(test)]
mod tests {
	use super::*;

	fn low_threshold_config() -> NPlusOneConfig {
		let mut config = NPlusOneConfig::default();
		config.threshold = 3;
		config.min_distinct_params = 3;
		config
	}

	#[test]
	fn normalizes_literals_and_whitespace() {
		let fingerprint = QueryFingerprint::from_sql(
			"SELECT  * FROM posts WHERE author_id = 42 AND status = 'published'",
		);

		assert_eq!(
			fingerprint.normalized,
			"SELECT * FROM posts WHERE author_id = ? AND status = ?"
		);
	}

	#[test]
	fn preserves_double_quoted_identifiers() {
		let posts = QueryFingerprint::from_sql(r#"SELECT * FROM "posts" WHERE "author_id" = 1"#);
		let comments =
			QueryFingerprint::from_sql(r#"SELECT * FROM "comments" WHERE "post_id" = 2"#);

		assert_eq!(
			posts.normalized,
			r#"SELECT * FROM "posts" WHERE "author_id" = ?"#
		);
		assert_eq!(
			comments.normalized,
			r#"SELECT * FROM "comments" WHERE "post_id" = ?"#
		);
		assert_ne!(posts.normalized, comments.normalized);
	}

	#[test]
	fn normalizes_bind_placeholders() {
		let fingerprint = QueryFingerprint::from_sql(
			"SELECT * FROM posts WHERE author_id = $1 AND category_id = :category_id",
		);

		assert_eq!(
			fingerprint.normalized,
			"SELECT * FROM posts WHERE author_id = ? AND category_id = ?"
		);
	}

	#[test]
	fn reports_same_shape_with_distinct_bind_values() {
		let mut state = ScopeState::new("posts.index".to_string(), low_threshold_config());
		for author_id in ["1", "2", "3"] {
			state.record_query(
				"SELECT * FROM posts WHERE author_id = $1",
				&[author_id.to_string()],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.findings.len(), 1);
		assert_eq!(report.findings[0].execution_count, 3);
		assert_eq!(report.findings[0].distinct_bind_signature_count, 3);
	}

	#[test]
	fn detects_inline_literals_when_params_are_empty() {
		let mut state = ScopeState::new("posts.index".to_string(), low_threshold_config());
		for author_id in ["1", "2", "3"] {
			state.record_query(
				&format!("SELECT * FROM posts WHERE author_id = {author_id}"),
				&[],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.findings.len(), 1);
		assert_eq!(
			report.findings[0].fingerprint,
			"SELECT * FROM posts WHERE author_id = ?"
		);
		assert_eq!(report.findings[0].distinct_bind_signature_count, 3);
	}

	#[test]
	fn representative_sql_uses_normalized_fingerprint() {
		let mut state = ScopeState::new("users.index".to_string(), low_threshold_config());
		for email in [
			"a@example.com",
			"b@example.com",
			"private-token@example.com",
		] {
			state.record_query(
				&format!("SELECT * FROM users WHERE email = '{email}'"),
				&[],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.findings.len(), 1);
		assert_eq!(
			report.findings[0].representative_sql,
			"SELECT * FROM users WHERE email = ?"
		);
		assert!(
			!report.findings[0]
				.representative_sql
				.contains("private-token")
		);
	}

	#[test]
	fn ignores_repeated_query_with_same_bind_signature() {
		let mut config = NPlusOneConfig::default();
		config.threshold = 3;
		config.min_distinct_params = 2;

		let mut state = ScopeState::new("posts.index".to_string(), config);
		for _ in 0..3 {
			state.record_query(
				"SELECT * FROM posts WHERE author_id = $1",
				&["1".to_string()],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert!(report.findings.is_empty());
	}

	#[test]
	fn ignored_fingerprint_suppresses_finding() {
		let mut config = low_threshold_config();
		config
			.ignored_fingerprints
			.insert("SELECT * FROM posts WHERE author_id = ?".to_string());

		let mut state = ScopeState::new("posts.index".to_string(), config);
		for author_id in ["1", "2", "3"] {
			state.record_query(
				"SELECT * FROM posts WHERE author_id = $1",
				&[author_id.to_string()],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert!(report.findings.is_empty());
		assert_eq!(report.total_recorded_queries, 0);
		assert_eq!(report.dropped_sample_count, 0);
	}

	#[test]
	fn ignored_fingerprints_do_not_consume_record_cap() {
		let mut config = low_threshold_config();
		config.max_records_per_scope = 3;
		config
			.ignored_fingerprints
			.insert("SELECT * FROM posts WHERE author_id = ?".to_string());

		let mut state = ScopeState::new("posts.index".to_string(), config);
		for author_id in ["1", "2", "3"] {
			state.record_query(
				"SELECT * FROM posts WHERE author_id = $1",
				&[author_id.to_string()],
				Duration::from_millis(1),
			);
		}
		for post_id in ["1", "2", "3"] {
			state.record_query(
				"SELECT * FROM comments WHERE post_id = $1",
				&[post_id.to_string()],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.total_recorded_queries, 3);
		assert_eq!(report.dropped_sample_count, 0);
		assert_eq!(report.findings.len(), 1);
		assert_eq!(
			report.findings[0].fingerprint,
			"SELECT * FROM comments WHERE post_id = ?"
		);
	}

	#[test]
	fn max_record_cap_drops_extra_samples() {
		let mut config = low_threshold_config();
		config.max_records_per_scope = 2;

		let mut state = ScopeState::new("posts.index".to_string(), config);
		for author_id in ["1", "2", "3"] {
			state.record_query(
				"SELECT * FROM posts WHERE author_id = $1",
				&[author_id.to_string()],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.total_recorded_queries, 2);
		assert_eq!(report.dropped_sample_count, 1);
		assert!(report.findings.is_empty());
	}

	#[test]
	fn findings_are_sorted_by_count_then_duration() {
		let mut config = low_threshold_config();
		config.threshold = 2;
		config.min_distinct_params = 2;
		let mut state = ScopeState::new("posts.index".to_string(), config);

		for author_id in ["1", "2", "3"] {
			state.record_query(
				"SELECT * FROM comments WHERE author_id = $1",
				&[author_id.to_string()],
				Duration::from_millis(1),
			);
		}
		for category_id in ["1", "2"] {
			state.record_query(
				"SELECT * FROM posts WHERE category_id = $1",
				&[category_id.to_string()],
				Duration::from_millis(20),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.findings.len(), 2);
		assert_eq!(
			report.findings[0].fingerprint,
			"SELECT * FROM comments WHERE author_id = ?"
		);
		assert_eq!(
			report.findings[1].fingerprint,
			"SELECT * FROM posts WHERE category_id = ?"
		);
	}

	#[test]
	fn does_not_expose_raw_bind_values_in_samples() {
		let mut state = ScopeState::new("posts.index".to_string(), low_threshold_config());
		for email in [
			"a@example.com",
			"b@example.com",
			"private-token@example.com",
		] {
			state.record_query(
				"SELECT * FROM users WHERE email = $1",
				&[email.to_string()],
				Duration::from_millis(1),
			);
		}

		let report = state.finish_report();
		assert_eq!(report.findings.len(), 1);
		assert!(
			report.findings[0]
				.representative_bind_samples
				.iter()
				.all(|sample| !sample.contains('@') && !sample.contains("private-token"))
		);
	}

	#[tokio::test]
	async fn warn_scope_collects_active_task_queries() {
		let (_, report) = NPlusOneScope::warn("posts.index", low_threshold_config())
			.run_with_report(async {
				for author_id in ["1", "2", "3"] {
					record_query(
						"SELECT * FROM posts WHERE author_id = $1",
						&[author_id.to_string()],
						Duration::from_millis(1),
					);
				}
			})
			.await;

		assert_eq!(report.findings.len(), 1);
	}

	#[tokio::test]
	#[should_panic(expected = "N+1 query pattern detected")]
	async fn fail_scope_panics_when_findings_exist() {
		NPlusOneScope::fail("posts.index", low_threshold_config())
			.run(async {
				for author_id in ["1", "2", "3"] {
					record_query(
						"SELECT * FROM posts WHERE author_id = $1",
						&[author_id.to_string()],
						Duration::from_millis(1),
					);
				}
			})
			.await;
	}

	#[tokio::test]
	async fn fail_scope_does_not_panic_when_findings_are_absent() {
		NPlusOneScope::fail("posts.index", low_threshold_config())
			.run(async {
				record_query(
					"SELECT * FROM posts WHERE author_id = $1",
					&["1".to_string()],
					Duration::from_millis(1),
				);
			})
			.await;
	}

	#[tokio::test]
	async fn parallel_scopes_do_not_share_query_samples() {
		let first =
			NPlusOneScope::warn("posts.index", low_threshold_config()).run_with_report(async {
				for author_id in ["1", "2", "3"] {
					record_query(
						"SELECT * FROM posts WHERE author_id = $1",
						&[author_id.to_string()],
						Duration::from_millis(1),
					);
				}
			});
		let second =
			NPlusOneScope::warn("users.index", low_threshold_config()).run_with_report(async {
				record_query(
					"SELECT * FROM users WHERE id = $1",
					&["1".to_string()],
					Duration::from_millis(1),
				);
			});

		let ((_, first_report), (_, second_report)) = tokio::join!(first, second);

		assert_eq!(first_report.findings.len(), 1);
		assert!(second_report.findings.is_empty());
		assert_eq!(first_report.total_recorded_queries, 3);
		assert_eq!(second_report.total_recorded_queries, 1);
	}

	#[tokio::test]
	async fn spawned_scope_tasks_share_query_samples() {
		let (_, report) = NPlusOneScope::warn("posts.index", low_threshold_config())
			.run_with_report(async {
				let mut handles = Vec::new();
				for author_id in ["1", "2", "3"] {
					handles.push(NPlusOneScope::spawn(async move {
						record_query(
							"SELECT * FROM posts WHERE author_id = $1",
							&[author_id.to_string()],
							Duration::from_millis(1),
						);
					}));
				}

				for handle in handles {
					handle.await.expect("scoped task should complete");
				}
			})
			.await;

		assert_eq!(report.findings.len(), 1);
		assert_eq!(report.total_recorded_queries, 3);
	}

	#[test]
	fn record_query_returns_false_outside_active_scope() {
		assert!(!record_query(
			"SELECT * FROM posts WHERE author_id = $1",
			&["1".to_string()],
			Duration::from_millis(1),
		));
	}
}
