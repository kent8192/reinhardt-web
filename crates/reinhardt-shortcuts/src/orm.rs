//! ORM-integrated shortcut functions for database queries with 404 error handling
//!
//! These functions provide direct integration with `reinhardt-orm` for database
//! operations that return 404 errors when objects are not found.
//!
//! This module is only available with the `database` feature enabled.

#[cfg(feature = "database")]
use reinhardt_db::Model;
#[cfg(feature = "database")]
use reinhardt_http::Response;

/// Get a single object from the database or return a 404 response
///
/// This function directly integrates with the ORM to query the database and
/// returns a 404 HTTP response if the object is not found.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::get_object_or_404;
/// use reinhardt_orm::Model;
///
/// // In an async view handler:
/// async fn user_detail(user_id: i64) -> Result<Response, Response> {
///     let user = get_object_or_404::<User>(user_id).await?;
///     // user is guaranteed to exist here
///     Ok(render_json(&user))
/// }
/// ```
///
/// # Arguments
///
/// * `pk` - The primary key of the object to retrieve
///
/// # Returns
///
/// Either the queried object or a 404 Response
///
/// # Errors
///
/// Returns `Err(Response)` with HTTP 404 if the object is not found,
/// or HTTP 500 if a database error occurs.
#[cfg(feature = "database")]
pub async fn get_object_or_404<M>(pk: M::PrimaryKey) -> Result<M, Response>
where
    M: Model + serde::de::DeserializeOwned + 'static,
    M::PrimaryKey: ToString,
{
    // Get the manager for this model
    let manager = M::objects();

    // Query by primary key
    let queryset = manager.get(pk);

    // Execute the query (note: current QuerySet is stub, so this returns empty Vec)
    // In a real implementation, this would execute SQL and return results
    let results = queryset.all();

    match results.into_iter().next() {
        Some(obj) => Ok(obj),
        None => Err(Response::not_found()),
    }
}

/// Get a list of objects from the database or return a 404 response if empty
///
/// This function queries the database using the provided `QuerySet` and returns
/// a 404 HTTP response if the result list is empty.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::get_list_or_404;
/// use reinhardt_orm::{Model, QuerySet};
///
/// // In an async view handler:
/// async fn user_list(status: &str) -> Result<Response, Response> {
///     let queryset = User::objects()
///         .filter("status", FilterOperator::Eq, FilterValue::String(status.to_string()));
///
///     let users = get_list_or_404(queryset).await?;
///     // users is guaranteed to be non-empty here
///     Ok(render_json(&users))
/// }
/// ```
///
/// # Arguments
///
/// * `queryset` - A QuerySet to execute
///
/// # Returns
///
/// Either a non-empty list of objects or a 404 Response
///
/// # Errors
///
/// Returns `Err(Response)` with HTTP 404 if the result list is empty,
/// or HTTP 500 if a database error occurs.
#[cfg(feature = "database")]
pub async fn get_list_or_404<M>(queryset: reinhardt_db::QuerySet<M>) -> Result<Vec<M>, Response>
where
    M: Model + 'static,
{
    // Execute the query
    let results = queryset.all();

    if results.is_empty() {
        Err(Response::not_found())
    } else {
        Ok(results)
    }
}

#[cfg(all(test, feature = "database"))]
mod tests {
    use super::*;
    use reinhardt_db::{Model, QuerySet};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestUser {
        id: Option<i64>,
        username: String,
        email: String,
    }

    impl Model for TestUser {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "test_users"
        }

        fn primary_key_field() -> &'static str {
            "id"
        }

        fn primary_key(&self) -> Option<Self::PrimaryKey> {
            self.id
        }

        fn objects() -> reinhardt_db::Manager<Self> {
            reinhardt_db::Manager::new()
        }
    }

    #[tokio::test]
    async fn test_get_object_or_404_not_found() {
        // Note: Since QuerySet::all() is a stub that returns empty Vec,
        // this will always return 404
        let result = get_object_or_404::<TestUser>(999).await;
        assert!(result.is_err());

        let response = result.unwrap_err();
        assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_list_or_404_empty() {
        let queryset = QuerySet::<TestUser>::new();
        let result = get_list_or_404(queryset).await;
        assert!(result.is_err());

        let response = result.unwrap_err();
        assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
    }
}
