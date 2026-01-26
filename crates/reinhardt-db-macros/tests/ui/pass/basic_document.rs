use reinhardt_db_macros::{document, field};
use bson::oid::ObjectId;

#[document(collection = "users", backend = "mongodb")]
struct User {
    #[field(primary_key)]
    id: ObjectId,
    name: String,
    email: String,
}
