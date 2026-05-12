use std::num::NonZeroU16;

/// Configuration for connecting to a Kafka cluster.
#[derive(Debug, Clone)]
pub struct KafkaConfig {
	pub brokers: Vec<String>,
	pub client_id: String,
	/// Number of partitions to use when creating topics through this config.
	///
	/// Defaults to `1`. Tests that need deterministic partition pinning
	/// (e.g. to assert ordering or to address a specific partition via
	/// `KafkaProducer::send_to_partition`) can override this with
	/// [`KafkaConfig::with_partitions`].
	///
	/// Encoded as `NonZeroU16` to make zero — which would be a meaningless
	/// topic configuration — unrepresentable at the type level.
	pub partitions: NonZeroU16,
}

impl KafkaConfig {
	pub fn new(brokers: impl IntoIterator<Item = impl Into<String>>) -> Self {
		Self {
			brokers: brokers.into_iter().map(Into::into).collect(),
			client_id: "reinhardt".to_owned(),
			// SAFETY: 1 is non-zero.
			partitions: NonZeroU16::new(1).expect("1 is non-zero"),
		}
	}

	pub fn with_client_id(mut self, id: impl Into<String>) -> Self {
		self.client_id = id.into();
		self
	}

	/// Set the partition count to use when this config drives topic creation.
	pub fn with_partitions(mut self, n: NonZeroU16) -> Self {
		self.partitions = n;
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	fn default_client_id_is_reinhardt() {
		let config = KafkaConfig::new(["localhost:9092"]);
		assert_eq!(config.client_id, "reinhardt");
	}

	#[rstest]
	fn brokers_are_stored() {
		let config = KafkaConfig::new(["a:9092", "b:9092"]);
		assert_eq!(config.brokers, vec!["a:9092", "b:9092"]);
	}

	#[rstest]
	fn builder_overrides_client_id() {
		let config = KafkaConfig::new(["localhost:9092"]).with_client_id("my-app");
		assert_eq!(config.client_id, "my-app");
	}

	#[rstest]
	fn default_partitions_is_one() {
		let config = KafkaConfig::new(["localhost:9092"]);
		assert_eq!(config.partitions.get(), 1);
	}

	#[rstest]
	fn builder_overrides_partitions() {
		let config = KafkaConfig::new(["localhost:9092"])
			.with_partitions(NonZeroU16::new(4).expect("4 is non-zero"));
		assert_eq!(config.partitions.get(), 4);
	}
}
