//! Database-backed durable job queue.
//!
//! The durable queue stores job records and lifecycle events in an application
//! database so long-running work can survive process restarts and expose
//! queryable status to server functions and UI polling.

use crate::{RetryStrategy, TaskId, TaskPriority};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{
	Row, SqlitePool,
	sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
};
use std::{fmt, str::FromStr, sync::Arc, time::Duration};
use thiserror::Error;

/// Durable job identifier.
pub type JobId = TaskId;

/// Shared durable queue type suitable for dependency injection.
pub type SharedDurableQueue = DurableQueue<Arc<dyn DurableJobStore>>;

/// Dependency injection key for shared durable queue providers.
#[cfg(feature = "di")]
#[derive(Debug, Clone, Copy, Default)]
pub struct DurableQueueKey;

#[cfg(feature = "di")]
impl reinhardt_di::InjectableKey for DurableQueueKey {}

const DEFAULT_DURABLE_QUEUE_NAME: &str = "default";
const DEFAULT_CLAIM_LEASE: Duration = Duration::from_secs(300);

/// Durable job lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
	/// The job is waiting to be claimed by a worker.
	Queued,
	/// The job has been claimed and is running.
	Running,
	/// The job finished successfully.
	Succeeded,
	/// The job failed and may be retried.
	FailedRetryable,
	/// The job failed permanently.
	FailedFinal,
	/// The job was canceled.
	Canceled,
}

impl JobState {
	/// Returns the stable database representation for this state.
	pub fn as_str(self) -> &'static str {
		match self {
			JobState::Queued => "queued",
			JobState::Running => "running",
			JobState::Succeeded => "succeeded",
			JobState::FailedRetryable => "failed_retryable",
			JobState::FailedFinal => "failed_final",
			JobState::Canceled => "canceled",
		}
	}

	/// Returns whether this state is terminal.
	pub fn is_terminal(self) -> bool {
		matches!(
			self,
			JobState::Succeeded | JobState::FailedFinal | JobState::Canceled
		)
	}

	/// Returns whether a transition from this state to `next` is legal.
	pub fn can_transition_to(self, next: JobState) -> bool {
		matches!(
			(self, next),
			(JobState::Queued, JobState::Running | JobState::Canceled)
				| (
					JobState::Running,
					JobState::Succeeded
						| JobState::FailedRetryable
						| JobState::FailedFinal
						| JobState::Canceled,
				) | (
				JobState::FailedRetryable,
				JobState::Queued | JobState::Canceled
			)
		)
	}
}

impl fmt::Display for JobState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for JobState {
	type Err = DurableQueueError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"queued" => Ok(Self::Queued),
			"running" => Ok(Self::Running),
			"succeeded" => Ok(Self::Succeeded),
			"failed_retryable" => Ok(Self::FailedRetryable),
			"failed_final" => Ok(Self::FailedFinal),
			"canceled" => Ok(Self::Canceled),
			other => Err(DurableQueueError::InvalidState(other.to_string())),
		}
	}
}

/// Durable job lifecycle event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobEventKind {
	/// The job was enqueued.
	Enqueued,
	/// The job was claimed by a worker.
	Claimed,
	/// The job succeeded.
	Succeeded,
	/// The job failed and can be retried.
	FailedRetryable,
	/// The job failed permanently.
	FailedFinal,
	/// Cancellation was requested for a running job.
	CancellationRequested,
	/// The job was canceled.
	Canceled,
	/// The job was requeued for retry.
	Retried,
}

impl JobEventKind {
	/// Returns the stable database representation for this event kind.
	pub fn as_str(self) -> &'static str {
		match self {
			JobEventKind::Enqueued => "enqueued",
			JobEventKind::Claimed => "claimed",
			JobEventKind::Succeeded => "succeeded",
			JobEventKind::FailedRetryable => "failed_retryable",
			JobEventKind::FailedFinal => "failed_final",
			JobEventKind::CancellationRequested => "cancellation_requested",
			JobEventKind::Canceled => "canceled",
			JobEventKind::Retried => "retried",
		}
	}
}

impl fmt::Display for JobEventKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for JobEventKind {
	type Err = DurableQueueError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"enqueued" => Ok(Self::Enqueued),
			"claimed" => Ok(Self::Claimed),
			"succeeded" => Ok(Self::Succeeded),
			"failed_retryable" => Ok(Self::FailedRetryable),
			"failed_final" => Ok(Self::FailedFinal),
			"cancellation_requested" => Ok(Self::CancellationRequested),
			"canceled" => Ok(Self::Canceled),
			"retried" => Ok(Self::Retried),
			other => Err(DurableQueueError::InvalidEventKind(other.to_string())),
		}
	}
}

/// A persisted durable job record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DurableJobRecord {
	/// Unique job identifier.
	pub id: JobId,
	/// Queue name used for worker claiming.
	pub queue: String,
	/// Application-defined job kind.
	pub kind: String,
	/// Optional application-defined target identifier.
	pub target: Option<String>,
	/// Current lifecycle state.
	pub state: JobState,
	/// Number of attempts already claimed by workers.
	pub attempt_count: u32,
	/// Maximum attempts allowed before final failure.
	pub max_attempts: u32,
	/// Job priority, where larger values are claimed first.
	pub priority: i32,
	/// JSON payload supplied at enqueue time.
	pub payload: Value,
	/// JSON result supplied on success.
	pub result: Option<Value>,
	/// Application-defined failure kind.
	pub failure_kind: Option<String>,
	/// Human-readable failure message.
	pub failure_message: Option<String>,
	/// Earliest time this job may be retried.
	pub retry_after: Option<DateTime<Utc>>,
	/// Whether cancellation was requested while the job was running.
	pub cancellation_requested: bool,
	/// Creation timestamp.
	pub created_at: DateTime<Utc>,
	/// Last update timestamp.
	pub updated_at: DateTime<Utc>,
	/// Timestamp for the current or latest run.
	pub started_at: Option<DateTime<Utc>>,
	/// Time at which the current running claim expires.
	pub lease_expires_at: Option<DateTime<Utc>>,
	/// Timestamp for the terminal state.
	pub finished_at: Option<DateTime<Utc>>,
}

impl DurableJobRecord {
	/// Returns a read-only snapshot for external status APIs.
	pub fn snapshot(&self) -> JobSnapshot {
		JobSnapshot {
			id: self.id,
			queue: self.queue.clone(),
			kind: self.kind.clone(),
			target: self.target.clone(),
			state: self.state,
			attempt_count: self.attempt_count,
			max_attempts: self.max_attempts,
			result: self.result.clone(),
			failure_kind: self.failure_kind.clone(),
			failure_message: self.failure_message.clone(),
			retry_after: self.retry_after,
			cancellation_requested: self.cancellation_requested,
			created_at: self.created_at,
			updated_at: self.updated_at,
			started_at: self.started_at,
			lease_expires_at: self.lease_expires_at,
			finished_at: self.finished_at,
		}
	}
}

/// Builder for durable jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
	queue: String,
	kind: String,
	target: Option<String>,
	max_attempts: u32,
	priority: i32,
	payload: Value,
	run_after: Option<DateTime<Utc>>,
}

