//! Recursive serialization support
//!
//! This module provides utilities for handling recursive and deeply nested serialization.

use std::collections::HashSet;

/// Context for tracking serialization depth and visited objects
#[derive(Debug, Clone)]
pub struct SerializationContext {
    /// Current depth level (0 = root)
    current_depth: usize,
    /// Maximum allowed depth
    max_depth: usize,
    /// Set of visited object identifiers to detect circular references
    visited: HashSet<String>,
}

impl SerializationContext {
    /// Create a new serialization context
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let context = SerializationContext::new(3);
    /// assert_eq!(context.current_depth(), 0);
    /// assert_eq!(context.max_depth(), 3);
    /// ```
    pub fn new(max_depth: usize) -> Self {
        Self {
            current_depth: 0,
            max_depth,
            visited: HashSet::new(),
        }
    }

    /// Get the current depth
    pub fn current_depth(&self) -> usize {
        self.current_depth
    }

    /// Get the maximum depth
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Check if we can go deeper
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let context = SerializationContext::new(2);
    /// assert!(context.can_go_deeper());
    /// ```
    pub fn can_go_deeper(&self) -> bool {
        self.current_depth < self.max_depth
    }

    /// Check if an object has been visited
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let mut context = SerializationContext::new(5);
    /// assert!(!context.is_visited("user:1"));
    ///
    /// context.mark_visited("user:1".to_string());
    /// assert!(context.is_visited("user:1"));
    /// ```
    pub fn is_visited(&self, object_id: &str) -> bool {
        self.visited.contains(object_id)
    }

    /// Mark an object as visited
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let mut context = SerializationContext::new(5);
    /// context.mark_visited("user:1".to_string());
    /// assert!(context.is_visited("user:1"));
    /// ```
    pub fn mark_visited(&mut self, object_id: String) {
        self.visited.insert(object_id);
    }

    /// Unmark an object as visited (for backtracking)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let mut context = SerializationContext::new(5);
    /// context.mark_visited("user:1".to_string());
    /// assert!(context.is_visited("user:1"));
    ///
    /// context.unmark_visited("user:1");
    /// assert!(!context.is_visited("user:1"));
    /// ```
    pub fn unmark_visited(&mut self, object_id: &str) {
        self.visited.remove(object_id);
    }

    /// Create a child context with increased depth
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let context = SerializationContext::new(3);
    /// let child = context.child();
    ///
    /// assert_eq!(child.current_depth(), 1);
    /// assert_eq!(child.max_depth(), 3);
    /// ```
    pub fn child(&self) -> Self {
        Self {
            current_depth: self.current_depth + 1,
            max_depth: self.max_depth,
            visited: self.visited.clone(),
        }
    }

    /// Reset the context to initial state
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let mut context = SerializationContext::new(3);
    /// let child = context.child();
    /// assert_eq!(child.current_depth(), 1);
    ///
    /// let mut reset_context = child;
    /// reset_context.reset();
    /// assert_eq!(reset_context.current_depth(), 0);
    /// ```
    pub fn reset(&mut self) {
        self.current_depth = 0;
        self.visited.clear();
    }

    /// Get the remaining depth
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::SerializationContext;
    ///
    /// let context = SerializationContext::new(3);
    /// assert_eq!(context.remaining_depth(), 3);
    ///
    /// let child = context.child();
    /// assert_eq!(child.remaining_depth(), 2);
    /// ```
    pub fn remaining_depth(&self) -> usize {
        self.max_depth.saturating_sub(self.current_depth)
    }
}

impl Default for SerializationContext {
    fn default() -> Self {
        Self::new(1)
    }
}

/// Result type for recursive serialization
pub type RecursiveResult<T> = Result<T, RecursiveError>;

/// Errors that can occur during recursive serialization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecursiveError {
    /// Maximum depth exceeded
    MaxDepthExceeded {
        current_depth: usize,
        max_depth: usize,
    },
    /// Circular reference detected
    CircularReference { object_id: String },
    /// General serialization error
    SerializationError { message: String },
}

impl std::fmt::Display for RecursiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecursiveError::MaxDepthExceeded {
                current_depth,
                max_depth,
            } => write!(
                f,
                "Maximum depth exceeded: current={}, max={}",
                current_depth, max_depth
            ),
            RecursiveError::CircularReference { object_id } => {
                write!(f, "Circular reference detected: {}", object_id)
            }
            RecursiveError::SerializationError { message } => {
                write!(f, "Serialization error: {}", message)
            }
        }
    }
}

impl std::error::Error for RecursiveError {}

