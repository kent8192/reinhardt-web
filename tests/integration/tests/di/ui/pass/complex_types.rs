use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
struct ComplexService {
	vec_data: Vec<String>,
	map_data: HashMap<String, i32>,
	optional_data: Option<String>,
}

#[async_trait::async_trait]
impl Injectable for ComplexService {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		let mut map = HashMap::new();
		map.insert("key1".to_string(), 100);
		map.insert("key2".to_string(), 200);

		Ok(Self {
			vec_data: vec!["item1".to_string(), "item2".to_string()],
			map_data: map,
			optional_data: Some("optional".to_string()),
		})
	}
}

#[tokio::main]
async fn main() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let service = ComplexService::inject(&ctx).await.unwrap();
	assert_eq!(service.vec_data.len(), 2);
	assert_eq!(service.map_data.get("key1"), Some(&100));
	assert_eq!(service.optional_data, Some("optional".to_string()));
}
