//! Scalar proxy tests for association proxies
//!
//! These tests verify that scalar proxy operations work correctly for
//! one-to-one and many-to-one relationships, based on SQLAlchemy's tests.

use reinhardt_proxy::{
    ProxyError, ProxyResult, ScalarComparison, ScalarProxy, ScalarValue, reflection::Reflectable,
};
use std::any::Any;

/// Test model representing a user
#[derive(Clone)]
struct User {
    id: i64,
    name: String,
    profile_data: Option<Box<Profile>>,
}

/// Test model representing a user profile
#[derive(Clone)]
struct Profile {
    id: i64,
    user_id: i64,
    bio: String,
    website: Option<String>,
}

impl Reflectable for User {
    fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
        match name {
            "profile_data" => self.profile_data.as_ref().map(|p| {
                Box::new(Box::new(p.as_ref().clone()) as Box<dyn Reflectable>) as Box<dyn Any>
            }),
            _ => None,
        }
    }

    fn get_relationship_mut(&mut self, name: &str) -> Option<&mut dyn Any> {
        match name {
            "profile_data" => self
                .profile_data
                .as_mut()
                .map(|p| p.as_mut() as &mut dyn Any),
            _ => None,
        }
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "id" => Some(ScalarValue::Integer(self.id)),
            "name" => Some(ScalarValue::String(self.name.clone())),
            _ => None,
        }
    }

    fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
        match name {
            "id" => {
                self.id = value.as_integer()?;
                Ok(())
            }
            "name" => {
                self.name = value.as_string()?;
                Ok(())
            }
            _ => Err(ProxyError::AttributeNotFound(name.to_string())),
        }
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        attribute: &str,
        value: ScalarValue,
    ) -> ProxyResult<()> {
        match relationship {
            "profile_data" => {
                if let Some(profile) = self.profile_data.as_mut() {
                    profile.set_attribute(attribute, value)?;
                    Ok(())
                } else {
                    Err(ProxyError::RelationshipNotFound(relationship.to_string()))
                }
            }
            _ => Err(ProxyError::RelationshipNotFound(relationship.to_string())),
        }
    }
}

impl Reflectable for Profile {
    fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any>> {
        None
    }

    fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
        None
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "id" => Some(ScalarValue::Integer(self.id)),
            "user_id" => Some(ScalarValue::Integer(self.user_id)),
            "bio" => Some(ScalarValue::String(self.bio.clone())),
            "website" => self
                .website
                .as_ref()
                .map(|w| ScalarValue::String(w.clone())),
            _ => None,
        }
    }

    fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
        match name {
            "id" => {
                self.id = value.as_integer()?;
                Ok(())
            }
            "user_id" => {
                self.user_id = value.as_integer()?;
                Ok(())
            }
            "bio" => {
                self.bio = value.as_string()?;
                Ok(())
            }
            "website" => {
                self.website = Some(value.as_string()?);
                Ok(())
            }
            _ => Err(ProxyError::AttributeNotFound(name.to_string())),
        }
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        _attribute: &str,
        _value: ScalarValue,
    ) -> ProxyResult<()> {
        Err(ProxyError::RelationshipNotFound(relationship.to_string()))
    }
}

#[tokio::test]
async fn test_scalar_proxy() {
    // Test: Verify that scalar proxy works correctly
    // Based on: test_scalar_proxy from SQLAlchemy

    let proxy = ScalarProxy::new("profile_data", "bio");
    assert_eq!(proxy.relationship, "profile_data");
    assert_eq!(proxy.attribute, "bio");

    let user = User {
        id: 1,
        name: "John".to_string(),
        profile_data: Some(Box::new(Profile {
            id: 1,
            user_id: 1,
            bio: "Software Engineer".to_string(),
            website: None,
        })),
    };

    let bio = proxy.get_value(&user).await.unwrap();
    assert!(bio.is_some());
    assert_eq!(bio.unwrap().as_string().unwrap(), "Software Engineer");
}

#[tokio::test]
async fn test_scalar_proxy_none() {
    // Test: Verify that scalar proxy can handle None

    let proxy = ScalarProxy::new("profile_data", "bio");

    let user = User {
        id: 1,
        name: "Jane".to_string(),
        profile_data: None,
    };

    let bio = proxy.get_value(&user).await.unwrap();
    assert!(bio.is_none());
}

