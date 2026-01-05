//! Admin macro with all options specified

use reinhardt_admin_core::ModelAdmin;
use reinhardt_macros::{admin, model};
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "profiles")]
#[derive(Serialize, Deserialize)]
struct Profile {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100)]
	user_id: String,

	#[field(max_length = 500, null = true)]
	bio: Option<String>,

	#[field(max_length = 255, null = true)]
	avatar_url: Option<String>,

	#[field(max_length = 100, null = true)]
	location: Option<String>,

	#[field(max_length = 255, null = true)]
	website: Option<String>,

	created_at: i64,

	updated_at: i64,
}

#[admin(model,
	for = Profile,
	name = "Profile",
	list_display = [id, user_id, bio, location, created_at],
	search_fields = [user_id, bio, location],
	list_filter = [created_at, updated_at],
	ordering = [(created_at, desc)],
	readonly_fields = [id, created_at, updated_at],
	fields = [user_id, bio, avatar_url, location, website],
	list_per_page = 50
)]
pub struct ProfileAdmin;

fn main() {
	// Compile test only - verify the macro expands with all options
	let admin = ProfileAdmin;
	assert_eq!(admin.model_name(), "Profile");
	assert_eq!(
		admin.list_display(),
		vec!["id", "user_id", "bio", "location", "created_at"]
	);
	assert_eq!(admin.search_fields(), vec!["user_id", "bio", "location"]);
	assert_eq!(admin.list_filter(), vec!["created_at", "updated_at"]);
	assert_eq!(admin.ordering(), vec!["-created_at"]);
	assert_eq!(
		admin.readonly_fields(),
		vec!["id", "created_at", "updated_at"]
	);
	assert_eq!(
		admin.fields(),
		Some(vec!["user_id", "bio", "avatar_url", "location", "website"])
	);
	assert_eq!(admin.list_per_page(), Some(50));
}
