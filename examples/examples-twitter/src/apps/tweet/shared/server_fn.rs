//! Tweet server functions
//!
//! Server functions for tweet management.
use crate::apps::tweet::shared::types::TweetInfo;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};
use uuid::Uuid;
#[cfg(native)]
use {
	crate::apps::auth::models::User,
	crate::apps::tweet::models::Tweet,
	crate::apps::tweet::shared::types::CreateTweetRequest,
	reinhardt::AuthUser,
	reinhardt::DatabaseConnection,
	reinhardt::Validate,
	reinhardt::db::orm::{Filter, FilterOperator, FilterValue, Model},
};
/// Create a new tweet
///
/// Accepts `content` as a String parameter (form! macro passes individual field values).
/// Internally constructs CreateTweetRequest for validation.
/// `_csrf_token` is auto-appended by the `form!` macro for non-GET forms;
/// CSRF is enforced by middleware. See #3825.
#[server_fn]
pub async fn create_tweet(
	content: String,
	_csrf_token: String,
	#[inject] db: DatabaseConnection,
	#[inject] AuthUser(user): AuthUser<User>,
) -> std::result::Result<TweetInfo, ServerFnError> {
	let request = CreateTweetRequest {
		content: content.clone(),
	};
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;
	let user_id = user.id();
	let tweet = Tweet::new(request.content.clone(), 0, 0, user_id);
	Tweet::objects()
		.create_with_conn(&db, &tweet)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;
	Ok(TweetInfo::new(
		tweet.id(),
		user_id,
		user.username().to_string(),
		tweet.content().to_string(),
		tweet.like_count(),
		tweet.retweet_count(),
		tweet.created_at().to_rfc3339(),
	))
}
/// List tweets
#[server_fn]
pub async fn list_tweets(
	user_id: Option<Uuid>,
	page: u32,
	#[inject] db: DatabaseConnection,
) -> std::result::Result<Vec<TweetInfo>, ServerFnError> {
	const PAGE_SIZE: u32 = 20;
	let mut query = Tweet::objects().all();
	if let Some(uid) = user_id {
		query = query.filter(Filter::new(
			"user_id",
			FilterOperator::Eq,
			FilterValue::string(uid),
		));
	}
	let tweets = query
		.order_by(&["-created_at"])
		.limit(PAGE_SIZE as usize)
		.offset((page * PAGE_SIZE) as usize)
		.all_with_db(&db)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;
	let user_ids: Vec<String> = tweets
		.iter()
		.map(|t| t.user_id().to_string())
		.collect::<std::collections::HashSet<_>>()
		.into_iter()
		.collect();
	let users = if user_ids.is_empty() {
		Vec::new()
	} else {
		User::objects()
			.filter_by(Filter::new(
				"id",
				FilterOperator::In,
				FilterValue::Array(user_ids),
			))
			.all_with_db(&db)
			.await
			.map_err(|e| ServerFnError::application(format!("Failed to fetch users: {}", e)))?
	};
	let user_map: std::collections::HashMap<uuid::Uuid, &User> =
		users.iter().map(|u| (u.id(), u)).collect();
	let mut tweet_infos = Vec::with_capacity(tweets.len());
	for tweet in tweets {
		let user = user_map.get(tweet.user_id()).ok_or_else(|| {
			ServerFnError::application(format!("User not found for tweet: {}", tweet.id()))
		})?;
		tweet_infos.push(TweetInfo::new(
			tweet.id(),
			*tweet.user_id(),
			user.username().to_string(),
			tweet.content().to_string(),
			tweet.like_count(),
			tweet.retweet_count(),
			tweet.created_at().to_rfc3339(),
		));
	}
	Ok(tweet_infos)
}
/// Delete a tweet
#[server_fn]
pub async fn delete_tweet(
	tweet_id: Uuid,
	#[inject] db: DatabaseConnection,
	#[inject] AuthUser(user): AuthUser<User>,
) -> std::result::Result<(), ServerFnError> {
	let user_id = user.id();
	let tweet = Tweet::objects()
		.filter_by(Filter::new(
			"id",
			FilterOperator::Eq,
			FilterValue::string(tweet_id),
		))
		.first_with_db(&db)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::application("Tweet not found".to_string()))?;
	if *tweet.user_id() != user_id {
		return Err(ServerFnError::server(
			403,
			"Permission denied: You can only delete your own tweets",
		));
	}
	Tweet::objects()
		.delete_with_conn(&db, tweet.id())
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;
	Ok(())
}
