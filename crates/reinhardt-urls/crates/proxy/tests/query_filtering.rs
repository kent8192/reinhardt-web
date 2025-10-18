//! Tests for query filtering functionality

use reinhardt_proxy::{
    CollectionProxy, FilterCondition, FilterOp, ProxyResult, Reflectable, ScalarValue,
};
use std::any::Any;

/// Test item that can be cloned
#[derive(Debug, Clone)]
struct TestItem {
    name: String,
    age: i64,
    score: f64,
}

impl Reflectable for TestItem {
    fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any>> {
        None
    }

    fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
        None
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "name" => Some(ScalarValue::String(self.name.clone())),
            "age" => Some(ScalarValue::Integer(self.age)),
            "score" => Some(ScalarValue::Float(self.score)),
            _ => None,
        }
    }

    fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
        match name {
            "name" => {
                if let ScalarValue::String(s) = value {
                    self.name = s;
                    Ok(())
                } else {
                    Err(reinhardt_proxy::ProxyError::TypeMismatch {
                        expected: "String".to_string(),
                        actual: format!("{:?}", value),
                    })
                }
            }
            "age" => {
                if let ScalarValue::Integer(i) = value {
                    self.age = i;
                    Ok(())
                } else {
                    Err(reinhardt_proxy::ProxyError::TypeMismatch {
                        expected: "Integer".to_string(),
                        actual: format!("{:?}", value),
                    })
                }
            }
            _ => Err(reinhardt_proxy::ProxyError::AttributeNotFound(
                name.to_string(),
            )),
        }
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        _attribute: &str,
        _value: ScalarValue,
    ) -> ProxyResult<()> {
        Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
            relationship.to_string(),
        ))
    }
}

/// Mock model for testing
struct TestModel {
    name: String,
    age: i64,
    score: f64,
    items: Vec<TestItem>,
}

impl Reflectable for TestModel {
    fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
        match name {
            "items" => {
                // Clone the items vector and box them
                let items: Vec<Box<dyn Reflectable>> = self
                    .items
                    .iter()
                    .map(|item| Box::new(item.clone()) as Box<dyn Reflectable>)
                    .collect();
                Some(Box::new(items))
            }
            _ => None,
        }
    }

    fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
        // Mutable relationships not supported in this test implementation
        None
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "name" => Some(ScalarValue::String(self.name.clone())),
            "age" => Some(ScalarValue::Integer(self.age)),
            "score" => Some(ScalarValue::Float(self.score)),
            _ => None,
        }
    }

    fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
        match name {
            "name" => {
                if let ScalarValue::String(s) = value {
                    self.name = s;
                    Ok(())
                } else {
                    Err(reinhardt_proxy::ProxyError::TypeMismatch {
                        expected: "String".to_string(),
                        actual: format!("{:?}", value),
                    })
                }
            }
            "age" => {
                if let ScalarValue::Integer(i) = value {
                    self.age = i;
                    Ok(())
                } else {
                    Err(reinhardt_proxy::ProxyError::TypeMismatch {
                        expected: "Integer".to_string(),
                        actual: format!("{:?}", value),
                    })
                }
            }
            _ => Err(reinhardt_proxy::ProxyError::AttributeNotFound(
                name.to_string(),
            )),
        }
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        _attribute: &str,
        _value: ScalarValue,
    ) -> ProxyResult<()> {
        Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
            relationship.to_string(),
        ))
    }
}

fn create_test_items() -> Vec<TestItem> {
    vec![
        TestItem {
            name: "Alice".to_string(),
            age: 25,
            score: 95.5,
        },
        TestItem {
            name: "Bob".to_string(),
            age: 30,
            score: 87.3,
        },
        TestItem {
            name: "Charlie".to_string(),
            age: 25,
            score: 92.1,
        },
        TestItem {
            name: "Alice".to_string(),
            age: 35,
            score: 88.8,
        },
    ]
}

#[tokio::test]
async fn test_filter_eq() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new("name", FilterOp::eq("Alice"));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results
        .iter()
        .all(|v| v == &ScalarValue::String("Alice".to_string())));
}

