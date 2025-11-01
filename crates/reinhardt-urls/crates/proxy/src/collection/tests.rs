#[cfg(test)]
mod tests {
	use crate::collection::{CollectionAggregations, CollectionOperations, CollectionProxy};
	use crate::proxy::ScalarValue;
	use crate::reflection::Reflectable;
	use crate::{ProxyError, ProxyResult};
	use std::any::Any;
	use std::sync::Arc;

	struct TestParent {
		id: i64,
		children: Vec<TestChild>,
		// For set_values/append tests - dynamic collection
		dynamic_children: Option<Vec<Box<dyn Reflectable>>>,
	}

	// Manual Clone implementation (can't derive because Box<dyn Reflectable> isn't Clone)
	impl Clone for TestParent {
		fn clone(&self) -> Self {
			Self {
				id: self.id,
				children: self.children.clone(),
				// Don't clone dynamic_children - it's for mutable operations only
				dynamic_children: None,
			}
		}
	}

	#[derive(Clone)]
	struct TestChild {
		id: i64,
		value: i64,
		score: f64,
	}

	// Factory for creating TestChild instances
	struct TestChildFactory;

	impl crate::reflection::ReflectableFactory for TestChildFactory {
		fn create_from_scalar(
			&self,
			attribute_name: &str,
			value: ScalarValue,
		) -> ProxyResult<Box<dyn Reflectable>> {
			match attribute_name {
				"value" => {
					let val = value.as_integer()?;
					Ok(Box::new(TestChild {
						id: 0, // ID will be set by ORM in real implementation
						value: val,
						score: 0.0,
					}))
				}
				"score" => {
					let val = value.as_float()?;
					Ok(Box::new(TestChild {
						id: 0,
						value: 0,
						score: val,
					}))
				}
				_ => Err(ProxyError::AttributeNotFound(attribute_name.to_string())),
			}
		}
	}

	impl Reflectable for TestParent {
		fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
			match name {
				"children" => {
					let boxed: Vec<Box<dyn Reflectable>> = self
						.children
						.iter()
						.map(|c| Box::new(c.clone()) as Box<dyn Reflectable>)
						.collect();
					Some(Box::new(boxed))
				}
				_ => None,
			}
		}

		fn get_relationship_mut(&mut self, name: &str) -> Option<&mut dyn Any> {
			match name {
				"children" => {
					// Prioritize dynamic_children if present (for factory-based tests)
					if let Some(ref mut dynamic) = self.dynamic_children {
						Some(dynamic as &mut dyn Any)
					} else {
						Some(&mut self.children as &mut dyn Any)
					}
				}
				_ => None,
			}
		}

		fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
			match name {
				"id" => Some(ScalarValue::Integer(self.id)),
				_ => None,
			}
		}

		fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
			match name {
				"id" => {
					self.id = value.as_integer()?;
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

		fn as_any(&self) -> &dyn Any {
			self
		}
	}

	impl Reflectable for TestChild {
		fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any>> {
			None
		}

		fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
			None
		}

		fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
			match name {
				"id" => Some(ScalarValue::Integer(self.id)),
				"value" => Some(ScalarValue::Integer(self.value)),
				"score" => Some(ScalarValue::Float(self.score)),
				_ => None,
			}
		}

		fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
			match name {
				"id" => {
					self.id = value.as_integer()?;
					Ok(())
				}
				"value" => {
					self.value = value.as_integer()?;
					Ok(())
				}
				"score" => {
					self.score = value.as_float()?;
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

		fn as_any(&self) -> &dyn Any {
			self
		}
	}

	#[test]
	fn test_proxy_collection_creation_unit() {
		let proxy = CollectionProxy::new("posts", "title");
		assert_eq!(proxy.relationship, "posts");
		assert_eq!(proxy.attribute, "title");
		assert!(!proxy.unique);
	}

	#[test]
	fn test_proxy_collection_unique_unit() {
		let proxy = CollectionProxy::unique("posts", "title");
		assert!(proxy.unique);
	}

	#[test]
	fn test_collection_operations_creation() {
		let proxy = CollectionProxy::new("posts", "title");
		let _ops = CollectionOperations::new(proxy);
		// Operations wrapper is created successfully
	}

	#[test]
	fn test_collection_aggregations_creation() {
		let proxy = CollectionProxy::new("posts", "score");
		let _agg = CollectionAggregations::new(proxy);
		// Aggregations wrapper is created successfully
	}

	#[tokio::test]
	async fn test_collection_operations_filter() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 30,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let filtered = ops
			.filter(&parent, |v| matches!(v, ScalarValue::Integer(i) if *i > 15))
			.await
			.unwrap();

		assert_eq!(filtered.len(), 2);
		assert_eq!(filtered[0].as_integer().unwrap(), 20);
		assert_eq!(filtered[1].as_integer().unwrap(), 30);
	}

	#[tokio::test]
	async fn test_collection_operations_filter_all_match() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let filtered = ops
			.filter(&parent, |v| matches!(v, ScalarValue::Integer(_)))
			.await
			.unwrap();

		assert_eq!(filtered.len(), 2);
	}

	#[tokio::test]
	async fn test_collection_operations_filter_none_match() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let filtered = ops
			.filter(&parent, |v| matches!(v, ScalarValue::String(_)))
			.await
			.unwrap();

		assert_eq!(filtered.len(), 0);
	}

	#[tokio::test]
	async fn test_collection_operations_filter_empty_collection() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let filtered = ops
			.filter(&parent, |v| matches!(v, ScalarValue::Integer(_)))
			.await
			.unwrap();

		assert_eq!(filtered.len(), 0);
	}

	#[tokio::test]
	async fn test_collection_operations_filter_complex_condition() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 15,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 25,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let filtered = ops
			.filter(
				&parent,
				|v| matches!(v, ScalarValue::Integer(i) if *i >= 10 && *i <= 20),
			)
			.await
			.unwrap();

		assert_eq!(filtered.len(), 2);
	}

	#[tokio::test]
	async fn test_collection_operations_map() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let mapped: Vec<i64> = ops
			.map(&parent, |v| match v {
				ScalarValue::Integer(i) => i * 2,
				_ => 0,
			})
			.await
			.unwrap();

		assert_eq!(mapped.len(), 2);
		assert_eq!(mapped[0], 20);
		assert_eq!(mapped[1], 40);
	}

	#[tokio::test]
	async fn test_collection_operations_map_to_string() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let mapped: Vec<String> = ops
			.map(&parent, |v| match v {
				ScalarValue::Integer(i) => format!("Value: {}", i),
				_ => String::from("Unknown"),
			})
			.await
			.unwrap();

		assert_eq!(mapped.len(), 2);
		assert_eq!(mapped[0], "Value: 10");
		assert_eq!(mapped[1], "Value: 20");
	}

	#[tokio::test]
	async fn test_collection_operations_map_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let mapped: Vec<i64> = ops
			.map(&parent, |v| match v {
				ScalarValue::Integer(i) => i * 2,
				_ => 0,
			})
			.await
			.unwrap();

		assert_eq!(mapped.len(), 0);
	}

	#[tokio::test]
	async fn test_collection_operations_map_identity() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let mapped: Vec<i64> = ops
			.map(&parent, |v| match v {
				ScalarValue::Integer(i) => *i,
				_ => 0,
			})
			.await
			.unwrap();

		assert_eq!(mapped.len(), 2);
		assert_eq!(mapped[0], 10);
		assert_eq!(mapped[1], 20);
	}

	#[tokio::test]
	async fn test_collection_operations_map_constant() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let mapped: Vec<i64> = ops.map(&parent, |_| 42).await.unwrap();

		assert_eq!(mapped.len(), 2);
		assert_eq!(mapped[0], 42);
		assert_eq!(mapped[1], 42);
	}

	#[tokio::test]
	async fn test_collection_operations_sort() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 30,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 10,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 20,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let sorted = ops.sort(&parent).await.unwrap();

		assert_eq!(sorted.len(), 3);
		assert_eq!(sorted[0].as_integer().unwrap(), 10);
		assert_eq!(sorted[1].as_integer().unwrap(), 20);
		assert_eq!(sorted[2].as_integer().unwrap(), 30);
	}

	#[tokio::test]
	async fn test_collection_operations_sort_already_sorted() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 30,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let sorted = ops.sort(&parent).await.unwrap();

		assert_eq!(sorted.len(), 3);
		assert_eq!(sorted[0].as_integer().unwrap(), 10);
		assert_eq!(sorted[1].as_integer().unwrap(), 20);
		assert_eq!(sorted[2].as_integer().unwrap(), 30);
	}

	#[tokio::test]
	async fn test_collection_operations_sort_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let sorted = ops.sort(&parent).await.unwrap();
		assert_eq!(sorted.len(), 0);
	}

	#[tokio::test]
	async fn test_collection_operations_sort_single() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![TestChild {
				id: 1,
				value: 42,
				score: 1.0,
			}],
			dynamic_children: None,
		};

		let sorted = ops.sort(&parent).await.unwrap();

		assert_eq!(sorted.len(), 1);
		assert_eq!(sorted[0].as_integer().unwrap(), 42);
	}

	#[tokio::test]
	async fn test_collection_operations_sort_reverse_order() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 50,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 40,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 30,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let sorted = ops.sort(&parent).await.unwrap();

		assert_eq!(sorted.len(), 3);
		assert_eq!(sorted[0].as_integer().unwrap(), 30);
		assert_eq!(sorted[1].as_integer().unwrap(), 40);
		assert_eq!(sorted[2].as_integer().unwrap(), 50);
	}

	#[tokio::test]
	async fn test_collection_operations_distinct() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 10,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let distinct = ops.distinct(&parent).await.unwrap();

		assert_eq!(distinct.len(), 2);
		assert_eq!(distinct[0].as_integer().unwrap(), 10);
		assert_eq!(distinct[1].as_integer().unwrap(), 20);
	}

	#[tokio::test]
	async fn test_collection_operations_distinct_all_unique() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 30,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let distinct = ops.distinct(&parent).await.unwrap();

		assert_eq!(distinct.len(), 3);
	}

	#[tokio::test]
	async fn test_collection_operations_distinct_all_same() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 10,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 10,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let distinct = ops.distinct(&parent).await.unwrap();

		assert_eq!(distinct.len(), 1);
		assert_eq!(distinct[0].as_integer().unwrap(), 10);
	}

	#[tokio::test]
	async fn test_collection_operations_distinct_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let distinct = ops.distinct(&parent).await.unwrap();
		assert_eq!(distinct.len(), 0);
	}

	#[tokio::test]
	async fn test_collection_operations_distinct_multiple_duplicates() {
		let proxy = CollectionProxy::new("children", "value");
		let ops = CollectionOperations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 10,
					score: 3.0,
				},
				TestChild {
					id: 4,
					value: 20,
					score: 4.0,
				},
				TestChild {
					id: 5,
					value: 30,
					score: 5.0,
				},
			],
			dynamic_children: None,
		};

		let distinct = ops.distinct(&parent).await.unwrap();

		assert_eq!(distinct.len(), 3);
		assert_eq!(distinct[0].as_integer().unwrap(), 10);
		assert_eq!(distinct[1].as_integer().unwrap(), 20);
		assert_eq!(distinct[2].as_integer().unwrap(), 30);
	}

	#[tokio::test]
	async fn test_collection_aggregations_sum() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 30,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let sum = agg.sum(&parent).await.unwrap();
		assert_eq!(sum, 60.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_sum_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let sum = agg.sum(&parent).await.unwrap();
		assert_eq!(sum, 0.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_sum_floats() {
		let proxy = CollectionProxy::new("children", "score");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.5,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.5,
				},
			],
			dynamic_children: None,
		};

		let sum = agg.sum(&parent).await.unwrap();
		assert_eq!(sum, 4.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_sum_single() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![TestChild {
				id: 1,
				value: 42,
				score: 1.0,
			}],
			dynamic_children: None,
		};

		let sum = agg.sum(&parent).await.unwrap();
		assert_eq!(sum, 42.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_sum_negative() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: -10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let sum = agg.sum(&parent).await.unwrap();
		assert_eq!(sum, 10.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_avg() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 30,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let avg = agg.avg(&parent).await.unwrap();
		assert_eq!(avg, 20.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_avg_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let avg = agg.avg(&parent).await.unwrap();
		assert_eq!(avg, 0.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_avg_single() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![TestChild {
				id: 1,
				value: 42,
				score: 1.0,
			}],
			dynamic_children: None,
		};

		let avg = agg.avg(&parent).await.unwrap();
		assert_eq!(avg, 42.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_avg_floats() {
		let proxy = CollectionProxy::new("children", "score");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let avg = agg.avg(&parent).await.unwrap();
		assert_eq!(avg, 2.0);
	}

	#[tokio::test]
	async fn test_collection_aggregations_avg_decimal() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 15,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let avg = agg.avg(&parent).await.unwrap();
		assert_eq!(avg, 12.5);
	}

	#[tokio::test]
	async fn test_collection_aggregations_min() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 30,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 10,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 20,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let min = agg.min(&parent).await.unwrap();
		assert!(min.is_some());
		assert_eq!(min.unwrap().as_integer().unwrap(), 10);
	}

	#[tokio::test]
	async fn test_collection_aggregations_min_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let min = agg.min(&parent).await.unwrap();
		assert!(min.is_none());
	}

	#[tokio::test]
	async fn test_collection_aggregations_min_single() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![TestChild {
				id: 1,
				value: 42,
				score: 1.0,
			}],
			dynamic_children: None,
		};

		let min = agg.min(&parent).await.unwrap();
		assert!(min.is_some());
		assert_eq!(min.unwrap().as_integer().unwrap(), 42);
	}

	#[tokio::test]
	async fn test_collection_aggregations_min_negative() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: -10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 20,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let min = agg.min(&parent).await.unwrap();
		assert!(min.is_some());
		assert_eq!(min.unwrap().as_integer().unwrap(), -10);
	}

	#[tokio::test]
	async fn test_collection_aggregations_min_all_same() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 10,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let min = agg.min(&parent).await.unwrap();
		assert!(min.is_some());
		assert_eq!(min.unwrap().as_integer().unwrap(), 10);
	}

	#[tokio::test]
	async fn test_collection_aggregations_max() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 30,
					score: 2.0,
				},
				TestChild {
					id: 3,
					value: 20,
					score: 3.0,
				},
			],
			dynamic_children: None,
		};

		let max = agg.max(&parent).await.unwrap();
		assert!(max.is_some());
		assert_eq!(max.unwrap().as_integer().unwrap(), 30);
	}

	#[tokio::test]
	async fn test_collection_aggregations_max_empty() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: None,
		};

		let max = agg.max(&parent).await.unwrap();
		assert!(max.is_none());
	}

	#[tokio::test]
	async fn test_collection_aggregations_max_single() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![TestChild {
				id: 1,
				value: 42,
				score: 1.0,
			}],
			dynamic_children: None,
		};

		let max = agg.max(&parent).await.unwrap();
		assert!(max.is_some());
		assert_eq!(max.unwrap().as_integer().unwrap(), 42);
	}

	#[tokio::test]
	async fn test_collection_aggregations_max_negative() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: -20,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: -10,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let max = agg.max(&parent).await.unwrap();
		assert!(max.is_some());
		assert_eq!(max.unwrap().as_integer().unwrap(), -10);
	}

	#[tokio::test]
	async fn test_collection_aggregations_max_all_same() {
		let proxy = CollectionProxy::new("children", "value");
		let agg = CollectionAggregations::new(proxy);

		let parent = TestParent {
			id: 1,
			children: vec![
				TestChild {
					id: 1,
					value: 10,
					score: 1.0,
				},
				TestChild {
					id: 2,
					value: 10,
					score: 2.0,
				},
			],
			dynamic_children: None,
		};

		let max = agg.max(&parent).await.unwrap();
		assert!(max.is_some());
		assert_eq!(max.unwrap().as_integer().unwrap(), 10);
	}

	// Tests for set_values() with factory
	#[tokio::test]
	async fn test_set_values_with_factory() {
		let factory = Arc::new(TestChildFactory);
		let proxy = CollectionProxy::with_factory("children", "value", factory);

		let mut parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: Some(vec![]),
		};

		let new_values = vec![
			ScalarValue::Integer(10),
			ScalarValue::Integer(20),
			ScalarValue::Integer(30),
		];

		proxy.set_values(&mut parent, new_values).await.unwrap();

		// Verify collection was replaced
		let dynamic = parent.dynamic_children.as_ref().unwrap();
		assert_eq!(dynamic.len(), 3);
		assert_eq!(
			dynamic[0]
				.get_attribute("value")
				.unwrap()
				.as_integer()
				.unwrap(),
			10
		);
		assert_eq!(
			dynamic[1]
				.get_attribute("value")
				.unwrap()
				.as_integer()
				.unwrap(),
			20
		);
		assert_eq!(
			dynamic[2]
				.get_attribute("value")
				.unwrap()
				.as_integer()
				.unwrap(),
			30
		);
	}

	#[tokio::test]
	async fn test_set_values_without_factory() {
		let proxy = CollectionProxy::new("children", "value");

		let mut parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: Some(vec![]),
		};

		let new_values = vec![ScalarValue::Integer(42)];

		let result = proxy.set_values(&mut parent, new_values).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			ProxyError::FactoryNotConfigured
		));
	}

	#[tokio::test]
	async fn test_append_with_factory() {
		let factory = Arc::new(TestChildFactory);
		let proxy = CollectionProxy::with_factory("children", "value", factory);

		let mut parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: Some(vec![]),
		};

		proxy
			.append(&mut parent, ScalarValue::Integer(42))
			.await
			.unwrap();

		let dynamic = parent.dynamic_children.as_ref().unwrap();
		assert_eq!(dynamic.len(), 1);
		assert_eq!(
			dynamic[0]
				.get_attribute("value")
				.unwrap()
				.as_integer()
				.unwrap(),
			42
		);
	}

	#[tokio::test]
	async fn test_append_without_factory() {
		let proxy = CollectionProxy::new("children", "value");

		let mut parent = TestParent {
			id: 1,
			children: vec![],
			dynamic_children: Some(vec![]),
		};

		let result = proxy.append(&mut parent, ScalarValue::Integer(42)).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			ProxyError::FactoryNotConfigured
		));
	}

	#[tokio::test]
	async fn test_set_factory_method() {
		let mut proxy = CollectionProxy::new("children", "value");
		assert!(proxy.factory.is_none());

		let factory = Arc::new(TestChildFactory);
		proxy.set_factory(factory);
		assert!(proxy.factory.is_some());
	}

	#[tokio::test]
	async fn test_collection_proxy_debug_with_factory() {
		let factory = Arc::new(TestChildFactory);
		let proxy = CollectionProxy::with_factory("children", "value", factory);
		let debug_str = format!("{:?}", proxy);
		assert!(debug_str.contains("CollectionProxy"));
		assert!(debug_str.contains("factory"));
		assert!(debug_str.contains("Some(<factory>)"));
	}

	#[tokio::test]
	async fn test_collection_proxy_debug_without_factory() {
		let proxy = CollectionProxy::new("children", "value");
		let debug_str = format!("{:?}", proxy);
		assert!(debug_str.contains("CollectionProxy"));
		assert!(debug_str.contains("factory"));
		assert!(debug_str.contains("None"));
	}
}
