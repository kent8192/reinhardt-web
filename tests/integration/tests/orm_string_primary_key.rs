//! Regression coverage for text primary keys in generated relationship builders.

use reinhardt::db::associations::ForeignKeyField;
use reinhardt::db::orm::IntoPrimaryKey;
use reinhardt::model;
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};

#[model(app_label = "string_pk", table_name = "string_pk_peers")]
#[derive(Serialize, Deserialize)]
struct Peer {
	#[field(primary_key = true, max_length = 255)]
	id: String,
}

#[model(app_label = "string_pk", table_name = "string_pk_connections")]
#[derive(Serialize, Deserialize)]
struct Connection {
	#[field(primary_key = true)]
	id: Option<i64>,

	#[rel(foreign_key, related_name = "connections")]
	node: ForeignKeyField<Peer>,
}

#[fixture]
fn peer() -> Peer {
	Peer::build().id("peer-001").finish()
}

#[rstest]
#[case("peer-001")]
#[case("000123")]
#[case("東京/peer")]
#[case("")]
#[case(" peer-001 ")]
fn foreign_key_builder_accepts_owned_text_key(#[case] key: &str) {
	let connection = Connection::build().node(key.to_owned()).finish();
	assert_eq!(connection.node_id(), key);
}

#[rstest]
#[case("peer-001")]
#[case("000123")]
#[case("東京/peer")]
#[case("")]
#[case(" peer-001 ")]
fn foreign_key_builder_owns_borrowed_text_key(#[case] key: &str) {
	let connection = {
		// Arrange
		let owned_key = key.to_owned();

		// Act
		Connection::build().node(owned_key.as_str()).finish()
	};

	// Assert
	assert_eq!(connection.node_id(), key);
}

#[rstest]
fn foreign_key_builder_preserves_model_reference_conversion(peer: Peer) {
	let connection = Connection::build().node(&peer).finish();
	assert_eq!(connection.node_id(), "peer-001");
	assert_eq!(peer.id(), "peer-001");
}

#[rstest]
#[case::owned(String::from("peer-001"))]
#[case::borrowed("peer-001")]
fn info_builder_accepts_text_key<Key: reinhardt::db::orm::IntoPrimaryKey<Peer>>(#[case] key: Key) {
	let info = ConnectionInfo::build().id(None).node(key).finish();
	assert_eq!(info.node.id, "peer-001");
}

struct NormalizedPeerKey<'a>(&'a str);

impl IntoPrimaryKey<Peer> for NormalizedPeerKey<'_> {
	fn into_primary_key(self) -> String {
		self.0.trim().to_owned()
	}
}

#[rstest]
#[case(" peer-001 ", "peer-001")]
#[case(" 東京/peer ", "東京/peer")]
#[case(" 000123 ", "000123")]
fn custom_text_key_conversion_uses_a_downstream_newtype(
	#[case] input: &str,
	#[case] expected: &str,
) {
	// Arrange
	let key = NormalizedPeerKey(input);

	// Act
	let connection = Connection::build().node(key).finish();
	let info = ConnectionInfo::build()
		.id(None)
		.node(NormalizedPeerKey(input))
		.finish();

	// Assert
	assert_eq!(connection.node_id(), expected);
	assert_eq!(info.node.id, expected);
}
