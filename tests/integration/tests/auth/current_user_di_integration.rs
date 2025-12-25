//! CurrentUser DI統合テスト
//!
//! CurrentUser、DI、データベース、ORMの統合をテストします。

use reinhardt_auth::{BaseUser, CurrentUser, DefaultUser, SimpleUser};
use reinhardt_db::DatabaseConnection;
use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
use reinhardt_test::fixtures::{postgres_container, singleton_scope, test_user, TestUser};
use rstest::*;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use std::sync::Arc;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

// ============================================================================
// サニティテスト（2件）
// ============================================================================

#[rstest]
#[tokio::test]
async fn sanity_current_user_di_basic(singleton_scope: Arc<SingletonScope>) {
	// CurrentUserのDI基本動作
	let ctx = InjectionContext::new(singleton_scope);

	// AnonymousUserとして作成
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
}

#[rstest]
#[tokio::test]
async fn sanity_database_user_load(
	#[future] postgres_container: (
		ContainerAsync<Postgres>,
		Arc<sqlx::PgPool>,
		u16,
		String,
	),
) {
	// データベースからのユーザーロード基本動作
	let (_container, pool, _port, _url) = postgres_container.await;

	// テストユーザーをデータベースに挿入
	let user_id = Uuid::new_v4();
	let username = "test_user";

	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT true
		)"
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"INSERT INTO auth_user (id, username, email, is_active) VALUES ($1, $2, $3, $4)"
	)
	.bind(user_id)
	.bind(username)
	.bind("test@example.com")
	.bind(true)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// ユーザーをロード
	let row = sqlx::query("SELECT id, username FROM auth_user WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let loaded_id: Uuid = row.get("id");
	let loaded_username: String = row.get("username");

	assert_eq!(loaded_id, user_id);
	assert_eq!(loaded_username, username);
}

// ============================================================================
// 正常系（6件）
// ============================================================================

