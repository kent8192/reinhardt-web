use db::orm::Model as _;
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(app_label = "test")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlogPost {
	#[field(primary_key = true)]
	id: i64,
}

#[model(app_label = "test")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HTTPRoute {
	#[field(primary_key = true)]
	id: i64,
}

fn main() {
	assert_eq!(BlogPost::table_name(), "blog_post");
	assert_eq!(HTTPRoute::table_name(), "http_route");
}