impl JobSpec {
	/// Creates a job spec for the given application-defined kind.
	pub fn new(kind: impl Into<String>) -> Self {
		Self {
			queue: DEFAULT_DURABLE_QUEUE_NAME.to_string(),
			kind: kind.into(),
			target: None,
			max_attempts: 3,
			priority: TaskPriority::default().value(),
			payload: Value::Null,
			run_after: None,
		}
	}

	/// Sets the queue used by workers.
	pub fn queue(mut self, queue: impl Into<String>) -> Self {
		self.queue = queue.into();
		self
	}

	/// Sets an application-defined target identifier.
	pub fn target(mut self, target: impl ToString) -> Self {
		self.target = Some(target.to_string());
		self
	}

	/// Sets the maximum number of attempts.
	pub fn max_attempts(mut self, max_attempts: u32) -> Self {
		self.max_attempts = max_attempts.max(1);
		self
	}

	/// Sets the job priority.
	pub fn priority(mut self, priority: TaskPriority) -> Self {
		self.priority = priority.value();
		self
	}

	/// Sets the job priority from a raw value.
	pub fn priority_value(mut self, priority: i32) -> Self {
		self.priority = TaskPriority::new(priority).value();
		self
	}

	/// Sets the JSON payload by serializing the provided value.
	pub fn payload<T: Serialize>(mut self, payload: &T) -> Result<Self, DurableQueueError> {
		self.payload = serde_json::to_value(payload)?;
		Ok(self)
	}

	/// Sets the earliest time at which this job can be claimed.
	pub fn run_after(mut self, run_after: DateTime<Utc>) -> Self {
		self.run_after = Some(run_after);
		self
	}

	fn into_record(self, now: DateTime<Utc>) -> DurableJobRecord {
		DurableJobRecord {
			id: JobId::new(),
			queue: self.queue,
			kind: self.kind,
			target: self.target,
			state: JobState::Queued,
			attempt_count: 0,
			max_attempts: self.max_attempts,
			priority: self.priority,
			payload: self.payload,
			result: None,
			failure_kind: None,
			failure_message: None,
			retry_after: self.run_after,
			cancellation_requested: false,
			created_at: now,
			updated_at: now,
			started_at: None,
			lease_expires_at: None,
			finished_at: None,
		}
	}
}

/// Read-only job status snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSnapshot {
	/// Unique job identifier.
	pub id: JobId,
	/// Queue name used for worker claiming.
	pub queue: String,
	/// Application-defined job kind.
	pub kind: String,
	/// Optional application-defined target identifier.
	pub target: Option<String>,
	/// Current lifecycle state.
	pub state: JobState,
	/// Number of attempts already claimed by workers.
	pub attempt_count: u32,
	/// Maximum attempts allowed before final failure.
	pub max_attempts: u32,
	/// JSON result supplied on success.
	pub result: Option<Value>,
	/// Application-defined failure kind.
	pub failure_kind: Option<String>,
	/// Human-readable failure message.
	pub failure_message: Option<String>,
	/// Earliest time this job may be retried.
	pub retry_after: Option<DateTime<Utc>>,
	/// Whether cancellation was requested while the job was running.
	pub cancellation_requested: bool,
	/// Creation timestamp.
	pub created_at: DateTime<Utc>,
	/// Last update timestamp.
	pub updated_at: DateTime<Utc>,
	/// Timestamp for the current or latest run.
	pub started_at: Option<DateTime<Utc>>,
	/// Time at which the current running claim expires.
	pub lease_expires_at: Option<DateTime<Utc>>,
	/// Timestamp for the terminal state.
	pub finished_at: Option<DateTime<Utc>>,
}

/// Claimed durable job passed to worker completion APIs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobClaim {
	record: DurableJobRecord,
}

impl JobClaim {
	/// Creates a claim from a running job record.
	pub fn new(record: DurableJobRecord) -> Result<Self, DurableQueueError> {
		if record.state != JobState::Running {
			return Err(DurableQueueError::Conflict(JobTransitionConflict {
				job_id: record.id,
				from: record.state,
				to: JobState::Running,
				reason: "claims must wrap running jobs".to_string(),
			}));
		}
		Ok(Self { record })
	}

	/// Returns the claimed job identifier.
	pub fn id(&self) -> JobId {
		self.record.id
	}

	/// Returns the claimed job record.
	pub fn record(&self) -> &DurableJobRecord {
		&self.record
	}

	/// Returns the claimed job payload.
	pub fn payload<T: DeserializeOwned>(&self) -> Result<T, DurableQueueError> {
		serde_json::from_value(self.record.payload.clone()).map_err(DurableQueueError::from)
	}
}

/// Failure information used by durable completion APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobFailure {
	/// Application-defined failure kind.
	pub kind: String,
	/// Human-readable failure message.
	pub message: String,
}

impl JobFailure {
	/// Creates failure information.
	pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			kind: kind.into(),
			message: message.into(),
		}
	}
}

/// Persisted durable job lifecycle event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobEvent {
	/// Job identifier.
	pub job_id: JobId,
	/// Monotonic per-job sequence number.
	pub sequence: u64,
	/// Event kind.
	pub kind: JobEventKind,
	/// Previous state, if this event represents a transition.
	pub from_state: Option<JobState>,
	/// New state after the event.
	pub to_state: JobState,
	/// Optional human-readable message.
	pub message: Option<String>,
	/// Event creation timestamp.
	pub created_at: DateTime<Utc>,
}

/// Event payload passed to durable job store implementations before sequencing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobEventDraft {
	/// Job identifier.
	pub job_id: JobId,
	/// Event kind.
	pub kind: JobEventKind,
	/// Previous state, if this event represents a transition.
	pub from_state: Option<JobState>,
	/// New state after the event.
	pub to_state: JobState,
	/// Optional human-readable message.
	pub message: Option<String>,
	/// Event creation timestamp.
	pub created_at: DateTime<Utc>,
}

/// Conflict returned for illegal lifecycle transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTransitionConflict {
	/// Job identifier.
	pub job_id: JobId,
	/// Current state.
	pub from: JobState,
	/// Requested next state.
	pub to: JobState,
	/// Human-readable reason.
	pub reason: String,
}

impl fmt::Display for JobTransitionConflict {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"illegal transition for job {}: {} -> {} ({})",
			self.job_id, self.from, self.to, self.reason
		)
	}
}

