//! Thread-local循環依存検出機構
//!
//! このモジュールは、DI依存性解決時の循環参照を検出するための最適化された機構を提供します。
//!
//! ## 特徴
//!
//! - **O(1)循環検出**: `HashSet<TypeId>` による高速なルックアップ
//! - **Thread-local**: `RefCell` による低コストの借用チェック（Mutexロック不要）
//! - **深度制限**: `MAX_RESOLUTION_DEPTH` で病的なケースを防止
//! - **サンプリング**: 深い依存チェーンでは10回に1回の頻度でチェック
//! - **RAII**: `ResolutionGuard` により自動的にクリーンアップ
//!
//! ## パフォーマンス目標
//!
//! - キャッシュヒット: < 5% オーバーヘッド（循環検出を完全スキップ）
//! - キャッシュミス: 10-20% オーバーヘッド（最適化された検出）
//! - 深い依存チェーン: サンプリングにより線形コストを削減

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// 最大解決深度（病的なケースを防止）
const MAX_RESOLUTION_DEPTH: usize = 100;

/// サンプリングレート（深い依存チェーンでN回に1回チェック）
const CYCLE_DETECTION_SAMPLING_RATE: usize = 10;

thread_local! {
	/// 現在解決中の型のセット（O(1) 循環検出）
	static RESOLUTION_SET: RefCell<HashSet<TypeId>> = RefCell::new(HashSet::new());

	/// 解決深度カウンター
	static RESOLUTION_DEPTH: RefCell<usize> = const { RefCell::new(0) };

	/// 型名マッピング（エラーメッセージ用）
	static TYPE_NAMES: RefCell<HashMap<TypeId, &'static str>> =
		RefCell::new(HashMap::new());

	/// 解決パス（循環パス表示用）
	static RESOLUTION_PATH: RefCell<Vec<(TypeId, &'static str)>> =
		const { RefCell::new(Vec::new()) };
}

/// 循環参照チェック（O(1)）
///
/// 指定された型が現在の解決スタックに含まれているかチェックします。
fn check_circular_dependency(type_id: TypeId) -> Result<(), CycleError> {
	RESOLUTION_SET.with(|set| {
		let set_ref = set.borrow();
		if set_ref.contains(&type_id) {
			// 循環検出: 型名を取得してエラー構築
			let type_name = get_type_name(type_id);
			let cycle_path = build_cycle_path(type_id);
			return Err(CycleError::CircularDependency {
				type_name: type_name.to_string(),
				path: cycle_path,
			});
		}
		Ok(())
	})
}

/// 解決開始を記録
///
/// 型の解決開始時に呼び出され、循環検出の準備を行います。
/// 戻り値の `ResolutionGuard` はRAIIパターンでクリーンアップを自動化します。
pub fn begin_resolution(
	type_id: TypeId,
	type_name: &'static str,
) -> Result<ResolutionGuard, CycleError> {
	// 深度チェック
	let depth = RESOLUTION_DEPTH.with(|d| {
		let mut depth = d.borrow_mut();
		*depth += 1;
		*depth
	});

	if depth > MAX_RESOLUTION_DEPTH {
		// 深度超過: クリーンアップしてエラー
		RESOLUTION_DEPTH.with(|d| {
			let mut depth = d.borrow_mut();
			*depth -= 1;
		});
		return Err(CycleError::MaxDepthExceeded(depth));
	}

	// サンプリング: 深い依存チェーンでは10回に1回だけチェック
	if depth > 50 && !depth.is_multiple_of(CYCLE_DETECTION_SAMPLING_RATE) {
		return Ok(ResolutionGuard::Sampled);
	}

	// 循環チェック
	check_circular_dependency(type_id)?;

	// セットに追加
	RESOLUTION_SET.with(|set| {
		set.borrow_mut().insert(type_id);
	});

	// パスに追加（エラーメッセージ用）
	RESOLUTION_PATH.with(|path| {
		path.borrow_mut().push((type_id, type_name));
	});

	Ok(ResolutionGuard::Tracked(type_id))
}

/// RAIIガード: Drop時に自動的にクリーンアップ
///
/// 解決が完了したら、スタックから型を削除します。
pub enum ResolutionGuard {
	/// 循環検出を追跡中
	Tracked(TypeId),
	/// サンプリングによりスキップ
	Sampled,
}

impl Drop for ResolutionGuard {
	fn drop(&mut self) {
		if let ResolutionGuard::Tracked(type_id) = self {
			RESOLUTION_SET.with(|set| {
				set.borrow_mut().remove(type_id);
			});

			RESOLUTION_PATH.with(|path| {
				let mut path = path.borrow_mut();
				if let Some(pos) = path.iter().rposition(|(id, _)| id == type_id) {
					path.remove(pos);
				}
			});
		}

		RESOLUTION_DEPTH.with(|d| {
			let mut depth = d.borrow_mut();
			*depth = depth.saturating_sub(1);
		});
	}
}

/// 型名を登録
///
/// エラーメッセージで型名を表示するため、型名をマッピングに登録します。
pub fn register_type_name<T: 'static>(name: &'static str) {
	TYPE_NAMES.with(|names| {
		names.borrow_mut().insert(TypeId::of::<T>(), name);
	});
}