#[tokio::test]
async fn test_filter_ne() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new("name", FilterOp::ne("Alice"));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.contains(&ScalarValue::String("Bob".to_string())));
    assert!(results.contains(&ScalarValue::String("Charlie".to_string())));
}

#[tokio::test]
async fn test_filter_gt_integer() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "age");
    let condition = FilterCondition::new("age", FilterOp::gt(25i64));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.contains(&ScalarValue::Integer(30)));
    assert!(results.contains(&ScalarValue::Integer(35)));
}

#[tokio::test]
async fn test_filter_gte() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "age");
    let condition = FilterCondition::new("age", FilterOp::gte(30i64));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.contains(&ScalarValue::Integer(30)));
    assert!(results.contains(&ScalarValue::Integer(35)));
}

#[tokio::test]
async fn test_filter_lt() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "age");
    let condition = FilterCondition::new("age", FilterOp::lt(30i64));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|v| v == &ScalarValue::Integer(25)));
}

#[tokio::test]
async fn test_filter_lte() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "age");
    let condition = FilterCondition::new("age", FilterOp::lte(30i64));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(
        results
            .iter()
            .filter(|v| **v == ScalarValue::Integer(25))
            .count(),
        2
    );
    assert_eq!(
        results
            .iter()
            .filter(|v| **v == ScalarValue::Integer(30))
            .count(),
        1
    );
}

#[tokio::test]
async fn test_filter_in() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new(
        "name",
        FilterOp::in_values(vec![
            ScalarValue::String("Alice".to_string()),
            ScalarValue::String("Charlie".to_string()),
        ]),
    );
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(
        results
            .iter()
            .filter(|v| **v == ScalarValue::String("Alice".to_string()))
            .count(),
        2
    );
    assert_eq!(
        results
            .iter()
            .filter(|v| **v == ScalarValue::String("Charlie".to_string()))
            .count(),
        1
    );
}

#[tokio::test]
async fn test_filter_not_in() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new(
        "name",
        FilterOp::not_in_values(vec![
            ScalarValue::String("Alice".to_string()),
            ScalarValue::String("Charlie".to_string()),
        ]),
    );
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], ScalarValue::String("Bob".to_string()));
}

#[tokio::test]
async fn test_filter_contains() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new("name", FilterOp::contains("li"));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 3); // Alice (2x) and Charlie
    assert!(results
        .iter()
        .any(|v| v == &ScalarValue::String("Alice".to_string())));
    assert!(results
        .iter()
        .any(|v| v == &ScalarValue::String("Charlie".to_string())));
}

#[tokio::test]
async fn test_filter_starts_with() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new("name", FilterOp::starts_with("A"));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results
        .iter()
        .all(|v| v == &ScalarValue::String("Alice".to_string())));
}

#[tokio::test]
async fn test_filter_ends_with() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new("name", FilterOp::ends_with("e"));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 3); // Alice (2x) and Charlie
}

#[tokio::test]
async fn test_filter_by_custom_predicate() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "score");
    let results = proxy
        .filter_by(&model, |v| {
            if let ScalarValue::Float(f) = v {
                *f > 90.0
            } else {
                false
            }
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2); // 95.5 and 92.1
}

#[tokio::test]
async fn test_filter_gt_float() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "score");
    let condition = FilterCondition::new("score", FilterOp::gt(90.0));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.contains(&ScalarValue::Float(95.5)));
    assert!(results.contains(&ScalarValue::Float(92.1)));
}

#[tokio::test]
async fn test_filter_empty_result() {
    let model = TestModel {
        name: "Parent".to_string(),
        age: 40,
        score: 100.0,
        items: create_test_items(),
    };

    let proxy = CollectionProxy::new("items", "name");
    let condition = FilterCondition::new("name", FilterOp::eq("NonExistent"));
    let results = proxy.filter(&model, condition).await.unwrap();

    assert_eq!(results.len(), 0);
}