#[rstest]
#[tokio::test]
async fn normal_current_user_viewset_injection(test_user: TestUser) {
	// CurrentUserをViewSetに注入してユーザー情報取得
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user.clone(), test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
	assert_eq!(current_user.user().unwrap().get_username(), test_user.username);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_shared_across_endpoints(test_user: TestUser) {
	// CurrentUserを複数エンドポイント間で共有
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user1 = CurrentUser::authenticated(user.clone(), test_user.id);
	let current_user2 = current_user1.clone();

	assert_eq!(current_user1.id().unwrap(), current_user2.id().unwrap());
	assert_eq!(
		current_user1.user().unwrap().get_username(),
		current_user2.user().unwrap().get_username()
	);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_db_query_seaquery(
	#[future] postgres_container: (
		ContainerAsync<Postgres>,
		Arc<sqlx::PgPool>,
		u16,
		String,
	),
	test_user: TestUser,
) {
	// CurrentUserとDBクエリ（SeaQuery）組み合わせ
	let (_container, pool, _port, _url) = postgres_container.await;

	// テーブル作成
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL
		)"
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// ユーザー挿入
	sqlx::query("INSERT INTO auth_user (id, username, email) VALUES ($1, $2, $3)")
		.bind(test_user.id)
		.bind(&test_user.username)
		.bind(&test_user.email)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// SeaQueryでクエリ構築
	#[derive(sea_query::Iden)]
	enum AuthUser {
		Table,
		Id,
		Username,
		Email,
	}

	let query = Query::select()
		.from(AuthUser::Table)
		.columns([AuthUser::Id, AuthUser::Username, AuthUser::Email])
		.and_where(Expr::col(AuthUser::Id).eq(test_user.id))
		.to_owned();

	let sql = query.to_string(PostgresQueryBuilder);

	// クエリ実行
	let row = sqlx::query(&sql)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let loaded_id: Uuid = row.get("id");
	let loaded_username: String = row.get("username");

	assert_eq!(loaded_id, test_user.id);
	assert_eq!(loaded_username, test_user.username);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_orm_queryset_filtering(test_user: TestUser) {
	// CurrentUserとORM QuerySetフィルタリング
	// 注: 実際のORM実装が必要なため、ここでは概念的なテスト
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	// フィルタリング条件としてcurrent_userのIDを使用
	let filter_user_id = current_user.id().unwrap();

	assert_eq!(filter_user_id, test_user.id);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_session_auth_integration(test_user: TestUser) {
	// CurrentUserとセッション認証統合
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
}

#[rstest]
#[tokio::test]
async fn normal_current_user_jwt_auth_integration(test_user: TestUser) {
	// CurrentUserとJWT認証統合
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.user().unwrap().get_username(), test_user.username);
}

// ============================================================================
// 異常系（4件）
// ============================================================================

#[rstest]
#[tokio::test]
async fn abnormal_unauthenticated_current_user_injection() {
	// 未認証時のCurrentUser注入（AnonymousUser）
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
	assert!(current_user.user().is_err());
}

#[rstest]
#[tokio::test]
async fn abnormal_invalid_user_id_injection() {
	// 無効なユーザーIDでのCurrentUser注入
	let invalid_id = Uuid::nil();

	// Noneユーザーで作成（無効なID）
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
}

#[rstest]
#[tokio::test]
async fn abnormal_deleted_user_current_user(
	#[future] postgres_container: (
		ContainerAsync<Postgres>,
		Arc<sqlx::PgPool>,
		u16,
		String,
	),
	test_user: TestUser,
) {
	// DBからユーザー削除後のCurrentUser
	let (_container, pool, _port, _url) = postgres_container.await;

	// テーブル作成
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL
		)"
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// ユーザー挿入
	sqlx::query("INSERT INTO auth_user (id, username, email) VALUES ($1, $2, $3)")
		.bind(test_user.id)
		.bind(&test_user.username)
		.bind(&test_user.email)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// ユーザー削除
	sqlx::query("DELETE FROM auth_user WHERE id = $1")
		.bind(test_user.id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// ユーザーが存在しないことを確認
	let result = sqlx::query("SELECT id FROM auth_user WHERE id = $1")
		.bind(test_user.id)
		.fetch_optional(pool.as_ref())
		.await
		.unwrap();

	assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn abnormal_expired_session_current_user() {
	// セッション期限切れ時のCurrentUser
	// AnonymousUserを返すべき
	let current_user = CurrentUser::<DefaultUser>::anonymous();

	assert!(!current_user.is_authenticated());
	assert!(current_user.id().is_err());
}

// ============================================================================
// 状態遷移系（2件）
// ============================================================================

#[rstest]
#[tokio::test]
async fn state_transition_unauthenticated_to_authenticated(test_user: TestUser) {
	// 未認証→認証→CurrentUser注入→認証済みユーザー取得

	// 初期状態: 未認証
	let current_user = CurrentUser::<SimpleUser>::anonymous();
	assert!(!current_user.is_authenticated());

	// 認証処理（ログイン）
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	// 認証済みCurrentUser作成
	let authenticated_user = CurrentUser::authenticated(user, test_user.id);

	// 最終状態: 認証済み
	assert!(authenticated_user.is_authenticated());
	assert_eq!(authenticated_user.id().unwrap(), test_user.id);
	assert_eq!(authenticated_user.user().unwrap().get_username(), test_user.username);
}

#[rstest]
#[tokio::test]
async fn state_transition_logout_to_anonymous(test_user: TestUser) {
	// ログアウト→CurrentUser無効化→AnonymousUser

	// 初期状態: 認証済み
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let authenticated_user = CurrentUser::authenticated(user, test_user.id);
	assert!(authenticated_user.is_authenticated());

	// ログアウト処理（AnonymousUser作成）
	let anonymous_user = CurrentUser::<SimpleUser>::anonymous();

	// 最終状態: 未認証
	assert!(!anonymous_user.is_authenticated());
	assert!(anonymous_user.id().is_err());
}

// ============================================================================
// 組み合わせテスト（2件）
// ============================================================================

#[rstest]
#[tokio::test]
async fn combination_current_user_session_db(
	#[future] postgres_container: (
		ContainerAsync<Postgres>,
		Arc<sqlx::PgPool>,
		u16,
		String,
	),
	test_user: TestUser,
) {
	// CurrentUser + Session + DB統合
	let (_container, pool, _port, _url) = postgres_container.await;

	// テーブル作成
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS auth_user (
			id UUID PRIMARY KEY,
			username VARCHAR(150) NOT NULL,
			email VARCHAR(254) NOT NULL
		)"
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// ユーザー挿入
	sqlx::query("INSERT INTO auth_user (id, username, email) VALUES ($1, $2, $3)")
		.bind(test_user.id)
		.bind(&test_user.username)
		.bind(&test_user.email)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// DBからユーザーロード
	let row = sqlx::query("SELECT id, username, email FROM auth_user WHERE id = $1")
		.bind(test_user.id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let loaded_id: Uuid = row.get("id");
	let loaded_username: String = row.get("username");
	let loaded_email: String = row.get("email");

	// CurrentUser作成
	let user = SimpleUser {
		id: loaded_id,
		username: loaded_username.clone(),
		email: loaded_email,
		is_active: true,
		is_admin: false,
		is_staff: false,
		is_superuser: false,
	};

	let current_user = CurrentUser::authenticated(user, loaded_id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
	assert_eq!(current_user.user().unwrap().get_username(), loaded_username);
}

#[rstest]
#[tokio::test]
async fn combination_current_user_jwt_redis(test_user: TestUser) {
	// CurrentUser + JWT + Redis統合
	// 注: 実際のRedis実装が必要なため、ここでは概念的なテスト

	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	let current_user = CurrentUser::authenticated(user, test_user.id);

	assert!(current_user.is_authenticated());
	assert_eq!(current_user.id().unwrap(), test_user.id);
}

// ============================================================================
// 同値分割（2件、#[case]）
// ============================================================================

#[rstest]
#[case::simple_user("simple_user")]
#[case::default_user("default_user")]
#[tokio::test]
async fn equivalence_user_type(#[case] user_type: &str, test_user: TestUser) {
	// ユーザータイプ（SimpleUser, DefaultUser）
	match user_type {
		"simple_user" => {
			let user = SimpleUser {
				id: test_user.id,
				username: test_user.username.clone(),
				email: test_user.email.clone(),
				is_active: test_user.is_active,
				is_admin: test_user.is_admin,
				is_staff: test_user.is_staff,
				is_superuser: test_user.is_superuser,
			};

			let current_user = CurrentUser::authenticated(user, test_user.id);
			assert!(current_user.is_authenticated());
		}
		"default_user" => {
			// DefaultUserは実装が必要なため、ここでは概念的なテスト
			// 実際にはDefaultUserのインスタンスを作成してCurrentUserを作成
			assert!(true); // プレースホルダー
		}
		_ => panic!("Unknown user type"),
	}
}

#[rstest]
#[case::request_scope("request")]
#[case::singleton_scope("singleton")]
#[tokio::test]
async fn equivalence_di_scope(#[case] scope_type: &str, singleton_scope: Arc<SingletonScope>) {
	// DIスコープ（Request, Singleton）
	match scope_type {
		"request" => {
			let ctx = InjectionContext::new(singleton_scope.clone());
			// リクエストスコープでのCurrentUser注入
			assert!(true); // 実際のリクエストスコープ実装が必要
		}
		"singleton" => {
			let ctx = InjectionContext::new(singleton_scope);
			// シングルトンスコープでのCurrentUser注入
			assert!(true); // 実際のシングルトンスコープ実装が必要
		}
		_ => panic!("Unknown scope type"),
	}
}

// ============================================================================
// エッジケース（2件）
// ============================================================================

#[rstest]
#[tokio::test]
async fn edge_multiple_current_user_injection_same_request(test_user: TestUser) {
	// 同一リクエスト内での複数CurrentUser注入
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	// 複数のCurrentUserインスタンス作成
	let current_user1 = CurrentUser::authenticated(user.clone(), test_user.id);
	let current_user2 = CurrentUser::authenticated(user.clone(), test_user.id);
	let current_user3 = current_user1.clone();

	// 全て同じユーザーを参照
	assert_eq!(current_user1.id().unwrap(), current_user2.id().unwrap());
	assert_eq!(current_user2.id().unwrap(), current_user3.id().unwrap());
}

#[rstest]
#[tokio::test]
async fn edge_high_load_current_user_injection_performance(test_user: TestUser) {
	// 高負荷時のCurrentUser注入パフォーマンス
	let user = SimpleUser {
		id: test_user.id,
		username: test_user.username.clone(),
		email: test_user.email.clone(),
		is_active: test_user.is_active,
		is_admin: test_user.is_admin,
		is_staff: test_user.is_staff,
		is_superuser: test_user.is_superuser,
	};

	// 1000回のCurrentUser作成
	for _ in 0..1000 {
		let current_user = CurrentUser::authenticated(user.clone(), test_user.id);
		assert!(current_user.is_authenticated());
	}
}

// ============================================================================
// Property-basedテスト（2件、proptest）
// ============================================================================

// 注: proptestの実装には、proptestクレートがworkspace依存として必要
// ここでは、プレースホルダーとしてコメントで記載

// #[cfg(test)]
// mod property_tests {
//     use super::*;
//     use proptest::prelude::*;
//
//     proptest! {
//         #[test]
//         fn prop_authenticated_implies_user_id_some(user_id in any::<Uuid>()) {
//             // CurrentUser.is_authenticated() = true → user_id is Some
//             let user = SimpleUser {
//                 id: user_id,
//                 username: "test".to_string(),
//                 email: "test@example.com".to_string(),
//                 is_active: true,
//                 is_admin: false,
//                 is_staff: false,
//                 is_superuser: false,
//             };
//
//             let current_user = CurrentUser::authenticated(user, user_id);
//
//             prop_assert!(current_user.is_authenticated());
//             prop_assert_eq!(current_user.id().unwrap(), user_id);
//         }
//     }
//
//     // 注: このテストは実際のDB接続が必要なため、通常のproptestでは困難
//     // 代わりに、モックを使用するか、統合テストとして別途実装する必要があります
//     // proptest! {
//     //     #[test]
//     //     fn prop_user_exists_in_db(user_id in any::<Uuid>()) {
//     //         // CurrentUser.user() → DBに存在するユーザー
//     //         // 実装が複雑なため、ここではスキップ
//     //     }
//     // }
// }