/// Errors returned by durable queue operations.
#[derive(Debug, Error)]
pub enum DurableQueueError {
	/// Requested job was not found.
	#[error("durable job not found: {0}")]
	NotFound(JobId),
	/// The requested lifecycle transition is illegal.
	#[error("{0}")]
	Conflict(JobTransitionConflict),
	/// Stored job state is unknown.
	#[error("invalid durable job state: {0}")]
	InvalidState(String),
	/// Stored event kind is unknown.
	#[error("invalid durable job event kind: {0}")]
	InvalidEventKind(String),
	/// Serialization failed.
	#[error("durable job serialization failed: {0}")]
	Serialization(#[from] serde_json::Error),
	/// Database operation failed.
	#[error("durable job database operation failed: {0}")]
	Database(#[from] sqlx::Error),
	/// Store implementation failed.
	#[error("durable job store failed: {0}")]
	Store(String),
}

/// State machine for durable job lifecycle transitions.
#[derive(Debug, Clone, Copy, Default)]
pub struct JobLifecycleService;

impl JobLifecycleService {
	/// Applies a legal lifecycle transition to a job record.
	pub fn transition(
		&self,
		record: &mut DurableJobRecord,
		to: JobState,
		now: DateTime<Utc>,
	) -> Result<JobEventKind, DurableQueueError> {
		let from = record.state;
		if !from.can_transition_to(to) {
			return Err(DurableQueueError::Conflict(JobTransitionConflict {
				job_id: record.id,
				from,
				to,
				reason: "transition is not allowed by the durable job lifecycle".to_string(),
			}));
		}

		record.state = to;
		record.updated_at = now;

		match to {
			JobState::Queued => {
				record.retry_after = None;
				record.started_at = None;
				record.lease_expires_at = None;
				record.finished_at = None;
			}
			JobState::Running => {
				record.started_at = Some(now);
			}
			JobState::Succeeded | JobState::FailedFinal | JobState::Canceled => {
				record.lease_expires_at = None;
				record.finished_at = Some(now);
			}
			JobState::FailedRetryable => {
				record.lease_expires_at = None;
			}
		}

		Ok(match to {
			JobState::Queued => JobEventKind::Retried,
			JobState::Running => JobEventKind::Claimed,
			JobState::Succeeded => JobEventKind::Succeeded,
			JobState::FailedRetryable => JobEventKind::FailedRetryable,
			JobState::FailedFinal => JobEventKind::FailedFinal,
			JobState::Canceled => JobEventKind::Canceled,
		})
	}
}

/// Storage abstraction for durable job records and events.
#[async_trait]
pub trait DurableJobStore: Send + Sync {
	/// Inserts a new durable job record.
	async fn insert_job(&self, record: DurableJobRecord) -> Result<(), DurableQueueError>;

	/// Inserts a new durable job record and its initial lifecycle event atomically.
	async fn insert_job_with_event(
		&self,
		record: DurableJobRecord,
		event: JobEventDraft,
	) -> Result<JobEvent, DurableQueueError>;

	/// Returns a durable job record by ID.
	async fn get_job(&self, job_id: JobId) -> Result<Option<DurableJobRecord>, DurableQueueError>;

	/// Updates a durable job record.
	async fn update_job(&self, record: DurableJobRecord) -> Result<(), DurableQueueError>;

	/// Updates a durable job only if it still matches the expected state and attempt.
	async fn update_job_if_current(
		&self,
		record: DurableJobRecord,
		expected_state: JobState,
		expected_attempt_count: u32,
	) -> Result<bool, DurableQueueError>;

	/// Atomically claims the next queued job for the named queue.
	async fn claim_next(
		&self,
		queue: &str,
		now: DateTime<Utc>,
		lease_expires_at: DateTime<Utc>,
	) -> Result<Option<ClaimedJobRecord>, DurableQueueError>;

	/// Appends a lifecycle event and assigns the next per-job sequence number.
	async fn append_event(&self, event: JobEventDraft) -> Result<JobEvent, DurableQueueError>;

	/// Lists lifecycle events for a job in sequence order.
	async fn list_events(&self, job_id: JobId) -> Result<Vec<JobEvent>, DurableQueueError>;
}

#[async_trait]
impl<T> DurableJobStore for Arc<T>
where
	T: DurableJobStore + ?Sized,
{
	async fn insert_job(&self, record: DurableJobRecord) -> Result<(), DurableQueueError> {
		(**self).insert_job(record).await
	}

	async fn insert_job_with_event(
		&self,
		record: DurableJobRecord,
		event: JobEventDraft,
	) -> Result<JobEvent, DurableQueueError> {
		(**self).insert_job_with_event(record, event).await
	}

	async fn get_job(&self, job_id: JobId) -> Result<Option<DurableJobRecord>, DurableQueueError> {
		(**self).get_job(job_id).await
	}

	async fn update_job(&self, record: DurableJobRecord) -> Result<(), DurableQueueError> {
		(**self).update_job(record).await
	}

	async fn update_job_if_current(
		&self,
		record: DurableJobRecord,
		expected_state: JobState,
		expected_attempt_count: u32,
	) -> Result<bool, DurableQueueError> {
		(**self)
			.update_job_if_current(record, expected_state, expected_attempt_count)
			.await
	}

	async fn claim_next(
		&self,
		queue: &str,
		now: DateTime<Utc>,
		lease_expires_at: DateTime<Utc>,
	) -> Result<Option<ClaimedJobRecord>, DurableQueueError> {
		(**self).claim_next(queue, now, lease_expires_at).await
	}

	async fn append_event(&self, event: JobEventDraft) -> Result<JobEvent, DurableQueueError> {
		(**self).append_event(event).await
	}

	async fn list_events(&self, job_id: JobId) -> Result<Vec<JobEvent>, DurableQueueError> {
		(**self).list_events(job_id).await
	}
}

/// Job record returned by an atomic claim operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimedJobRecord {
	/// Previous state before the claim update.
	pub previous_state: JobState,
	/// Running job record after the claim update.
	pub record: DurableJobRecord,
}

/// Lifecycle event publisher for durable jobs.
#[derive(Debug, Clone)]
pub struct JobEventPublisher<S> {
	store: S,
}

impl<S> JobEventPublisher<S>
where
	S: DurableJobStore,
{
	/// Creates a publisher backed by the given store.
	pub fn new(store: S) -> Self {
		Self { store }
	}

	/// Publishes an enqueue event.
	pub async fn publish_enqueued(
		&self,
		record: &DurableJobRecord,
		now: DateTime<Utc>,
	) -> Result<JobEvent, DurableQueueError> {
		self.store
			.append_event(JobEventDraft {
				job_id: record.id,
				kind: JobEventKind::Enqueued,
				from_state: None,
				to_state: record.state,
				message: None,
				created_at: now,
			})
			.await
	}

	/// Publishes a lifecycle transition event.
	pub async fn publish_transition(
		&self,
		record: &DurableJobRecord,
		kind: JobEventKind,
		from_state: JobState,
		message: Option<String>,
		now: DateTime<Utc>,
	) -> Result<JobEvent, DurableQueueError> {
		self.store
			.append_event(JobEventDraft {
				job_id: record.id,
				kind,
				from_state: Some(from_state),
				to_state: record.state,
				message,
				created_at: now,
			})
			.await
	}
}

/// Durable job queue service.
#[derive(Debug, Clone)]
pub struct DurableQueue<S> {
	store: S,
	lifecycle: JobLifecycleService,
	publisher: JobEventPublisher<S>,
	retry_strategy: RetryStrategy,
	default_queue: String,
	claim_lease: Duration,
}

impl<S> DurableQueue<S>
where
	S: DurableJobStore + Clone,
{
	/// Creates a durable queue backed by the given store.
	pub fn new(store: S) -> Self {
		Self {
			publisher: JobEventPublisher::new(store.clone()),
			store,
			lifecycle: JobLifecycleService,
			retry_strategy: RetryStrategy::exponential_backoff(),
			default_queue: DEFAULT_DURABLE_QUEUE_NAME.to_string(),
			claim_lease: DEFAULT_CLAIM_LEASE,
		}
	}

	/// Overrides the default retry strategy.
	pub fn with_retry_strategy(mut self, retry_strategy: RetryStrategy) -> Self {
		self.retry_strategy = retry_strategy;
		self
	}

	/// Overrides the default queue used by [`Self::claim_next`].
	pub fn with_default_queue(mut self, queue: impl Into<String>) -> Self {
		self.default_queue = queue.into();
		self
	}

	/// Overrides how long a running claim may be idle before another worker can reclaim it.
	pub fn with_claim_lease(mut self, claim_lease: Duration) -> Self {
		self.claim_lease = claim_lease.max(Duration::from_millis(1));
		self
	}

	/// Enqueues a durable job.
	pub async fn enqueue(&self, spec: JobSpec) -> Result<JobSnapshot, DurableQueueError> {
		let now = Utc::now();
		let record = spec.into_record(now);
		self.store
			.insert_job_with_event(
				record.clone(),
				JobEventDraft {
					job_id: record.id,
					kind: JobEventKind::Enqueued,
					from_state: None,
					to_state: record.state,
					message: None,
					created_at: now,
				},
			)
			.await?;
		Ok(record.snapshot())
	}

	/// Returns a job snapshot by ID.
	pub async fn status(&self, job_id: JobId) -> Result<JobSnapshot, DurableQueueError> {
		self.load_job(job_id).await.map(|job| job.snapshot())
	}

	/// Atomically claims the next job from the default queue.
	pub async fn claim_next(&self) -> Result<Option<JobClaim>, DurableQueueError> {
		self.claim_next_from(&self.default_queue).await
	}

	/// Atomically claims the next job from the named queue.
	pub async fn claim_next_from(
		&self,
		queue: &str,
	) -> Result<Option<JobClaim>, DurableQueueError> {
		let now = Utc::now();
		let lease_expires_at = now + duration_to_chrono(self.claim_lease);
		let Some(claimed) = self.store.claim_next(queue, now, lease_expires_at).await? else {
			return Ok(None);
		};
		self.publisher
			.publish_transition(
				&claimed.record,
				JobEventKind::Claimed,
				claimed.previous_state,
				None,
				now,
			)
			.await?;
		Ok(Some(JobClaim::new(claimed.record)?))
	}

	/// Marks a claimed job as succeeded.
	pub async fn succeed<T: Serialize>(
		&self,
		claim: JobClaim,
		result: &T,
	) -> Result<JobSnapshot, DurableQueueError> {
		let mut record = self.load_running_claim(&claim).await?;
		let now = Utc::now();
		let from = record.state;
		let expected_attempt_count = record.attempt_count;
		let event_kind = self
			.lifecycle
			.transition(&mut record, JobState::Succeeded, now)?;
		record.result = Some(serde_json::to_value(result)?);
		record.failure_kind = None;
		record.failure_message = None;
		record.retry_after = None;
		self.update_job_if_current(record.clone(), from, expected_attempt_count)
			.await?;
		self.publisher
			.publish_transition(&record, event_kind, from, None, now)
			.await?;
		Ok(record.snapshot())
	}

	/// Marks a claimed job as failed and retryable, or final when attempts are exhausted.
	pub async fn fail_retryable(
		&self,
		claim: JobClaim,
		failure: &JobFailure,
	) -> Result<JobSnapshot, DurableQueueError> {
		let mut record = self.load_running_claim(&claim).await?;
		let now = Utc::now();
		let from = record.state;
		let expected_attempt_count = record.attempt_count;
		let retry_attempts_so_far = record.attempt_count.saturating_sub(1);
		let should_retry = record.attempt_count < record.max_attempts
			&& self.retry_strategy.should_retry(retry_attempts_so_far);
		let next_state = if should_retry {
			JobState::FailedRetryable
		} else {
			JobState::FailedFinal
		};
		let event_kind = self.lifecycle.transition(&mut record, next_state, now)?;
		record.failure_kind = Some(failure.kind.clone());
		record.failure_message = Some(failure.message.clone());
		record.result = None;
		record.retry_after = if next_state == JobState::FailedRetryable {
			Some(
				now + duration_to_chrono(self.retry_strategy.calculate_delay(record.attempt_count)),
			)
		} else {
			None
		};
		self.update_job_if_current(record.clone(), from, expected_attempt_count)
			.await?;
		self.publisher
			.publish_transition(
				&record,
				event_kind,
				from,
				Some(failure.message.clone()),
				now,
			)
			.await?;
		Ok(record.snapshot())
	}

	/// Marks a claimed job as permanently failed.
	pub async fn fail_final(
		&self,
		claim: JobClaim,
		failure: &JobFailure,
	) -> Result<JobSnapshot, DurableQueueError> {
		let mut record = self.load_running_claim(&claim).await?;
		let now = Utc::now();
		let from = record.state;
		let expected_attempt_count = record.attempt_count;
		let event_kind = self
			.lifecycle
			.transition(&mut record, JobState::FailedFinal, now)?;
		record.failure_kind = Some(failure.kind.clone());
		record.failure_message = Some(failure.message.clone());
		record.result = None;
		record.retry_after = None;
		self.update_job_if_current(record.clone(), from, expected_attempt_count)
			.await?;
		self.publisher
			.publish_transition(
				&record,
				event_kind,
				from,
				Some(failure.message.clone()),
				now,
			)
			.await?;
		Ok(record.snapshot())
	}

	/// Requests cancellation for a job.
	pub async fn request_cancel(&self, job_id: JobId) -> Result<JobSnapshot, DurableQueueError> {
		let mut record = self.load_job(job_id).await?;
		let now = Utc::now();

		if record.state.is_terminal() {
			return Err(DurableQueueError::Conflict(JobTransitionConflict {
				job_id,
				from: record.state,
				to: JobState::Canceled,
				reason: "terminal jobs cannot be canceled".to_string(),
			}));
		}

		let from = record.state;
		let expected_attempt_count = record.attempt_count;
		record.cancellation_requested = true;
		record.updated_at = now;

		if matches!(record.state, JobState::Queued | JobState::FailedRetryable) {
			let event_kind = self
				.lifecycle
				.transition(&mut record, JobState::Canceled, now)?;
			self.update_job_if_current(record.clone(), from, expected_attempt_count)
				.await?;
			self.publisher
				.publish_transition(&record, event_kind, from, None, now)
				.await?;
		} else {
			self.update_job_if_current(record.clone(), from, expected_attempt_count)
				.await?;
			self.publisher
				.publish_transition(
					&record,
					JobEventKind::CancellationRequested,
					from,
					None,
					now,
				)
				.await?;
		}

		Ok(record.snapshot())
	}

	/// Marks a claimed running job as canceled.
	pub async fn cancel(&self, claim: JobClaim) -> Result<JobSnapshot, DurableQueueError> {
		let mut record = self.load_running_claim(&claim).await?;
		let now = Utc::now();
		let from = record.state;
		let expected_attempt_count = record.attempt_count;
		let event_kind = self
			.lifecycle
			.transition(&mut record, JobState::Canceled, now)?;
		record.cancellation_requested = true;
		record.retry_after = None;
		self.update_job_if_current(record.clone(), from, expected_attempt_count)
			.await?;
		self.publisher
			.publish_transition(&record, event_kind, from, None, now)
			.await?;
		Ok(record.snapshot())
	}

	/// Requeues a failed-retryable job.
	pub async fn retry(&self, job_id: JobId) -> Result<JobSnapshot, DurableQueueError> {
		let mut record = self.load_job(job_id).await?;
		let now = Utc::now();
		let from = record.state;
		let expected_attempt_count = record.attempt_count;
		let event_kind = self
			.lifecycle
			.transition(&mut record, JobState::Queued, now)?;
		self.update_job_if_current(record.clone(), from, expected_attempt_count)
			.await?;
		self.publisher
			.publish_transition(&record, event_kind, from, None, now)
			.await?;
		Ok(record.snapshot())
	}

	/// Lists lifecycle events for a job.
	pub async fn events(&self, job_id: JobId) -> Result<Vec<JobEvent>, DurableQueueError> {
		self.store.list_events(job_id).await
	}

	async fn load_job(&self, job_id: JobId) -> Result<DurableJobRecord, DurableQueueError> {
		self.store
			.get_job(job_id)
			.await?
			.ok_or(DurableQueueError::NotFound(job_id))
	}

	async fn load_running_claim(
		&self,
		claim: &JobClaim,
	) -> Result<DurableJobRecord, DurableQueueError> {
		let record = self.load_job(claim.id()).await?;
		if record.state != JobState::Running {
			return Err(DurableQueueError::Conflict(JobTransitionConflict {
				job_id: record.id,
				from: record.state,
				to: JobState::Running,
				reason: "claimed job is no longer running".to_string(),
			}));
		}
		if record.attempt_count != claim.record.attempt_count {
			return Err(DurableQueueError::Conflict(JobTransitionConflict {
				job_id: record.id,
				from: record.state,
				to: record.state,
				reason: "claim attempt no longer matches the stored job".to_string(),
			}));
		}
		Ok(record)
	}

	async fn update_job_if_current(
		&self,
		record: DurableJobRecord,
		expected_state: JobState,
		expected_attempt_count: u32,
	) -> Result<(), DurableQueueError> {
		if self
			.store
			.update_job_if_current(record.clone(), expected_state, expected_attempt_count)
			.await?
		{
			Ok(())
		} else {
			Err(DurableQueueError::Conflict(JobTransitionConflict {
				job_id: record.id,
				from: expected_state,
				to: record.state,
				reason: "job changed before the lifecycle update could be committed".to_string(),
			}))
		}
	}
}

/// SQLite durable job store.
#[derive(Debug, Clone)]
pub struct SqliteDurableJobStore {
	pool: SqlitePool,
}

impl SqliteDurableJobStore {
	/// Opens a SQLite durable job store and creates required tables.
	pub async fn new(database_url: &str) -> Result<Self, DurableQueueError> {
		let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
		let pool = SqlitePoolOptions::new()
			.max_connections(1)
			.connect_with(options)
			.await?;
		let store = Self { pool };
		store.create_tables().await?;
		Ok(store)
	}