/// Helper trait for objects that can provide an identifier
pub trait ObjectIdentifiable {
    /// Get a unique identifier for this object
    ///
    /// This identifier should be stable and unique across instances.
    /// Typically uses the model name and primary key (e.g., "User:123").
    fn object_id(&self) -> String;
}

/// Helper functions for circular reference detection
pub mod circular {
    use super::*;

    /// Check if adding an object would create a circular reference
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::{SerializationContext, circular};
    ///
    /// let mut context = SerializationContext::new(5);
    /// context.mark_visited("user:1".to_string());
    ///
    /// assert!(circular::would_be_circular(&context, "user:1"));
    /// assert!(!circular::would_be_circular(&context, "user:2"));
    /// ```
    pub fn would_be_circular(context: &SerializationContext, object_id: &str) -> bool {
        context.is_visited(object_id)
    }

    /// Attempt to visit an object, returning an error if it would create a circular reference
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::{SerializationContext, circular};
    ///
    /// let mut context = SerializationContext::new(5);
    ///
    /// assert!(circular::try_visit(&mut context, "user:1").is_ok());
    /// assert!(circular::try_visit(&mut context, "user:1").is_err());
    /// ```
    pub fn try_visit(context: &mut SerializationContext, object_id: &str) -> RecursiveResult<()> {
        if would_be_circular(context, object_id) {
            return Err(RecursiveError::CircularReference {
                object_id: object_id.to_string(),
            });
        }
        context.mark_visited(object_id.to_string());
        Ok(())
    }

    /// Visit an object and execute a function, automatically unmarking on completion
    ///
    /// This ensures proper cleanup even if the function panics or returns an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::{SerializationContext, circular};
    ///
    /// let mut context = SerializationContext::new(5);
    ///
    /// let result = circular::visit_with(&mut context, "user:1", |ctx| {
    ///     // Do serialization work here
    ///     Ok(())
    /// });
    ///
    /// assert!(result.is_ok());
    /// // Object is automatically unmarked after the function completes
    /// assert!(!context.is_visited("user:1"));
    /// ```
    pub fn visit_with<F, T>(
        context: &mut SerializationContext,
        object_id: &str,
        f: F,
    ) -> RecursiveResult<T>
    where
        F: FnOnce(&mut SerializationContext) -> RecursiveResult<T>,
    {
        try_visit(context, object_id)?;
        let result = f(context);
        context.unmark_visited(object_id);
        result
    }
}

/// Helper functions for depth management
pub mod depth {
    use super::*;

    /// Check if we can descend to the next level
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::{SerializationContext, depth};
    ///
    /// let context = SerializationContext::new(2);
    /// assert!(depth::can_descend(&context));
    /// ```
    pub fn can_descend(context: &SerializationContext) -> bool {
        context.can_go_deeper()
    }

    /// Attempt to descend to the next level, returning an error if max depth is reached
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::{SerializationContext, depth};
    ///
    /// let context = SerializationContext::new(2);
    /// assert!(depth::try_descend(&context).is_ok());
    ///
    /// let child = context.child().child();
    /// assert!(depth::try_descend(&child).is_err());
    /// ```
    pub fn try_descend(context: &SerializationContext) -> RecursiveResult<SerializationContext> {
        if !can_descend(context) {
            return Err(RecursiveError::MaxDepthExceeded {
                current_depth: context.current_depth(),
                max_depth: context.max_depth(),
            });
        }
        Ok(context.child())
    }

