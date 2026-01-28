use reinhardt_db_macros::{document, field};
use bson::oid::ObjectId;

#[document(collection = "users", backend = "postgres")]
struct User {
    #[field(primary_key)]
    id: ObjectId,
}