	/// Creates a store from an existing SQLite pool and creates required tables.
	pub async fn from_pool(pool: SqlitePool) -> Result<Self, DurableQueueError> {
		let store = Self { pool };
		store.reject_private_in_memory_pool().await?;
		store.create_tables().await?;
		Ok(store)
	}

	/// Returns the underlying SQLite pool.
	pub fn pool(&self) -> &SqlitePool {
		&self.pool
	}

	async fn reject_private_in_memory_pool(&self) -> Result<(), DurableQueueError> {
		let rows = sqlx::query("PRAGMA database_list")
			.fetch_all(&self.pool)
			.await?;
		let is_in_memory = rows.iter().any(|row| {
			let name: String = row.get("name");
			let file: String = row.get("file");
			name == "main" && file.is_empty()
		});

		if is_in_memory {
			return Err(DurableQueueError::Store(
				"SqliteDurableJobStore::from_pool cannot safely use private in-memory SQLite pools; use SqliteDurableJobStore::new(\"sqlite::memory:\") or a shared/file-backed database"
					.to_string(),
			));
		}

		Ok(())
	}

	async fn create_tables(&self) -> Result<(), DurableQueueError> {
		sqlx::query(
			r#"
			CREATE TABLE IF NOT EXISTS durable_jobs (
				id TEXT PRIMARY KEY,
				queue TEXT NOT NULL,
				kind TEXT NOT NULL,
				target TEXT,
				state TEXT NOT NULL,
				attempt_count INTEGER NOT NULL,
				max_attempts INTEGER NOT NULL,
				priority INTEGER NOT NULL,
				payload TEXT NOT NULL,
				result TEXT,
				failure_kind TEXT,
				failure_message TEXT,
				retry_after INTEGER,
				cancellation_requested INTEGER NOT NULL,
				created_at INTEGER NOT NULL,
				updated_at INTEGER NOT NULL,
				started_at INTEGER,
				lease_expires_at INTEGER,
				finished_at INTEGER
			)
			"#,
		)
		.execute(&self.pool)
		.await?;

		sqlx::query(
			r#"
			CREATE INDEX IF NOT EXISTS durable_jobs_claim_idx
			ON durable_jobs (queue, state, priority DESC, retry_after, created_at)
			"#,
		)
		.execute(&self.pool)
		.await?;

		sqlx::query(
			r#"
			CREATE TABLE IF NOT EXISTS durable_job_events (
				job_id TEXT NOT NULL,
				event_sequence INTEGER NOT NULL,
				event_kind TEXT NOT NULL,
				from_state TEXT,
				to_state TEXT NOT NULL,
				message TEXT,
				created_at INTEGER NOT NULL,
				PRIMARY KEY (job_id, event_sequence)
			)
			"#,
		)
		.execute(&self.pool)
		.await?;

		Ok(())
	}
}

#[async_trait]
impl DurableJobStore for SqliteDurableJobStore {
	async fn insert_job(&self, record: DurableJobRecord) -> Result<(), DurableQueueError> {
		sqlx::query(
			r#"
			INSERT INTO durable_jobs (
				id, queue, kind, target, state, attempt_count, max_attempts, priority,
				payload, result, failure_kind, failure_message, retry_after,
				cancellation_requested, created_at, updated_at, started_at, lease_expires_at,
				finished_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(record.id.to_string())
		.bind(record.queue)
		.bind(record.kind)
		.bind(record.target)
		.bind(record.state.as_str())
		.bind(i64::from(record.attempt_count))
		.bind(i64::from(record.max_attempts))
		.bind(i64::from(record.priority))
		.bind(serde_json::to_string(&record.payload)?)
		.bind(
			record
				.result
				.map(|result| serde_json::to_string(&result))
				.transpose()?,
		)
		.bind(record.failure_kind)
		.bind(record.failure_message)
		.bind(record.retry_after.map(timestamp_millis))
		.bind(if record.cancellation_requested {
			1_i64
		} else {
			0_i64
		})
		.bind(timestamp_millis(record.created_at))
		.bind(timestamp_millis(record.updated_at))
		.bind(record.started_at.map(timestamp_millis))
		.bind(record.lease_expires_at.map(timestamp_millis))
		.bind(record.finished_at.map(timestamp_millis))
		.execute(&self.pool)
		.await?;
		Ok(())
	}

	async fn insert_job_with_event(
		&self,
		record: DurableJobRecord,
		event: JobEventDraft,
	) -> Result<JobEvent, DurableQueueError> {
		let mut tx = self.pool.begin().await?;

		sqlx::query(
			r#"
			INSERT INTO durable_jobs (
				id, queue, kind, target, state, attempt_count, max_attempts, priority,
				payload, result, failure_kind, failure_message, retry_after,
				cancellation_requested, created_at, updated_at, started_at, lease_expires_at,
				finished_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(record.id.to_string())
		.bind(record.queue)
		.bind(record.kind)
		.bind(record.target)
		.bind(record.state.as_str())
		.bind(i64::from(record.attempt_count))
		.bind(i64::from(record.max_attempts))
		.bind(i64::from(record.priority))
		.bind(serde_json::to_string(&record.payload)?)
		.bind(
			record
				.result
				.map(|result| serde_json::to_string(&result))
				.transpose()?,
		)
		.bind(record.failure_kind)
		.bind(record.failure_message)
		.bind(record.retry_after.map(timestamp_millis))
		.bind(if record.cancellation_requested {
			1_i64
		} else {
			0_i64
		})
		.bind(timestamp_millis(record.created_at))
		.bind(timestamp_millis(record.updated_at))
		.bind(record.started_at.map(timestamp_millis))
		.bind(record.lease_expires_at.map(timestamp_millis))
		.bind(record.finished_at.map(timestamp_millis))
		.execute(&mut *tx)
		.await?;

		let event = insert_event_in_tx(&mut tx, event).await?;
		tx.commit().await?;
		Ok(event)
	}

	async fn get_job(&self, job_id: JobId) -> Result<Option<DurableJobRecord>, DurableQueueError> {
		let row = sqlx::query("SELECT * FROM durable_jobs WHERE id = ?")
			.bind(job_id.to_string())
			.fetch_optional(&self.pool)
			.await?;
		row.map(row_to_record).transpose()
	}

	async fn update_job(&self, record: DurableJobRecord) -> Result<(), DurableQueueError> {
		let result = sqlx::query(
			r#"
			UPDATE durable_jobs SET
				queue = ?,
				kind = ?,
				target = ?,
				state = ?,
				attempt_count = ?,
				max_attempts = ?,
				priority = ?,
				payload = ?,
				result = ?,
				failure_kind = ?,
				failure_message = ?,
				retry_after = ?,
				cancellation_requested = ?,
				created_at = ?,
				updated_at = ?,
				started_at = ?,
				lease_expires_at = ?,
				finished_at = ?
			WHERE id = ?
			"#,
		)
		.bind(record.queue)
		.bind(record.kind)
		.bind(record.target)
		.bind(record.state.as_str())
		.bind(i64::from(record.attempt_count))
		.bind(i64::from(record.max_attempts))
		.bind(i64::from(record.priority))
		.bind(serde_json::to_string(&record.payload)?)
		.bind(
			record
				.result
				.map(|result| serde_json::to_string(&result))
				.transpose()?,
		)
		.bind(record.failure_kind)
		.bind(record.failure_message)
		.bind(record.retry_after.map(timestamp_millis))
		.bind(if record.cancellation_requested {
			1_i64
		} else {
			0_i64
		})
		.bind(timestamp_millis(record.created_at))
		.bind(timestamp_millis(record.updated_at))
		.bind(record.started_at.map(timestamp_millis))
		.bind(record.lease_expires_at.map(timestamp_millis))
		.bind(record.finished_at.map(timestamp_millis))
		.bind(record.id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			Err(DurableQueueError::NotFound(record.id))
		} else {
			Ok(())
		}
	}

	async fn update_job_if_current(
		&self,
		record: DurableJobRecord,
		expected_state: JobState,
		expected_attempt_count: u32,
	) -> Result<bool, DurableQueueError> {
		let result = sqlx::query(
			r#"
			UPDATE durable_jobs SET
				queue = ?,
				kind = ?,
				target = ?,
				state = ?,
				attempt_count = ?,
				max_attempts = ?,
				priority = ?,
				payload = ?,
				result = ?,
				failure_kind = ?,
				failure_message = ?,
				retry_after = ?,
				cancellation_requested = ?,
				created_at = ?,
				updated_at = ?,
				started_at = ?,
				lease_expires_at = ?,
				finished_at = ?
			WHERE id = ? AND state = ? AND attempt_count = ?
			"#,
		)
		.bind(record.queue)
		.bind(record.kind)
		.bind(record.target)
		.bind(record.state.as_str())
		.bind(i64::from(record.attempt_count))
		.bind(i64::from(record.max_attempts))
		.bind(i64::from(record.priority))
		.bind(serde_json::to_string(&record.payload)?)
		.bind(
			record
				.result
				.map(|result| serde_json::to_string(&result))
				.transpose()?,
		)
		.bind(record.failure_kind)
		.bind(record.failure_message)
		.bind(record.retry_after.map(timestamp_millis))
		.bind(if record.cancellation_requested {
			1_i64
		} else {
			0_i64
		})
		.bind(timestamp_millis(record.created_at))
		.bind(timestamp_millis(record.updated_at))
		.bind(record.started_at.map(timestamp_millis))
		.bind(record.lease_expires_at.map(timestamp_millis))
		.bind(record.finished_at.map(timestamp_millis))
		.bind(record.id.to_string())
		.bind(expected_state.as_str())
		.bind(i64::from(expected_attempt_count))
		.execute(&self.pool)
		.await?;

		Ok(result.rows_affected() > 0)
	}

	async fn claim_next(
		&self,
		queue: &str,
		now: DateTime<Utc>,
		lease_expires_at: DateTime<Utc>,
	) -> Result<Option<ClaimedJobRecord>, DurableQueueError> {
		let mut tx = self.pool.begin().await?;
		let now_millis = timestamp_millis(now);
		let lease_expires_millis = timestamp_millis(lease_expires_at);

		let expired_final_candidate: Option<String> = sqlx::query_scalar(
			r#"
			SELECT id
			FROM durable_jobs
			WHERE queue = ?
			  AND state = ?
			  AND cancellation_requested = 0
			  AND lease_expires_at IS NOT NULL
			  AND lease_expires_at <= ?
			  AND attempt_count >= max_attempts
			ORDER BY priority DESC, created_at ASC
			LIMIT 1
			"#,
		)
		.bind(queue)
		.bind(JobState::Running.as_str())
		.bind(now_millis)
		.fetch_optional(&mut *tx)
		.await?;

		if let Some(expired_job_id) = expired_final_candidate {
			let final_message = "job claim lease expired after maximum attempts";
			let result = sqlx::query(
				r#"
				UPDATE durable_jobs
				SET state = ?,
					failure_kind = ?,
					failure_message = ?,
					retry_after = NULL,
					lease_expires_at = NULL,
					finished_at = ?,
					updated_at = ?
				WHERE id = ?
				  AND queue = ?
				  AND state = ?
				  AND lease_expires_at IS NOT NULL
				  AND lease_expires_at <= ?
				  AND attempt_count >= max_attempts
				"#,
			)
			.bind(JobState::FailedFinal.as_str())
			.bind("claim_lease_expired")
			.bind(final_message)
			.bind(now_millis)
			.bind(now_millis)
			.bind(&expired_job_id)
			.bind(queue)
			.bind(JobState::Running.as_str())
			.bind(now_millis)
			.execute(&mut *tx)
			.await?;

			if result.rows_affected() > 0 {
				let job_id = expired_job_id
					.parse()
					.map_err(|error: uuid::Error| DurableQueueError::Store(error.to_string()))?;
				insert_event_in_tx(
					&mut tx,
					JobEventDraft {
						job_id,
						kind: JobEventKind::FailedFinal,
						from_state: Some(JobState::Running),
						to_state: JobState::FailedFinal,
						message: Some(final_message.to_string()),
						created_at: now,
					},
				)
				.await?;
			}
		}

		let candidate: Option<(String, String)> = sqlx::query_as(
			r#"
			SELECT id, state
			FROM durable_jobs
			WHERE queue = ?
			  AND cancellation_requested = 0
			  AND (
				(state = ? AND (retry_after IS NULL OR retry_after <= ?))
				OR (state = ? AND retry_after IS NOT NULL AND retry_after <= ?)
				OR (
					state = ?
					AND lease_expires_at IS NOT NULL
					AND lease_expires_at <= ?
					AND attempt_count < max_attempts
				)
			  )
			ORDER BY priority DESC, created_at ASC
			LIMIT 1
			"#,
		)
		.bind(queue)
		.bind(JobState::Queued.as_str())
		.bind(now_millis)
		.bind(JobState::FailedRetryable.as_str())
		.bind(now_millis)
		.bind(JobState::Running.as_str())
		.bind(now_millis)
		.fetch_optional(&mut *tx)
		.await?;

		let Some((job_id, previous_state)) = candidate else {
			tx.commit().await?;
			return Ok(None);
		};
		let previous_state = JobState::from_str(&previous_state)?;

		let result = sqlx::query(
			r#"
			UPDATE durable_jobs
			SET state = ?,
				attempt_count = attempt_count + 1,
				retry_after = NULL,
				lease_expires_at = ?,
				started_at = ?,
				updated_at = ?
			WHERE id = ? AND state = ?
			"#,
		)
		.bind(JobState::Running.as_str())
		.bind(lease_expires_millis)
		.bind(now_millis)
		.bind(now_millis)
		.bind(&job_id)
		.bind(previous_state.as_str())
		.execute(&mut *tx)
		.await?;

		if result.rows_affected() == 0 {
			tx.commit().await?;
			return Ok(None);
		}

		let row = sqlx::query("SELECT * FROM durable_jobs WHERE id = ?")
			.bind(&job_id)
			.fetch_one(&mut *tx)
			.await?;
		tx.commit().await?;
		Ok(Some(ClaimedJobRecord {
			previous_state,
			record: row_to_record(row)?,
		}))
	}

	async fn append_event(&self, event: JobEventDraft) -> Result<JobEvent, DurableQueueError> {
		let mut tx = self.pool.begin().await?;
		let event = insert_event_in_tx(&mut tx, event).await?;
		tx.commit().await?;
		Ok(event)
	}

	async fn list_events(&self, job_id: JobId) -> Result<Vec<JobEvent>, DurableQueueError> {
		let rows = sqlx::query(
			r#"
			SELECT *
			FROM durable_job_events
			WHERE job_id = ?
			ORDER BY event_sequence ASC
			"#,
		)
		.bind(job_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(row_to_event).collect()
	}
}

fn row_to_record(row: SqliteRow) -> Result<DurableJobRecord, DurableQueueError> {
	let id: String = row.get("id");
	let state: String = row.get("state");
	let payload: String = row.get("payload");
	let result: Option<String> = row.get("result");
	let attempt_count: i64 = row.get("attempt_count");
	let max_attempts: i64 = row.get("max_attempts");
	let priority: i64 = row.get("priority");
	let cancellation_requested: i64 = row.get("cancellation_requested");

	Ok(DurableJobRecord {
		id: id
			.parse()
			.map_err(|error: uuid::Error| DurableQueueError::Store(error.to_string()))?,
		queue: row.get("queue"),
		kind: row.get("kind"),
		target: row.get("target"),
		state: JobState::from_str(&state)?,
		attempt_count: attempt_count as u32,
		max_attempts: max_attempts as u32,
		priority: priority as i32,
		payload: serde_json::from_str(&payload)?,
		result: result
			.map(|stored| serde_json::from_str(&stored))
			.transpose()?,
		failure_kind: row.get("failure_kind"),
		failure_message: row.get("failure_message"),
		retry_after: optional_timestamp(row.get("retry_after"))?,
		cancellation_requested: cancellation_requested != 0,
		created_at: required_timestamp(row.get("created_at"))?,
		updated_at: required_timestamp(row.get("updated_at"))?,
		started_at: optional_timestamp(row.get("started_at"))?,
		lease_expires_at: optional_timestamp(row.get("lease_expires_at"))?,
		finished_at: optional_timestamp(row.get("finished_at"))?,
	})
}

async fn insert_event_in_tx(
	tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
	event: JobEventDraft,
) -> Result<JobEvent, DurableQueueError> {
	let sequence: i64 = sqlx::query_scalar(
		r#"
		SELECT COALESCE(MAX(event_sequence), 0) + 1
		FROM durable_job_events
		WHERE job_id = ?
		"#,
	)
	.bind(event.job_id.to_string())
	.fetch_one(&mut **tx)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO durable_job_events (
			job_id, event_sequence, event_kind, from_state, to_state, message, created_at
		)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		"#,
	)
	.bind(event.job_id.to_string())
	.bind(sequence)
	.bind(event.kind.as_str())
	.bind(event.from_state.map(JobState::as_str))
	.bind(event.to_state.as_str())
	.bind(event.message.as_deref())
	.bind(timestamp_millis(event.created_at))
	.execute(&mut **tx)
	.await?;

	Ok(JobEvent {
		job_id: event.job_id,
		sequence: sequence as u64,
		kind: event.kind,
		from_state: event.from_state,
		to_state: event.to_state,
		message: event.message,
		created_at: event.created_at,
	})
}

fn row_to_event(row: SqliteRow) -> Result<JobEvent, DurableQueueError> {
	let job_id: String = row.get("job_id");
	let event_kind: String = row.get("event_kind");
	let from_state: Option<String> = row.get("from_state");
	let to_state: String = row.get("to_state");
	let sequence: i64 = row.get("event_sequence");

	Ok(JobEvent {
		job_id: job_id
			.parse()
			.map_err(|error: uuid::Error| DurableQueueError::Store(error.to_string()))?,
		sequence: sequence as u64,
		kind: JobEventKind::from_str(&event_kind)?,
		from_state: from_state
			.map(|state| JobState::from_str(&state))
			.transpose()?,
		to_state: JobState::from_str(&to_state)?,
		message: row.get("message"),
		created_at: required_timestamp(row.get("created_at"))?,
	})
}

fn timestamp_millis(timestamp: DateTime<Utc>) -> i64 {
	timestamp.timestamp_millis()
}

fn required_timestamp(value: i64) -> Result<DateTime<Utc>, DurableQueueError> {
	DateTime::from_timestamp_millis(value)
		.ok_or_else(|| DurableQueueError::Store(format!("invalid timestamp millis: {value}")))
}

fn optional_timestamp(value: Option<i64>) -> Result<Option<DateTime<Utc>>, DurableQueueError> {
	value.map(required_timestamp).transpose()
}

fn duration_to_chrono(duration: Duration) -> chrono::Duration {
	chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::MAX)
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	async fn queue() -> DurableQueue<SqliteDurableJobStore> {
		let store = SqliteDurableJobStore::new("sqlite::memory:").await.unwrap();
		DurableQueue::new(store).with_retry_strategy(RetryStrategy::fixed_delay(Duration::ZERO))
	}

	#[tokio::test]
	async fn enqueue_claim_and_succeed_persists_snapshot_and_events() {
		let queue = queue().await;

		let enqueued = queue
			.enqueue(
				JobSpec::new("manuscript_export")
					.target("project-1")
					.max_attempts(2)
					.payload(&json!({"chapter": 1}))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(enqueued.state, JobState::Queued);
		assert_eq!(enqueued.attempt_count, 0);

		let claim = queue.claim_next().await.unwrap().unwrap();
		assert_eq!(claim.payload::<Value>().unwrap(), json!({"chapter": 1}));

		let running = queue.status(enqueued.id).await.unwrap();
		assert_eq!(running.state, JobState::Running);
		assert_eq!(running.attempt_count, 1);

		let succeeded = queue.succeed(claim, &json!({"ok": true})).await.unwrap();
		assert_eq!(succeeded.state, JobState::Succeeded);
		assert_eq!(succeeded.result, Some(json!({"ok": true})));

		let events = queue.events(enqueued.id).await.unwrap();
		let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
		assert_eq!(
			kinds,
			vec![
				JobEventKind::Enqueued,
				JobEventKind::Claimed,
				JobEventKind::Succeeded
			]
		);
		assert_eq!(events[0].sequence, 1);
		assert_eq!(events[2].sequence, 3);
	}

	#[tokio::test]
	async fn retryable_failure_requeues_until_attempts_are_exhausted() {
		let queue = queue().await;
		let enqueued = queue
			.enqueue(JobSpec::new("index_document").max_attempts(2))
			.await
			.unwrap();

		let first_claim = queue.claim_next().await.unwrap().unwrap();
		let failed = queue
			.fail_retryable(
				first_claim,
				&JobFailure::new("provider_timeout", "try again"),
			)
			.await
			.unwrap();
		assert_eq!(failed.state, JobState::FailedRetryable);
		assert_eq!(failed.failure_kind.as_deref(), Some("provider_timeout"));

		let second_claim = queue.claim_next().await.unwrap().unwrap();
		assert_eq!(second_claim.id(), enqueued.id);
		assert_eq!(second_claim.record().attempt_count, 2);
		let final_failure = queue
			.fail_retryable(
				second_claim,
				&JobFailure::new("provider_timeout", "still down"),
			)
			.await
			.unwrap();
		assert_eq!(final_failure.state, JobState::FailedFinal);
		assert_eq!(final_failure.attempt_count, 2);
		assert_eq!(queue.claim_next().await.unwrap(), None);
	}

	#[tokio::test]
	async fn no_retry_strategy_marks_first_retryable_failure_final() {
		let store = SqliteDurableJobStore::new("sqlite::memory:").await.unwrap();
		let queue = DurableQueue::new(store).with_retry_strategy(RetryStrategy::no_retry());
		let enqueued = queue
			.enqueue(JobSpec::new("index_document").max_attempts(3))
			.await
			.unwrap();

		let claim = queue.claim_next().await.unwrap().unwrap();
		let failed = queue
			.fail_retryable(claim, &JobFailure::new("provider_timeout", "do not retry"))
			.await
			.unwrap();

		assert_eq!(failed.id, enqueued.id);
		assert_eq!(failed.state, JobState::FailedFinal);
		assert_eq!(failed.retry_after, None);
		assert_eq!(queue.claim_next().await.unwrap(), None);
	}

	#[tokio::test]
	async fn expired_running_claim_can_be_reclaimed() {
		let store = SqliteDurableJobStore::new("sqlite::memory:").await.unwrap();
		let queue = DurableQueue::new(store)
			.with_retry_strategy(RetryStrategy::fixed_delay(Duration::ZERO))
			.with_claim_lease(Duration::from_millis(1));
		let enqueued = queue
			.enqueue(JobSpec::new("render_pdf").max_attempts(2))
			.await
			.unwrap();

		let stale_claim = queue.claim_next().await.unwrap().unwrap();
		tokio::time::sleep(Duration::from_millis(5)).await;
		let reclaimed = queue.claim_next().await.unwrap().unwrap();

		assert_eq!(reclaimed.id(), enqueued.id);
		assert_eq!(reclaimed.record().attempt_count, 2);
		assert!(reclaimed.record().lease_expires_at.is_some());

		let stale_error = queue
			.succeed(stale_claim, &json!({"stale": true}))
			.await
			.unwrap_err();
		assert!(matches!(stale_error, DurableQueueError::Conflict(_)));
	}

	#[tokio::test]
	async fn expired_running_claim_at_max_attempts_fails_final() {
		let store = SqliteDurableJobStore::new("sqlite::memory:").await.unwrap();
		let queue = DurableQueue::new(store).with_claim_lease(Duration::from_millis(1));
		let enqueued = queue
			.enqueue(JobSpec::new("render_pdf").max_attempts(1))
			.await
			.unwrap();

		let _claim = queue.claim_next().await.unwrap().unwrap();
		tokio::time::sleep(Duration::from_millis(5)).await;

		assert_eq!(queue.claim_next().await.unwrap(), None);
		let status = queue.status(enqueued.id).await.unwrap();
		assert_eq!(status.state, JobState::FailedFinal);
		assert_eq!(status.failure_kind.as_deref(), Some("claim_lease_expired"));
		assert_eq!(status.lease_expires_at, None);

		let events = queue.events(enqueued.id).await.unwrap();
		let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
		assert_eq!(
			kinds,
			vec![
				JobEventKind::Enqueued,
				JobEventKind::Claimed,
				JobEventKind::FailedFinal
			]
		);
	}

	#[tokio::test]
	async fn from_pool_rejects_private_in_memory_sqlite_pool() {
		let pool = SqlitePoolOptions::new()
			.max_connections(2)
			.connect("sqlite::memory:")
			.await
			.unwrap();

		let error = SqliteDurableJobStore::from_pool(pool).await.unwrap_err();

		assert!(
			matches!(error, DurableQueueError::Store(message) if message.contains("private in-memory SQLite pools"))
		);
	}

	#[tokio::test]
	async fn illegal_transitions_return_conflict_errors() {
		let queue = queue().await;
		let enqueued = queue.enqueue(JobSpec::new("send_email")).await.unwrap();
		let claim = queue.claim_next().await.unwrap().unwrap();
		queue.succeed(claim, &json!({"sent": true})).await.unwrap();

		let error = queue.request_cancel(enqueued.id).await.unwrap_err();
		match error {
			DurableQueueError::Conflict(conflict) => {
				assert_eq!(conflict.from, JobState::Succeeded);
				assert_eq!(conflict.to, JobState::Canceled);
			}
			other => panic!("expected conflict, got {other:?}"),
		}
	}

	#[tokio::test]
	async fn request_cancel_cancels_queued_jobs_and_flags_running_jobs() {
		let queue = queue().await;

		let queued = queue.enqueue(JobSpec::new("queued_job")).await.unwrap();
		let canceled = queue.request_cancel(queued.id).await.unwrap();
		assert_eq!(canceled.state, JobState::Canceled);
		assert!(canceled.cancellation_requested);

		let running = queue.enqueue(JobSpec::new("running_job")).await.unwrap();
		let claim = queue.claim_next().await.unwrap().unwrap();
		let flagged = queue.request_cancel(running.id).await.unwrap();
		assert_eq!(flagged.state, JobState::Running);
		assert!(flagged.cancellation_requested);

		let canceled = queue.cancel(claim).await.unwrap();
		assert_eq!(canceled.state, JobState::Canceled);
	}
}
