//! HMR protocol messages.

use serde::{Deserialize, Serialize};

use super::protocol::{
	BuildDiagnostic, BuildTarget, ClientHello, CompiledBuildId, DynamicAbiHash, PatchGeneration,
	PatchRejection, TemplateKey, TemplatePatch,
};

/// Messages sent from the HMR server to connected browser clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HmrMessage {
	/// A CSS file was updated - the client should replace the stylesheet.
	CssUpdate {
		/// The path of the changed CSS file, relative to the project root.
		path: String,
	},
	/// A non-CSS file was changed - the client should perform a full page reload.
	FullReload {
		/// Human-readable reason for the reload.
		reason: String,
	},
	/// Initial connection acknowledgment.
	Connected,
	/// Client identity and dynamic ABI inventory sent after connection.
	ClientHello {
		/// Build currently loaded by the browser.
		build_id: CompiledBuildId,
		/// Template manifest currently loaded by the browser.
		manifest_digest: [u8; 32],
		/// Dynamic ABI known for each compiled template.
		abi_hashes: Vec<(TemplateKey, DynamicAbiHash)>,
	},
	/// A compatible collection of template patches.
	TemplatePatchBatch {
		/// Compiled build expected by the patch.
		build_id: CompiledBuildId,
		/// Template manifest expected by the patch.
		manifest_digest: [u8; 32],
		/// Source generation represented by the patch.
		generation: PatchGeneration,
		/// Compatible template replacements.
		patches: Vec<TemplatePatch>,
	},
	/// Acknowledges a successfully applied template patch batch.
	PatchApplied {
		/// Applied source generation.
		generation: PatchGeneration,
	},
	/// Reports a rejected template patch batch.
	PatchRejected {
		/// Rejected source generation.
		generation: PatchGeneration,
		/// Typed rejection reason.
		reason: PatchRejection,
	},
	/// Announces a build that is currently in progress.
	BuildStarted {
		/// Source generation being built.
		generation: PatchGeneration,
		/// Build targets included in this run.
		targets: Vec<BuildTarget>,
	},
	/// Sends normalized diagnostics for the current build.
	BuildDiagnostics {
		/// Source generation represented by these diagnostics.
		generation: PatchGeneration,
		/// Diagnostics grouped into this update.
		diagnostics: Vec<BuildDiagnostic>,
	},
	/// Announces that the development build has recovered.
	BuildRecovered {
		/// Recovered source generation.
		generation: PatchGeneration,
	},
}

impl From<ClientHello> for HmrMessage {
	fn from(hello: ClientHello) -> Self {
		Self::ClientHello {
			build_id: hello.build_id,
			manifest_digest: hello.manifest_digest,
			abi_hashes: hello.abi_hashes,
		}
	}
}

impl HmrMessage {
	/// Returns a typed client hello when this message came from a browser.
	pub fn client_hello(&self) -> Option<ClientHello> {
		match self {
			Self::ClientHello {
				build_id,
				manifest_digest,
				abi_hashes,
			} => Some(ClientHello {
				build_id: *build_id,
				manifest_digest: *manifest_digest,
				abi_hashes: abi_hashes.clone(),
			}),
			_ => None,
		}
	}
}

