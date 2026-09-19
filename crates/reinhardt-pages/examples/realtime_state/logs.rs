use std::collections::VecDeque;
use std::num::NonZeroUsize;

const FIXED_ROW_BYTES: usize = 16;
pub(crate) const MAX_HTTP_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LogRow {
	pub id: Option<u64>,
	pub cursor: Option<u64>,
	pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LogSnapshot {
	pub rows: Vec<LogRow>,
	pub watermark: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LogLimits {
	pub rows: NonZeroUsize,
	pub bytes: NonZeroUsize,
	pub record_bytes: NonZeroUsize,
}

impl Default for LogLimits {
	fn default() -> Self {
		Self {
			rows: NonZeroUsize::new(1_000).expect("fixed row limit is positive"),
			bytes: NonZeroUsize::new(1024 * 1024).expect("fixed byte limit is positive"),
			record_bytes: NonZeroUsize::new(16 * 1024)
				.expect("fixed record limit is positive"),
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReconcileToken {
	pub selection: u64,
	pub connection: u64,
	pub attempt: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LogGap {
	UnverifiedContinuity,
	PendingOverflow,
	CursorExpired,
	FetchFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LogSync {
	Syncing,
	Current,
	Degraded(LogGap),
	Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LogError {
	RecordTooLarge,
	SnapshotTooLarge,
}

#[derive(Clone, Debug)]
pub(crate) struct LogState {
	_limits: LogLimits,
	_active: Option<ReconcileToken>,
	_reconciling: bool,
	_pending_overflow: bool,
	_visible: VecDeque<LogRow>,
	_pending: VecDeque<LogRow>,
	_sync: LogSync,
	_retention_removed: usize,
}

impl LogState {
	pub(crate) fn new(limits: LogLimits) -> Self {
		Self {
			_limits: limits,
			_active: None,
			_reconciling: false,
			_pending_overflow: false,
			_visible: VecDeque::new(),
			_pending: VecDeque::new(),
			_sync: LogSync::Stopped,
			_retention_removed: 0,
		}
	}

	pub(crate) fn begin(&mut self, token: ReconcileToken) {
		let selection_changed = self
			._active
			.is_some_and(|active| active.selection != token.selection);
		if selection_changed {
			self._visible.clear();
			self._retention_removed = 0;
		}
		self._active = Some(token);
		self._reconciling = true;
		self._pending_overflow = false;
		self._pending.clear();
		self._sync = LogSync::Syncing;
	}

	pub(crate) fn push(
		&mut self,
		token: ReconcileToken,
		row: LogRow,
	) -> Result<(), LogError> {
		if !self.accepts_token(token) {
			return Ok(());
		}
		self.validate_record(&row)?;
		if self._reconciling {
			self._pending.push_back(row);
			if self.trim_pending() {
				self._pending_overflow = true;
				self._sync = LogSync::Degraded(LogGap::PendingOverflow);
			}
		} else {
			let mut conflict = false;
			if let Some(id) = row.id
				&& let Some(existing) = self._visible.iter().find(|existing| existing.id == Some(id))
			{
				if existing != &row {
					conflict = true;
				}
			} else {
				self._visible.push_back(row);
			}
			if conflict {
				self._sync = LogSync::Degraded(LogGap::UnverifiedContinuity);
			}
			self.trim_visible();
		}
		Ok(())
	}

	pub(crate) fn finish(
		&mut self,
		token: ReconcileToken,
		snapshot: LogSnapshot,
	) -> Result<(), LogError> {
		if !self.accepts_token(token) || !self._reconciling {
			return Ok(());
		}
		self.validate_snapshot(&snapshot)?;

		let pending = std::mem::take(&mut self._pending);
		let pending_overflow = self._pending_overflow;
		let watermark = snapshot.watermark;
		let snapshot_rows = snapshot.rows;
		let all_ids = snapshot_rows.iter().all(|row| row.id.is_some())
			&& pending.iter().all(|row| row.id.is_some());
		let all_cursors = snapshot_rows.iter().all(|row| row.cursor.is_some())
			&& pending.iter().all(|row| row.cursor.is_some());
		let watermark_mode = watermark.is_some() && all_ids && all_cursors;
		let history_exceeds_watermark = watermark_mode
			&& snapshot_rows.iter().any(|row| {
				row.cursor
					.is_some_and(|cursor| cursor > watermark.expect("watermark mode has a watermark"))
			});
		let mut continuity_verified = watermark_mode && !pending_overflow;
		if history_exceeds_watermark {
			continuity_verified = false;
		}
		let mut merged = Vec::new();

		for row in snapshot_rows {
			Self::push_unique(&mut merged, row, &mut continuity_verified);
		}

		if watermark_mode {
			let watermark = watermark.expect("watermark mode has a watermark");
			let mut after_watermark = Vec::new();
			for row in pending {
				if row.cursor.expect("watermark mode has cursors") <= watermark {
					continue;
				}
				after_watermark.push(row);
			}
			after_watermark.sort_by_key(|row| row.cursor.expect("watermark mode has cursors"));
			for row in after_watermark {
				Self::push_unique(&mut merged, row, &mut continuity_verified);
			}
		} else if all_ids {
			continuity_verified = false;
			for row in pending {
				Self::push_unique(&mut merged, row, &mut continuity_verified);
			}
		} else {
			continuity_verified = false;
		}

		self._visible = merged.into_iter().collect();
		self.trim_visible();
		self._reconciling = false;
		self._sync = if pending_overflow {
			LogSync::Degraded(LogGap::PendingOverflow)
		} else if continuity_verified {
			LogSync::Current
		} else {
			LogSync::Degraded(LogGap::UnverifiedContinuity)
		};
		Ok(())
	}

	pub(crate) fn fail(&mut self, token: ReconcileToken, gap: LogGap) {
		if !self.accepts_token(token) {
			return;
		}
		self._pending.clear();
		self._reconciling = false;
		self._sync = LogSync::Degraded(gap);
	}

	pub(crate) fn stop(&mut self) {
		self._active = None;
		self._reconciling = false;
		self._pending.clear();
		self._visible.clear();
		self._retention_removed = 0;
		self._sync = LogSync::Stopped;
	}

	pub(crate) fn rows(&self) -> Vec<LogRow> {
		self._visible.iter().cloned().collect()
	}

	pub(crate) fn sync(&self) -> LogSync {
		self._sync
	}

	pub(crate) fn retention_removed(&self) -> usize {
		self._retention_removed
	}

	fn accepts_token(&self, token: ReconcileToken) -> bool {
		self._active == Some(token) && !matches!(self._sync, LogSync::Stopped)
	}

	fn validate_record(&self, row: &LogRow) -> Result<(), LogError> {
		let Some(bytes) = row_bytes(row) else {
			return Err(LogError::RecordTooLarge);
		};
		if bytes > self._limits.record_bytes.get() {
			Err(LogError::RecordTooLarge)
		} else {
			Ok(())
		}
	}

	fn validate_snapshot(&self, snapshot: &LogSnapshot) -> Result<(), LogError> {
		let mut total = 0usize;
		for row in &snapshot.rows {
			self.validate_record(row)?;
			total = total
				.checked_add(row_bytes(row).expect("validated row size"))
				.ok_or(LogError::SnapshotTooLarge)?;
		}
		let snapshot_limit = self
			._limits
			.bytes
			.get()
			.saturating_mul(2)
			.min(MAX_HTTP_RESPONSE_BYTES);
		if total > snapshot_limit {
			Err(LogError::SnapshotTooLarge)
		} else {
			Ok(())
		}
	}

	fn trim_pending(&mut self) -> bool {
		let mut removed = false;
		while self._pending.len() > self._limits.rows.get()
			|| Self::deque_bytes(&self._pending).is_some_and(|bytes| bytes > self._limits.bytes.get())
		{
			if self._pending.pop_front().is_none() {
				break;
			}
			removed = true;
		}
		removed
	}

	fn trim_visible(&mut self) {
		while self._visible.len() > self._limits.rows.get()
			|| Self::deque_bytes(&self._visible).is_some_and(|bytes| bytes > self._limits.bytes.get())
		{
			if self._visible.pop_front().is_none() {
				break;
			}
			self._retention_removed += 1;
		}
	}

	fn deque_bytes(rows: &VecDeque<LogRow>) -> Option<usize> {
		rows.iter().try_fold(0usize, |total, row| {
			total.checked_add(row_bytes(row)?)
		})
	}

	fn push_unique(rows: &mut Vec<LogRow>, row: LogRow, conflict: &mut bool) {
		if let Some(id) = row.id
			&& let Some(existing) = rows.iter().find(|existing| existing.id == Some(id))
		{
			if existing != &row {
				*conflict = false;
			}
			return;
		}
		rows.push(row);
	}
}

fn row_bytes(row: &LogRow) -> Option<usize> {
	FIXED_ROW_BYTES.checked_add(row.text.len())
}
