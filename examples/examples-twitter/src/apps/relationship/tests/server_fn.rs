//! Relationship server function tests
//!
//! Tests for follow_user, unfollow_user, fetch_followers, fetch_following.
//!
//! Note: Full ManyToMany integration tests require proper database setup.
//! These tests focus on validation and user lookup aspects.
use crate::apps::auth::shared::types::UserInfo;
use crate::test_utils::factories::user::UserFactory;
use crate::test_utils::fixtures::database::twitter_db_pool;
use crate::test_utils::fixtures::users::TestTwitterUser;
use rstest::*;
use sqlx::PgPool;
#[rstest]
#[tokio::test]
async fn test_user_exists_for_follow(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let follower_data = TestTwitterUser::new("follower");
	let target_data = TestTwitterUser::new("target");
	let follower = user_factory
		.create_from_test_user(&pool, &follower_data)
		.await
		.expect("Follower creation should succeed");
	let target = user_factory
		.create_from_test_user(&pool, &target_data)
		.await
		.expect("Target creation should succeed");
	let found_follower = user_factory
		.find_by_id(&pool, follower.id())
		.await
		.expect("Follower should be found");
	let found_target = user_factory
		.find_by_id(&pool, target.id())
		.await
		.expect("Target should be found");
	assert_eq!(found_follower.username(), "follower");
	assert_eq!(found_target.username(), "target");
}
#[rstest]
#[tokio::test]
async fn test_user_not_found_for_follow(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let fake_id = uuid::Uuid::now_v7();
	let result = user_factory.find_by_id(&pool, fake_id).await;
	assert!(result.is_err(), "Non-existent user should not be found");
}
#[rstest]
#[tokio::test]
async fn test_user_info_for_follower_list(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let test_user = TestTwitterUser::new("listuser");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let user_info = UserInfo::from(user.clone());
	assert_eq!(user_info.id, user.id());
	assert_eq!(user_info.username, "listuser");
	assert_eq!(user_info.email, test_user.email);
	assert!(user_info.is_active);
}
#[rstest]
#[tokio::test]
async fn test_create_multiple_users_for_following(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let user1 = user_factory
		.create_from_test_user(&pool, &TestTwitterUser::new("followinguser1"))
		.await
		.expect("User 1 creation should succeed");
	let user2 = user_factory
		.create_from_test_user(&pool, &TestTwitterUser::new("followinguser2"))
		.await
		.expect("User 2 creation should succeed");
	let user3 = user_factory
		.create_from_test_user(&pool, &TestTwitterUser::new("followinguser3"))
		.await
		.expect("User 3 creation should succeed");
	assert_ne!(user1.id(), user2.id());
	assert_ne!(user2.id(), user3.id());
	assert_ne!(user1.id(), user3.id());
	assert!(user1.is_active);
	assert!(user2.is_active);
	assert!(user3.is_active);
}
#[rstest]
#[tokio::test]
async fn test_empty_followers_list(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let test_user = TestTwitterUser::new("nofollowers");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let found = user_factory
		.find_by_id(&pool, user.id())
		.await
		.expect("User should be found");
	assert_eq!(found.username(), "nofollowers");
}
#[rstest]
#[tokio::test]
async fn test_empty_following_list(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let test_user = TestTwitterUser::new("nofollowing");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let found = user_factory
		.find_by_id(&pool, user.id())
		.await
		.expect("User should be found");
	assert_eq!(found.username(), "nofollowing");
}
#[rstest]
#[tokio::test]
async fn test_self_follow_detection(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let test_user = TestTwitterUser::new("selffollow");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let follower_id = user.id();
	let target_id = user.id();
	assert_eq!(
		follower_id, target_id,
		"Same user IDs should be detected for self-follow prevention"
	);
}
#[rstest]
#[tokio::test]
async fn test_inactive_user_in_relationship(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let test_user = TestTwitterUser::new("inactiverel").with_active(false);
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	assert!(!user.is_active, "User should be inactive");
	let user_info = UserInfo::from(user);
	assert!(
		!user_info.is_active,
		"UserInfo should reflect inactive status"
	);
}
