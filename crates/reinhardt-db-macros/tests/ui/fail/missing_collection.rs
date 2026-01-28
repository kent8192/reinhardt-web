use reinhardt_db_macros::{document, field};
use bson::oid::ObjectId;

#[document(backend = "mongodb")]
struct User {
    #[field(primary_key)]
    id: ObjectId,
}
