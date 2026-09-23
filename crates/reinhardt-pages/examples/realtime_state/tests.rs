use super::logs::{
	LogError, LogGap, LogLimits, LogRow, LogSnapshot, LogState, LogSync, ReconcileToken,
};
use reinhardt_pages::reactive::{ReactiveScope, Signal};
use rstest::rstest;
use std::num::NonZeroUsize;

#[derive(Clone, Copy)]
enum RealtimeLogCase {
	WatermarkMerge,
	EqualTextDistinctIds,
	NonconsecutiveCursors,
	IdsWithoutWatermark,
	NoIdsWithoutWatermark,
	VisibleRetention,
	PendingOverflow,
	OversizedMultibyte,
	OldToken,
	FailureAndStop,
}

fn limits(rows: usize, bytes: usize, record_bytes: usize) -> LogLimits {
	LogLimits {
		rows: NonZeroUsize::new(rows).expect("test row limit must be positive"),
		bytes: NonZeroUsize::new(bytes).expect("test byte limit must be positive"),
		record_bytes: NonZeroUsize::new(record_bytes).expect("test record limit must be positive"),
	}
}

fn row(id: Option<u64>, cursor: Option<u64>, text: &str) -> LogRow {
	LogRow {
		id,
		cursor,
		text: text.to_owned(),
	}
}

fn ids(state: &Signal<LogState>) -> Vec<u64> {
	state
		.get()
		.rows()
		.into_iter()
		.filter_map(|row| row.id)
		.collect()
}

#[rstest]
fn bounded_realtime_log_cases_use_signal_updates() {
	ReactiveScope::run(|| {
		let cases = [
			RealtimeLogCase::WatermarkMerge,
			RealtimeLogCase::EqualTextDistinctIds,
			RealtimeLogCase::NonconsecutiveCursors,
			RealtimeLogCase::IdsWithoutWatermark,
			RealtimeLogCase::NoIdsWithoutWatermark,
			RealtimeLogCase::VisibleRetention,
			RealtimeLogCase::PendingOverflow,
			RealtimeLogCase::OversizedMultibyte,
			RealtimeLogCase::OldToken,
			RealtimeLogCase::FailureAndStop,
		];

		for case in cases {
			match case {
				RealtimeLogCase::WatermarkMerge => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let token = ReconcileToken {
						selection: 1,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						state
							.push(token, row(Some(2), Some(20), "overlap"))
							.unwrap();
						state.push(token, row(Some(3), Some(30), "live")).unwrap();
					});
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: vec![
										row(Some(1), Some(10), "one"),
										row(Some(2), Some(20), "two"),
									],
									watermark: Some(20),
								},
							)
							.unwrap();
					});
					assert_eq!(ids(&state), vec![1, 2, 3]);
					assert_eq!(state.get().sync(), LogSync::Current);
				}
				RealtimeLogCase::EqualTextDistinctIds => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let token = ReconcileToken {
						selection: 2,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: vec![
										row(Some(1), Some(1), "same"),
										row(Some(2), Some(2), "same"),
									],
									watermark: Some(2),
								},
							)
							.unwrap();
					});
					assert_eq!(ids(&state), vec![1, 2]);
					assert_eq!(state.get().sync(), LogSync::Current);
				}
				RealtimeLogCase::NonconsecutiveCursors => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let token = ReconcileToken {
						selection: 3,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						state.push(token, row(Some(2), Some(30), "late")).unwrap();
					});
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: vec![row(Some(1), Some(10), "first")],
									watermark: Some(10),
								},
							)
							.unwrap();
					});
					assert_eq!(ids(&state), vec![1, 2]);
					assert_eq!(state.get().sync(), LogSync::Current);
				}
				RealtimeLogCase::IdsWithoutWatermark => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let token = ReconcileToken {
						selection: 4,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						state
							.push(token, row(Some(2), Some(20), "duplicate"))
							.unwrap();
						state.push(token, row(Some(3), Some(30), "three")).unwrap();
					});
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: vec![
										row(Some(1), Some(10), "one"),
										row(Some(2), Some(20), "two"),
									],
									watermark: None,
								},
							)
							.unwrap();
					});
					assert_eq!(ids(&state), vec![1, 2, 3]);
					assert_eq!(
						state.get().sync(),
						LogSync::Degraded(LogGap::UnverifiedContinuity)
					);
				}
				RealtimeLogCase::NoIdsWithoutWatermark => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let token = ReconcileToken {
						selection: 5,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						state
							.push(token, row(None, None, "ambiguous-live"))
							.unwrap();
					});
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: vec![row(None, None, "history")],
									watermark: None,
								},
							)
							.unwrap();
					});
					assert_eq!(state.get().rows(), vec![row(None, None, "history")]);
					assert_eq!(
						state.get().sync(),
						LogSync::Degraded(LogGap::UnverifiedContinuity)
					);
				}
				RealtimeLogCase::VisibleRetention => {
					let state = Signal::new(LogState::new(limits(2, 512, 128)));
					let token = ReconcileToken {
						selection: 6,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: vec![
										row(Some(1), Some(1), "one"),
										row(Some(2), Some(2), "two"),
										row(Some(3), Some(3), "three"),
									],
									watermark: Some(3),
								},
							)
							.unwrap();
					});
					assert_eq!(ids(&state), vec![2, 3]);
					assert_eq!(state.get().retention_removed(), 1);
				}
				RealtimeLogCase::PendingOverflow => {
					let state = Signal::new(LogState::new(limits(2, 512, 128)));
					let token = ReconcileToken {
						selection: 7,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| {
						for id in 1..=3 {
							state.push(token, row(Some(id), Some(id), "live")).unwrap();
						}
					});
					assert_eq!(
						state.get().sync(),
						LogSync::Degraded(LogGap::PendingOverflow)
					);
					state.update(|state| {
						state
							.finish(
								token,
								LogSnapshot {
									rows: Vec::new(),
									watermark: Some(0),
								},
							)
							.unwrap();
					});
					assert_eq!(ids(&state), vec![2, 3]);
					assert_eq!(
						state.get().sync(),
						LogSync::Degraded(LogGap::PendingOverflow)
					);
				}
				RealtimeLogCase::OversizedMultibyte => {
					let state = Signal::new(LogState::new(limits(2, 512, 19)));
					let token = ReconcileToken {
						selection: 8,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					let result = {
						let mut result = Ok(());
						state.update(|state| {
							result = state.push(token, row(None, None, "éé"));
						});
						result
					};
					assert_eq!(result, Err(LogError::RecordTooLarge));
					assert!(state.get().rows().is_empty());
					assert_eq!(state.get().sync(), LogSync::Syncing);
				}
				RealtimeLogCase::OldToken => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let old = ReconcileToken {
						selection: 9,
						connection: 1,
						attempt: 1,
					};
					let current = ReconcileToken {
						selection: 10,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(old));
					state.update(|state| state.begin(current));
					state.update(|state| {
						state.push(old, row(Some(1), Some(1), "old")).unwrap();
						state
							.finish(
								old,
								LogSnapshot {
									rows: vec![row(Some(1), Some(1), "old")],
									watermark: Some(1),
								},
							)
							.unwrap();
					});
					assert!(state.get().rows().is_empty());
					assert_eq!(state.get().sync(), LogSync::Syncing);
				}
				RealtimeLogCase::FailureAndStop => {
					let state = Signal::new(LogState::new(limits(8, 512, 128)));
					let token = ReconcileToken {
						selection: 11,
						connection: 1,
						attempt: 1,
					};
					state.update(|state| state.begin(token));
					state.update(|state| state.fail(token, LogGap::CursorExpired));
					assert_eq!(state.get().sync(), LogSync::Degraded(LogGap::CursorExpired));

					let next = ReconcileToken {
						selection: 11,
						connection: 2,
						attempt: 1,
					};
					state.update(|state| state.begin(next));
					state.update(|state| state.fail(next, LogGap::FetchFailed));
					assert_eq!(state.get().sync(), LogSync::Degraded(LogGap::FetchFailed));
					state.update(LogState::stop);
					state.update(|state| {
						state.push(next, row(Some(99), Some(99), "late")).unwrap();
						state
							.finish(
								next,
								LogSnapshot {
									rows: vec![row(Some(99), Some(99), "late")],
									watermark: Some(99),
								},
							)
							.unwrap();
					});
					assert!(state.get().rows().is_empty());
					assert_eq!(state.get().sync(), LogSync::Stopped);
				}
			}
		}
	});
}

