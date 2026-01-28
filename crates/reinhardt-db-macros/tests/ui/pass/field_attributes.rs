use reinhardt_db_macros::{document, field};
use bson::oid::ObjectId;

#[document(collection = "users", backend = "mongodb")]
struct User {
    #[field(primary_key)]
    id: ObjectId,

    #[field(required, unique)]
    email: String,

    #[field(default = "Anonymous")]
    name: String,

    #[field(rename = "user_age")]
    age: i32,

    #[field(min = 0, max = 120)]
    score: i32,
}
