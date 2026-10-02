use reinhardt_db::orm::{FieldSelector, IntoPrimaryKey, Manager, Model};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct Peer {
	id: String,
}

#[derive(Clone)]
struct PeerFields;

impl FieldSelector for PeerFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for Peer {
	type PrimaryKey = String;
	type Fields = PeerFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"coherence_peers"
	}

	fn new_fields() -> Self::Fields {
		PeerFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		Some(self.id.clone())
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = value;
	}
}

impl IntoPrimaryKey<Peer> for String {
	fn into_primary_key(self) -> String {
		self
	}
}

impl IntoPrimaryKey<Peer> for &str {
	fn into_primary_key(self) -> String {
		self.to_owned()
	}
}

fn main() {}
