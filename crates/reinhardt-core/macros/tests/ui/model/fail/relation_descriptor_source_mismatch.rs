use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(app_label = "users", table_name = "users")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	#[field(primary_key = true)]
	id: i64,
}

#[model(app_label = "posts", table_name = "posts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
	#[field(primary_key = true)]
	id: i64,
	#[rel(foreign_key)]
	author: db::associations::ForeignKeyField<User>,
}

#[model(app_label = "comments", table_name = "comments")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Comment {
	#[field(primary_key = true)]
	id: i64,
}

fn main() {
	let _invalid =
		db::orm::relations::RelationPath::<Comment, User>::from_descriptor::<PostAuthorRelationDescriptor>();
}