/// 型名を取得
fn get_type_name(type_id: TypeId) -> &'static str {
	TYPE_NAMES.with(|names| names.borrow().get(&type_id).copied().unwrap_or("<unknown>"))
}

/// 循環パスを構築
///
/// 現在の解決パスから循環部分を抽出し、わかりやすい文字列として返します。
fn build_cycle_path(current_type_id: TypeId) -> String {
	RESOLUTION_PATH.with(|path| {
		let path = path.borrow();

		// 循環の開始位置を見つける
		if let Some(cycle_start) = path.iter().position(|(id, _)| *id == current_type_id) {
			// 循環部分を抽出
			let cycle: Vec<&str> = path[cycle_start..].iter().map(|(_, name)| *name).collect();

			// 最後に現在の型を追加して循環を完成
			let cycle_with_end = format!(
				"{} -> {}",
				cycle.join(" -> "),
				get_type_name(current_type_id)
			);

			cycle_with_end
		} else {
			// 循環が見つからない場合（通常は発生しない）
			format!("Unknown cycle involving {}", get_type_name(current_type_id))
		}
	})
}

/// 循環依存エラー
#[derive(Debug, thiserror::Error)]
pub enum CycleError {
	/// 循環依存が検出された
	#[error(
		"Circular dependency detected: {type_name}\n  Path: {path}\nThis forms a cycle that cannot be resolved."
	)]
	CircularDependency {
		/// 循環に含まれる型の名前
		type_name: String,
		/// 循環パス（A -> B -> C -> A の形式）
		path: String,
	},

	/// 最大解決深度を超過した
	#[error(
		"Maximum resolution depth exceeded: {0}\nThis likely indicates an extremely deep or circular dependency chain."
	)]
	MaxDepthExceeded(usize),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_simple_cycle_detection() {
		// Start resolving TypeA
		let type_a = TypeId::of::<TypeA>();
		register_type_name::<TypeA>("TypeA");

		let guard_a = begin_resolution(type_a, "TypeA").unwrap();

		// Attempting to resolve TypeA again should cause circular error
		let result = begin_resolution(type_a, "TypeA");
		assert!(matches!(result, Err(CycleError::CircularDependency { .. })));

		// Drop guard to cleanup
		drop(guard_a);

		// After cleanup, resolution should succeed again
		let result = begin_resolution(type_a, "TypeA");
		assert!(result.is_ok());
	}

	#[test]
	fn test_depth_limit() {
		// Test depth tracking and cleanup
		// Since we can't easily create 100 unique TypeIds, we test that:
		// 1. Depth is tracked correctly
		// 2. Depth is reset after guards are dropped

		use std::marker::PhantomData;

		// Test depth tracking with a few different types
		let type1 = std::any::TypeId::of::<PhantomData<[u8; 0]>>();
		let type2 = std::any::TypeId::of::<PhantomData<[u8; 1]>>();
		let type3 = std::any::TypeId::of::<PhantomData<[u8; 2]>>();

		// Initial depth should be 0
		let initial_depth = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(initial_depth, 0, "Initial depth should be 0");

		// Start first resolution
		let guard1 = begin_resolution(type1, "Type1").unwrap();
		let depth1 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(depth1, 1, "Depth should be 1 after first resolution");

		// Start second resolution (different type)
		let guard2 = begin_resolution(type2, "Type2").unwrap();
		let depth2 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(depth2, 2, "Depth should be 2 after second resolution");

		// Start third resolution (different type)
		let guard3 = begin_resolution(type3, "Type3").unwrap();
		let depth3 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(depth3, 3, "Depth should be 3 after third resolution");

		// Drop guards in reverse order
		drop(guard3);
		let depth_after_drop3 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(
			depth_after_drop3, 2,
			"Depth should be 2 after dropping guard3"
		);

		drop(guard2);
		let depth_after_drop2 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(
			depth_after_drop2, 1,
			"Depth should be 1 after dropping guard2"
		);

		drop(guard1);
		let depth_after_drop1 = RESOLUTION_DEPTH.with(|d| *d.borrow());
		assert_eq!(
			depth_after_drop1, 0,
			"Depth should be 0 after dropping all guards"
		);
	}

	// Dummy type for testing
	struct TypeA;
}
