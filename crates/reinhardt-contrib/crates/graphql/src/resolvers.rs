//! GraphQL resolvers

use async_graphql::Result as GqlResult;
use async_trait::async_trait;

/// Base resolver trait
#[async_trait]
pub trait Resolver: Send + Sync {
    type Output;

    async fn resolve(&self) -> GqlResult<Self::Output>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestResolver {
        value: i32,
    }

    #[async_trait]
    impl Resolver for TestResolver {
        type Output = i32;

        async fn resolve(&self) -> GqlResult<Self::Output> {
            Ok(self.value * 2)
        }
    }

    struct StringResolver {
        message: String,
    }

    #[async_trait]
    impl Resolver for StringResolver {
        type Output = String;

        async fn resolve(&self) -> GqlResult<Self::Output> {
            Ok(format!("Resolved: {}", self.message))
        }
    }

    #[tokio::test]
    async fn test_resolver_trait_implementation() {
        let resolver = TestResolver { value: 21 };
        let result = resolver.resolve().await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_string_resolver() {
        let resolver = StringResolver {
            message: "Hello GraphQL".to_string(),
        };
        let result = resolver.resolve().await.unwrap();
        assert_eq!(result, "Resolved: Hello GraphQL");
    }

    #[tokio::test]
    async fn test_resolver_multiple_calls() {
        let resolver = TestResolver { value: 10 };

        let result1 = resolver.resolve().await.unwrap();
        let result2 = resolver.resolve().await.unwrap();

        // Should return same result on multiple calls
        assert_eq!(result1, result2);
        assert_eq!(result1, 20);
    }
}