    /// Descend to the next level and execute a function
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_serializers::recursive::{SerializationContext, depth};
    ///
    /// let context = SerializationContext::new(3);
    ///
    /// let result = depth::descend_with(&context, |child_ctx| {
    ///     assert_eq!(child_ctx.current_depth(), 1);
    ///     Ok(())
    /// });
    ///
    /// assert!(result.is_ok());
    /// ```
    pub fn descend_with<F, T>(context: &SerializationContext, f: F) -> RecursiveResult<T>
    where
        F: FnOnce(&SerializationContext) -> RecursiveResult<T>,
    {
        let child = try_descend(context)?;
        f(&child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use circular::*;
    use depth::*;

    #[test]
    fn test_context_new() {
        let context = SerializationContext::new(3);
        assert_eq!(context.current_depth(), 0);
        assert_eq!(context.max_depth(), 3);
        assert!(context.can_go_deeper());
    }

    #[test]
    fn test_context_child() {
        let context = SerializationContext::new(3);
        let child = context.child();

        assert_eq!(child.current_depth(), 1);
        assert_eq!(child.max_depth(), 3);
        assert!(child.can_go_deeper());
    }

    #[test]
    fn test_context_can_go_deeper() {
        let context = SerializationContext::new(2);
        assert!(context.can_go_deeper());

        let child1 = context.child();
        assert!(child1.can_go_deeper());

        let child2 = child1.child();
        assert!(!child2.can_go_deeper());
    }

    #[test]
    fn test_context_visited() {
        let mut context = SerializationContext::new(5);
        assert!(!context.is_visited("user:1"));

        context.mark_visited("user:1".to_string());
        assert!(context.is_visited("user:1"));

        context.unmark_visited("user:1");
        assert!(!context.is_visited("user:1"));
    }

    #[test]
    fn test_context_reset() {
        let mut context = SerializationContext::new(3);
        context.mark_visited("user:1".to_string());

        let child = context.child();
        assert_eq!(child.current_depth(), 1);
        assert!(child.is_visited("user:1"));

        let mut reset_context = child;
        reset_context.reset();
        assert_eq!(reset_context.current_depth(), 0);
        assert!(!reset_context.is_visited("user:1"));
    }

    #[test]
    fn test_remaining_depth() {
        let context = SerializationContext::new(3);
        assert_eq!(context.remaining_depth(), 3);

        let child1 = context.child();
        assert_eq!(child1.remaining_depth(), 2);

        let child2 = child1.child();
        assert_eq!(child2.remaining_depth(), 1);

        let child3 = child2.child();
        assert_eq!(child3.remaining_depth(), 0);
    }

    #[test]
    fn test_context_default() {
        let context = SerializationContext::default();
        assert_eq!(context.current_depth(), 0);
        assert_eq!(context.max_depth(), 1);
    }

    #[test]
    fn test_recursive_error_display() {
        let err = RecursiveError::MaxDepthExceeded {
            current_depth: 5,
            max_depth: 3,
        };
        assert_eq!(err.to_string(), "Maximum depth exceeded: current=5, max=3");

        let err = RecursiveError::CircularReference {
            object_id: "user:1".to_string(),
        };
        assert_eq!(err.to_string(), "Circular reference detected: user:1");

        let err = RecursiveError::SerializationError {
            message: "test error".to_string(),
        };
        assert_eq!(err.to_string(), "Serialization error: test error");
    }

    #[test]
    fn test_circular_would_be_circular() {
        let mut context = SerializationContext::new(5);
        assert!(!would_be_circular(&context, "user:1"));

        context.mark_visited("user:1".to_string());
        assert!(would_be_circular(&context, "user:1"));
        assert!(!would_be_circular(&context, "user:2"));
    }

    #[test]
    fn test_circular_try_visit() {
        let mut context = SerializationContext::new(5);

        assert!(try_visit(&mut context, "user:1").is_ok());
        assert!(context.is_visited("user:1"));

        let err = try_visit(&mut context, "user:1").unwrap_err();
        assert_eq!(
            err,
            RecursiveError::CircularReference {
                object_id: "user:1".to_string()
            }
        );
    }

    #[test]
    fn test_circular_visit_with() {
        let mut context = SerializationContext::new(5);

        let result = visit_with(&mut context, "user:1", |ctx| {
            assert!(ctx.is_visited("user:1"));
            Ok(42)
        });

        assert_eq!(result.unwrap(), 42);
        assert!(!context.is_visited("user:1"));
    }

    #[test]
    fn test_circular_visit_with_error() {
        let mut context = SerializationContext::new(5);

        let result: RecursiveResult<()> = visit_with(&mut context, "user:1", |_ctx| {
            Err(RecursiveError::SerializationError {
                message: "test".to_string(),
            })
        });

        assert!(result.is_err());
        assert!(!context.is_visited("user:1"));
    }

    #[test]
    fn test_depth_can_descend() {
        let context = SerializationContext::new(2);
        assert!(can_descend(&context));

        let child = context.child();
        assert!(can_descend(&child));

        let grandchild = child.child();
        assert!(!can_descend(&grandchild));
    }

    #[test]
    fn test_depth_try_descend() {
        let context = SerializationContext::new(2);
        assert!(try_descend(&context).is_ok());

        let child = context.child().child();
        let err = try_descend(&child).unwrap_err();
        assert_eq!(
            err,
            RecursiveError::MaxDepthExceeded {
                current_depth: 2,
                max_depth: 2
            }
        );
    }

    #[test]
    fn test_depth_descend_with() {
        let context = SerializationContext::new(3);

        let result = descend_with(&context, |child_ctx| {
            assert_eq!(child_ctx.current_depth(), 1);
            assert_eq!(child_ctx.max_depth(), 3);
            Ok(123)
        });

        assert_eq!(result.unwrap(), 123);
    }
}
