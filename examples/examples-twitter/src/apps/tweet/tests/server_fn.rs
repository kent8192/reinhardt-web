//! Tweet server function tests
//!
//! Tests for create_tweet, list_tweets, and delete_tweet server functions.
use crate::apps::tweet::shared::types::{CreateTweetRequest, TweetInfo};
use crate::test_utils::factories::tweet::TweetFactory;
use crate::test_utils::factories::user::UserFactory;
use crate::test_utils::fixtures::database::twitter_db_pool;
use crate::test_utils::fixtures::users::TestTwitterUser;
use rstest::*;
use sqlx::PgPool;
#[rstest]
#[tokio::test]
async fn test_create_tweet_validation_success(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let tweet_factory = TweetFactory::new();
	let test_user = TestTwitterUser::new("tweetcreator");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let tweet = tweet_factory
		.create(&pool, user.id(), "Hello, Twitter!")
		.await
		.expect("Tweet creation should succeed");
	assert_eq!(tweet.content(), "Hello, Twitter!");
	assert_eq!(tweet.like_count(), 0);
	assert_eq!(tweet.retweet_count(), 0);
}
#[rstest]
#[tokio::test]
async fn test_create_tweet_validation_empty_content() {
	use reinhardt::Validate;
	let request = CreateTweetRequest {
		content: "".to_string(),
	};
	let result = request.validate();
	assert!(result.is_err(), "Empty content should fail validation");
}
#[rstest]
#[tokio::test]
async fn test_create_tweet_validation_too_long() {
	use reinhardt::Validate;
	let long_content = "a".repeat(281);
	let request = CreateTweetRequest {
		content: long_content,
	};
	let result = request.validate();
	assert!(
		result.is_err(),
		"Content over 280 chars should fail validation"
	);
}
#[rstest]
#[tokio::test]
async fn test_create_tweet_validation_max_length() {
	use reinhardt::Validate;
	let max_content = "a".repeat(280);
	let request = CreateTweetRequest {
		content: max_content,
	};
	let result = request.validate();
	assert!(
		result.is_ok(),
		"Content at 280 chars should pass validation"
	);
}
#[rstest]
#[tokio::test]
async fn test_list_tweets_empty(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let tweet_factory = TweetFactory::new();
	let user_factory = UserFactory::new();
	let test_user = TestTwitterUser::new("notweets");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let tweets = tweet_factory
		.find_by_user_id(&pool, user.id())
		.await
		.expect("Query should succeed");
	assert!(tweets.is_empty(), "User should have no tweets");
}
#[rstest]
#[tokio::test]
async fn test_list_tweets_multiple(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let tweet_factory = TweetFactory::new();
	let test_user = TestTwitterUser::new("multitweets");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let contents = ["Tweet 1", "Tweet 2", "Tweet 3"];
	let created = tweet_factory
		.create_many(&pool, user.id(), &contents)
		.await
		.expect("Tweet creation should succeed");
	assert_eq!(created.len(), 3);
	let tweets = tweet_factory
		.find_by_user_id(&pool, user.id())
		.await
		.expect("Query should succeed");
	assert_eq!(tweets.len(), 3);
}
#[rstest]
#[tokio::test]
async fn test_list_tweets_ordering(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let tweet_factory = TweetFactory::new();
	let test_user = TestTwitterUser::new("orderedtweets");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	tweet_factory
		.create(&pool, user.id(), "First tweet")
		.await
		.expect("Tweet creation should succeed");
	tweet_factory
		.create(&pool, user.id(), "Second tweet")
		.await
		.expect("Tweet creation should succeed");
	tweet_factory
		.create(&pool, user.id(), "Third tweet")
		.await
		.expect("Tweet creation should succeed");
	let tweets = tweet_factory
		.find_by_user_id(&pool, user.id())
		.await
		.expect("Query should succeed");
	assert_eq!(tweets.len(), 3);
	assert_eq!(tweets[0].content(), "Third tweet");
	assert_eq!(tweets[1].content(), "Second tweet");
	assert_eq!(tweets[2].content(), "First tweet");
}
#[rstest]
#[tokio::test]
async fn test_delete_tweet_success(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let tweet_factory = TweetFactory::new();
	let test_user = TestTwitterUser::new("deletetweet");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let tweet = tweet_factory
		.create(&pool, user.id(), "To be deleted")
		.await
		.expect("Tweet creation should succeed");
	tweet_factory
		.delete(&pool, tweet.id())
		.await
		.expect("Delete should succeed");
	let count = tweet_factory
		.count_by_user_id(&pool, user.id())
		.await
		.expect("Count should succeed");
	assert_eq!(count, 0, "Tweet should be deleted");
}
#[rstest]
#[tokio::test]
async fn test_delete_tweet_not_found(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let tweet_factory = TweetFactory::new();
	let fake_id = uuid::Uuid::now_v7();
	let result = tweet_factory.delete(&pool, fake_id).await;
	assert!(
		result.is_ok(),
		"Delete should not error for non-existent tweet"
	);
}
#[rstest]
#[tokio::test]
async fn test_tweet_info_from_tweet(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let tweet_factory = TweetFactory::new();
	let test_user = TestTwitterUser::new("infouser");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let tweet = tweet_factory
		.create(&pool, user.id(), "Test content")
		.await
		.expect("Tweet creation should succeed");
	let tweet_info = TweetInfo::from(tweet.clone());
	assert_eq!(tweet_info.id, tweet.id());
	assert_eq!(tweet_info.user_id, *tweet.user_id());
	assert_eq!(tweet_info.content, "Test content");
	assert_eq!(tweet_info.like_count, 0);
	assert_eq!(tweet_info.retweet_count, 0);
}
#[rstest]
#[tokio::test]
async fn test_create_tweet_with_counts(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let user_factory = UserFactory::new();
	let tweet_factory = TweetFactory::new();
	let test_user = TestTwitterUser::new("countuser");
	let user = user_factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let tweet = tweet_factory
		.create_with_counts(&pool, user.id(), "Popular tweet", 100, 50)
		.await
		.expect("Tweet creation should succeed");
	assert_eq!(tweet.content(), "Popular tweet");
	assert_eq!(tweet.like_count(), 100);
	assert_eq!(tweet.retweet_count(), 50);
}