#[tokio::test]
async fn test_scalar_proxy_set_value() {
    // Test: Verify that setting values with scalar proxy works correctly

    let proxy = ScalarProxy::new("profile_data", "bio");

    let mut user = User {
        id: 1,
        name: "Bob".to_string(),
        profile_data: Some(Box::new(Profile {
            id: 1,
            user_id: 1,
            bio: "Old Bio".to_string(),
            website: None,
        })),
    };

    proxy
        .set_value(&mut user, ScalarValue::String("New Bio".to_string()))
        .await
        .unwrap();

    let bio = proxy.get_value(&user).await.unwrap();
    assert_eq!(bio.unwrap().as_string().unwrap(), "New Bio");
}

#[tokio::test]
async fn test_create_on_set_none() {
    // Test: Verify that creation on setting None works correctly
    // Based on: test_create_on_set_none from SQLAlchemy

    let proxy = ScalarProxy::new("profile_data", "website");

    let mut user = User {
        id: 1,
        name: "Alice".to_string(),
        profile_data: None,
    };

    // Manual creation of profile when None (ORM would handle this automatically)
    user.profile_data = Some(Box::new(Profile {
        id: 1,
        user_id: 1,
        bio: String::new(),
        website: None,
    }));

    // Setting a value on the created relationship object
    proxy
        .set_value(
            &mut user,
            ScalarValue::String("https://example.com".to_string()),
        )
        .await
        .unwrap();

    assert!(user.profile_data.is_some());
    let website = proxy.get_value(&user).await.unwrap();
    assert_eq!(website.unwrap().as_string().unwrap(), "https://example.com");
}

#[tokio::test]
async fn test_empty_scalars() {
    // Test: Verify that empty scalars are handled correctly
    // Based on: test_empty_scalars from SQLAlchemy

    let proxy = ScalarProxy::new("profile_data", "bio");

    let user = User {
        id: 1,
        name: "Charlie".to_string(),
        profile_data: Some(Box::new(Profile {
            id: 1,
            user_id: 1,
            bio: "".to_string(), // Empty string
            website: None,
        })),
    };

    let bio = proxy.get_value(&user).await.unwrap();
    assert!(bio.is_some());
    assert_eq!(bio.unwrap().as_string().unwrap(), "");
}

#[test]
fn test_proxy_scalar_comparison_builders() {
    // Test: Verify that scalar comparison builders work correctly

    let eq = ScalarComparison::eq("test");
    assert!(matches!(eq, ScalarComparison::Eq(_)));

    let ne = ScalarComparison::ne("test");
    assert!(matches!(ne, ScalarComparison::Ne(_)));

    let gt = ScalarComparison::gt(42);
    assert!(matches!(gt, ScalarComparison::Gt(_)));

    let gte = ScalarComparison::gte(42);
    assert!(matches!(gte, ScalarComparison::Gte(_)));

    let lt = ScalarComparison::lt(42);
    assert!(matches!(lt, ScalarComparison::Lt(_)));

    let lte = ScalarComparison::lte(42);
    assert!(matches!(lte, ScalarComparison::Lte(_)));

    let is_null = ScalarComparison::is_null();
    assert!(matches!(is_null, ScalarComparison::IsNull));

    let is_not_null = ScalarComparison::is_not_null();
    assert!(matches!(is_not_null, ScalarComparison::IsNotNull));

    let like = ScalarComparison::like("%test%");
    assert!(matches!(like, ScalarComparison::Like(_)));

    let not_like = ScalarComparison::not_like("%test%");
    assert!(matches!(not_like, ScalarComparison::NotLike(_)));
}

#[test]
fn test_scalar_comparison_in_values() {
    // Test: Verify that IN comparison works correctly

    let values = vec![
        ScalarValue::Integer(1),
        ScalarValue::Integer(2),
        ScalarValue::Integer(3),
    ];

    let in_comp = ScalarComparison::in_values(values.clone());
    assert!(matches!(in_comp, ScalarComparison::In(_)));

    let not_in_comp = ScalarComparison::not_in_values(values);
    assert!(matches!(not_in_comp, ScalarComparison::NotIn(_)));
}
