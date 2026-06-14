#![allow(unexpected_cfgs)]
//! Pass case: `#[model(manager = MyManager)]` sets `type Objects` so that
//! `Model::objects()` resolves to the user-supplied type. Issue #3980, #3984.

use reinhardt::Model;
use reinhardt::db::orm::custom_manager::CustomManager;
use reinhardt::model;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct ActiveUserManager;

impl CustomManager for ActiveUserManager {
	type Model = User;

	fn new() -> Self {
		Self
	}
}

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users", manager = ActiveUserManager)]
pub(crate) struct User {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 64)]
	pub username: String,
}

fn main() {
	let _: ActiveUserManager = User::objects();
}