impl HmrMessage {
	/// Serializes the message to a JSON string.
	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string(self)
	}

	/// Deserializes a message from a JSON string.
	pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(json)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::hmr::protocol::{
		DiagnosticLevel, DiagnosticSpan, DiagnosticTarget, StaticTemplateNode,
	};
	use rstest::rstest;

	#[rstest]
	fn test_css_update_serialization() {
		// Arrange
		let msg = HmrMessage::CssUpdate {
			path: "styles/main.css".to_string(),
		};

		// Act
		let json = msg.to_json().unwrap();
		let deserialized = HmrMessage::from_json(&json).unwrap();

		// Assert
		assert_eq!(msg, deserialized);
		assert!(json.contains("\"type\":\"css_update\""));
		assert!(json.contains("\"path\":\"styles/main.css\""));
	}

	#[rstest]
	fn test_full_reload_serialization() {
		// Arrange
		let msg = HmrMessage::FullReload {
			reason: "Rust source changed".to_string(),
		};

		// Act
		let json = msg.to_json().unwrap();
		let deserialized = HmrMessage::from_json(&json).unwrap();

		// Assert
		assert_eq!(msg, deserialized);
		assert!(json.contains("\"type\":\"full_reload\""));
		assert!(json.contains("\"reason\":\"Rust source changed\""));
	}

	#[rstest]
	fn test_connected_serialization() {
		// Arrange
		let msg = HmrMessage::Connected;

		// Act
		let json = msg.to_json().unwrap();
		let deserialized = HmrMessage::from_json(&json).unwrap();

		// Assert
		assert_eq!(msg, deserialized);
		assert!(json.contains("\"type\":\"connected\""));
	}

	#[rstest]
	fn test_deserialization_from_raw_json() {
		// Arrange
		let json = r#"{"type":"css_update","path":"app.css"}"#;

		// Act
		let msg = HmrMessage::from_json(json).unwrap();

		// Assert
		assert_eq!(
			msg,
			HmrMessage::CssUpdate {
				path: "app.css".to_string()
			}
		);
	}

	// --- Error cases ---

	#[rstest]
	fn test_from_json_invalid_json_returns_err() {
		// Arrange
		let json = "not valid json at all {{";

		// Act
		let result = HmrMessage::from_json(json);

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	fn test_from_json_unknown_type_returns_err() {
		// Arrange — `type` field exists but is not a known variant
		let json = r#"{"type":"hot_update","data":"something"}"#;

		// Act
		let result = HmrMessage::from_json(json);

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	fn test_from_json_missing_type_returns_err() {
		// Arrange
		let json = r#"{"path":"app.css"}"#;

		// Act
		let result = HmrMessage::from_json(json);

		// Assert
		assert!(result.is_err());
	}

	// --- Edge cases ---

	#[rstest]
	fn test_css_update_empty_path() {
		// Arrange
		let msg = HmrMessage::CssUpdate {
			path: String::new(),
		};

		// Act
		let json = msg.to_json().unwrap();
		let deserialized = HmrMessage::from_json(&json).unwrap();

		// Assert
		assert_eq!(msg, deserialized);
		assert!(json.contains("\"path\":\"\""));
	}

	#[rstest]
	fn test_css_update_unicode_path() {
		// Arrange
		let msg = HmrMessage::CssUpdate {
			path: "スタイル/main.css".to_string(),
		};

		// Act
		let json = msg.to_json().unwrap();
		let deserialized = HmrMessage::from_json(&json).unwrap();

		// Assert
		assert_eq!(msg, deserialized);
	}

	#[rstest]
	fn test_full_reload_empty_reason() {
		// Arrange
		let msg = HmrMessage::FullReload {
			reason: String::new(),
		};

		// Act
		let json = msg.to_json().unwrap();
		let deserialized = HmrMessage::from_json(&json).unwrap();

		// Assert
		assert_eq!(msg, deserialized);
	}

	#[rstest]
	fn test_hmr_message_clone_and_eq() {
		let msg = HmrMessage::CssUpdate {
			path: "a.css".to_string(),
		};
		let cloned = msg.clone();
		assert_eq!(msg, cloned);
	}

	#[rstest]
	fn test_template_protocol_messages_round_trip() {
		let key = TemplateKey {
			source_id: crate::hmr::SourceId("src/page.rs".to_owned()),
			line: 4,
			column: 8,
			nested_template_index: 0,
		};
		let patch = TemplatePatch {
			key: key.clone(),
			abi_hash: DynamicAbiHash([2; 32]),
			static_tree: StaticTemplateNode::Text("updated".to_owned()),
			placements: Vec::new(),
		};
		let messages = vec![
			HmrMessage::ClientHello {
				build_id: CompiledBuildId([1; 32]),
				manifest_digest: [3; 32],
				abi_hashes: vec![(key.clone(), DynamicAbiHash([2; 32]))],
			},
			HmrMessage::TemplatePatchBatch {
				build_id: CompiledBuildId([1; 32]),
				manifest_digest: [3; 32],
				generation: PatchGeneration(9),
				patches: vec![patch],
			},
			HmrMessage::PatchApplied {
				generation: PatchGeneration(9),
			},
			HmrMessage::PatchRejected {
				generation: PatchGeneration(9),
				reason: PatchRejection::PlacementFailure,
			},
			HmrMessage::BuildStarted {
				generation: PatchGeneration(9),
				targets: vec![BuildTarget::Server, BuildTarget::Wasm],
			},
			HmrMessage::BuildDiagnostics {
				generation: PatchGeneration(9),
				diagnostics: vec![BuildDiagnostic {
					generation: PatchGeneration(9),
					target: DiagnosticTarget::WasmRustc,
					level: DiagnosticLevel::Error,
					message: "failed".to_owned(),
					code: Some("E0001".to_owned()),
					rendered: "error[E0001]".to_owned(),
					relative_spans: vec![DiagnosticSpan {
						file_name: "src/page.rs".to_owned(),
						line_start: 4,
						line_end: 4,
						column_start: 8,
						column_end: 9,
						is_primary: true,
						label: None,
					}],
				}],
			},
			HmrMessage::BuildRecovered {
				generation: PatchGeneration(9),
			},
		];

		for message in messages {
			let json = message.to_json().expect("serialize");
			let round_trip = HmrMessage::from_json(&json).expect("deserialize");
			assert_eq!(message, round_trip);
		}
	}

	#[rstest]
	fn test_client_hello_conversion_preserves_identity() {
		let hello = ClientHello {
			build_id: CompiledBuildId([4; 32]),
			manifest_digest: [5; 32],
			abi_hashes: Vec::new(),
		};
		let message = HmrMessage::from(hello.clone());
		assert_eq!(message.client_hello(), Some(hello));
	}
}