#[rstest]
fn conflicting_ids_degrade_watermark_reconciliation() {
	ReactiveScope::run(|| {
		let state = Signal::new(LogState::new(limits(8, 512, 128)));
		let token = ReconcileToken {
			selection: 12,
			connection: 1,
			attempt: 1,
		};
		state.update(|state| state.begin(token));
		state.update(|state| {
			state.push(token, row(Some(1), Some(2), "live")).unwrap();
		});
		state.update(|state| {
			state
				.finish(
					token,
					LogSnapshot {
						rows: vec![row(Some(1), Some(1), "history")],
						watermark: Some(1),
					},
				)
				.unwrap();
		});

		assert_eq!(ids(&state), vec![1]);
		assert_eq!(
			state.get().sync(),
			LogSync::Degraded(LogGap::UnverifiedContinuity)
		);
	});
}

#[rstest]
fn oversized_snapshot_is_rejected_before_replacing_state() {
	ReactiveScope::run(|| {
		let state = Signal::new(LogState::new(limits(8, 32, 128)));
		let token = ReconcileToken {
			selection: 13,
			connection: 1,
			attempt: 1,
		};
		state.update(|state| state.begin(token));
		let result = {
			let mut result = Ok(());
			state.update(|state| {
				result = state.finish(
					token,
					LogSnapshot {
						rows: vec![
							row(Some(1), Some(1), "12345678901234567"),
							row(Some(2), Some(2), "12345678901234567"),
						],
						watermark: Some(2),
					},
				);
			});
			result
		};

		assert_eq!(result, Err(LogError::SnapshotTooLarge));
		assert!(state.get().rows().is_empty());
		assert_eq!(state.get().sync(), LogSync::Syncing);
	});
}

#[rstest]
fn pending_byte_limit_trims_oldest_rows_and_records_a_gap() {
	ReactiveScope::run(|| {
		let state = Signal::new(LogState::new(limits(8, 30, 128)));
		let token = ReconcileToken {
			selection: 14,
			connection: 1,
			attempt: 1,
		};
		state.update(|state| state.begin(token));
		state.update(|state| {
			state.push(token, row(Some(1), Some(1), "aa")).unwrap();
			state.push(token, row(Some(2), Some(2), "bb")).unwrap();
		});

		state.update(|state| {
			state
				.finish(
					token,
					LogSnapshot {
						rows: Vec::new(),
						watermark: Some(0),
					},
				)
				.unwrap();
		});

		assert_eq!(ids(&state), vec![2]);
		assert_eq!(
			state.get().sync(),
			LogSync::Degraded(LogGap::PendingOverflow)
		);
	});
}
