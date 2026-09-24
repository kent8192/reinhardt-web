//! RS256 signing boundary. A host can provide a non-exportable KMS signer.

use super::store::PublicRsaJwk;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rsa::{
	RsaPrivateKey, RsaPublicKey,
	pkcs1::DecodeRsaPrivateKey,
	pkcs1v15::SigningKey,
	pkcs8::DecodePrivateKey,
	signature::{SignatureEncoding, Signer},
	traits::PublicKeyParts,
};
use sha2::Sha256;
use std::{collections::HashMap, sync::RwLock};

/// Host signing capability; private key bytes never pass through the state store.
#[async_trait]
pub trait OidcSigner: Send + Sync {
	/// Return the public RSA parameters for a provisioned key identifier.
	async fn public_key(&self, kid: &str) -> Result<Option<PublicRsaJwk>, String>;
	/// Sign a JWS signing input with RS256 and return the raw signature.
	async fn sign(&self, kid: &str, input: &[u8]) -> Result<Vec<u8>, String>;
}

struct KeyMaterial {
	public: PublicRsaJwk,
	signing: SigningKey<Sha256>,
}

/// Development or host-managed RSA PEM key ring.
///
/// Each production node must receive the same provisioned key material before
/// the corresponding key is activated in the shared OIDC state store.
#[derive(Default)]
pub struct RsaPemKeyRing {
	keys: RwLock<HashMap<String, KeyMaterial>>,
}

impl RsaPemKeyRing {
	/// Create an empty key ring.
	pub fn new() -> Self {
		Self::default()
	}

	/// Provision a PKCS#1 or PKCS#8 PEM private key under a unique identifier.
	pub fn add_private_key_pem(&self, kid: &str, pem: &str) -> Result<PublicRsaJwk, String> {
		if kid.is_empty() || kid.len() > 128 || !kid.is_ascii() {
			return Err("invalid key identifier".to_owned());
		}
		let key = RsaPrivateKey::from_pkcs8_pem(pem)
			.or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))
			.map_err(|_| "invalid RSA private key".to_owned())?;
		let public = RsaPublicKey::from(&key);
		if public.n().bits() < 2048 {
			return Err("RSA key must have at least 2048 bits".to_owned());
		}
		let public = PublicRsaJwk {
			kid: kid.to_owned(),
			n: URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()),
			e: URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()),
		};
		let mut keys = self.keys.write().map_err(|_| "key ring lock poisoned")?;
		if keys.contains_key(kid) {
			return Err("key identifier already provisioned".to_owned());
		}
		keys.insert(
			kid.to_owned(),
			KeyMaterial {
				public: public.clone(),
				signing: SigningKey::<Sha256>::new(key),
			},
		);
		Ok(public)
	}
}

#[async_trait]
impl OidcSigner for RsaPemKeyRing {
	async fn public_key(&self, kid: &str) -> Result<Option<PublicRsaJwk>, String> {
		Ok(self
			.keys
			.read()
			.map_err(|_| "key ring lock poisoned")?
			.get(kid)
			.map(|key| key.public.clone()))
	}

	async fn sign(&self, kid: &str, input: &[u8]) -> Result<Vec<u8>, String> {
		let keys = self.keys.read().map_err(|_| "key ring lock poisoned")?;
		let key = keys.get(kid).ok_or("signing key unavailable")?;
		Ok(key.signing.sign(input).to_vec())
	}
}
