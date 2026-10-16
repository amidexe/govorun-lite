//! RFC 9474 client port. Only blinded messages cross the issuance boundary;
//! snapshots contain secrets and belong in the existing protected local vault.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use blind_rsa_signatures::{
    reexports::{
        crypto_bigint::{modular::BoxedMontyForm, BoxedUint},
        rsa::{self, pkcs8::DecodePublicKey, traits::PublicKeyParts},
    },
    BlindMessage, BlindSignature, BlindingResult, DefaultRng, MessageRandomizer,
    PublicKeySha384PSSRandomized as PublicKey, Secret,
};
use serde::{Deserialize, Serialize};

#[path = "style_pass_crypto/android_material.rs"]
pub(crate) mod android_material;

pub const ALGORITHM: &str = "RSABSSA-SHA384-PSS-Randomized";
const MAX_MILLIS: u64 = 253_402_300_799_999;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CryptoError {
    InvalidKey,
    InvalidExpiry,
    InvalidRequest,
    InvalidSignature,
    RandomUnavailable,
}
pub type Result<T> = std::result::Result<T, CryptoError>;

/// No Debug implementation: a persisted request must never appear in logs.
/// Binary fields keep the existing big-endian bytes, encoded without padding.
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestSnapshot {
    pub key_id: String,
    pub prepared_message: String,
    pub blinded_message: String,
    pub encoded_message: String,
    pub inverse: String,
    pub modulus: String,
    pub exponent: String,
    pub key_not_before_millis: u64,
    pub key_not_after_millis: u64,
}

pub struct StylePassKey {
    id: String,
    key: PublicKey,
    not_before: u64,
    not_after: u64,
}

impl StylePassKey {
    pub fn from_spki(
        id: &str,
        algorithm: &str,
        spki: &[u8],
        not_before: u64,
        not_after: u64,
    ) -> Result<Self> {
        if algorithm != ALGORITHM || spki.len() > 2048 {
            return Err(CryptoError::InvalidKey);
        }
        let key =
            rsa::RsaPublicKey::from_public_key_der(spki).map_err(|_| CryptoError::InvalidKey)?;
        Self::new(id, key, not_before, not_after)
    }

    pub fn from_components(
        id: &str,
        modulus: &[u8],
        exponent: &[u8],
        not_before: u64,
        not_after: u64,
    ) -> Result<Self> {
        if !(256..=1024).contains(&modulus.len()) || !(1..=16).contains(&exponent.len()) {
            return Err(CryptoError::InvalidKey);
        }
        let n = BoxedUint::from_be_slice_vartime(modulus);
        let e = BoxedUint::from_be_slice_vartime(exponent);
        let key = rsa::RsaPublicKey::new(n, e).map_err(|_| CryptoError::InvalidKey)?;
        Self::new(id, key, not_before, not_after)
    }

    fn new(id: &str, key: rsa::RsaPublicKey, not_before: u64, not_after: u64) -> Result<Self> {
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || key.n().bits() < 2048
            || key.n().bits() > 8192
            || not_before >= not_after
            || not_after > MAX_MILLIS
        {
            return Err(CryptoError::InvalidKey);
        }
        Ok(Self {
            id: id.into(),
            key: PublicKey::new(key),
            not_before,
            not_after,
        })
    }

    /// Low-level RFC primitive; callers normally use the constrained port below.
    /// Injectable randomness permits exact RFC vectors without a test-only
    /// cryptographic implementation or platform branches in the business core.
    pub fn blind<R: rsa::rand_core::CryptoRng + ?Sized>(
        &self,
        message: &[u8],
        rng: &mut R,
    ) -> Result<BlindRequest> {
        if message.is_empty() || message.len() > 224 {
            return Err(CryptoError::InvalidRequest);
        }
        let result = self
            .key
            .blind(rng, message)
            .map_err(|_| CryptoError::InvalidRequest)?;
        let prefix = result.msg_randomizer.ok_or(CryptoError::InvalidRequest)?;
        let prepared = [prefix.0.as_slice(), message].concat();
        let encoded = recover_encoded(&self.key, &result.blind_message, &result.secret)?;
        let raw: &rsa::RsaPublicKey = self.key.as_ref();
        let snapshot = RequestSnapshot {
            key_id: self.id.clone(),
            prepared_message: B64.encode(prepared),
            blinded_message: B64.encode(&result.blind_message),
            encoded_message: B64.encode(encoded),
            inverse: B64.encode(&result.secret),
            modulus: B64.encode(raw.n().to_be_bytes()),
            exponent: B64.encode(minimal(&raw.e().to_be_bytes())),
            key_not_before_millis: self.not_before,
            key_not_after_millis: self.not_after,
        };
        Ok(BlindRequest {
            snapshot,
            key: self.key.clone(),
            result,
        })
    }
}

pub trait StylePassCryptoPort {
    fn prepare(
        &self,
        key: &StylePassKey,
        expires_at_millis: u64,
        now_millis: u64,
    ) -> Result<BlindRequest>;
    fn restore(&self, snapshot: RequestSnapshot) -> Result<BlindRequest>;
}
pub struct RustStylePassCrypto;
impl StylePassCryptoPort for RustStylePassCrypto {
    fn prepare(
        &self,
        key: &StylePassKey,
        expires_at_millis: u64,
        now_millis: u64,
    ) -> Result<BlindRequest> {
        let message = pass_message(key, expires_at_millis, now_millis)?;
        key.blind(&message, &mut DefaultRng)
    }
    fn restore(&self, snapshot: RequestSnapshot) -> Result<BlindRequest> {
        BlindRequest::restore(snapshot)
    }
}

pub struct BlindRequest {
    snapshot: RequestSnapshot,
    key: PublicKey,
    result: BlindingResult,
}
impl BlindRequest {
    pub fn snapshot(&self) -> RequestSnapshot {
        self.snapshot.clone()
    }
    pub fn blinded_message(&self) -> &[u8] {
        self.result.blind_message.as_ref()
    }

    pub fn restore(snapshot: RequestSnapshot) -> Result<Self> {
        let modulus = decode(&snapshot.modulus, 1024)?;
        let exponent = decode(&snapshot.exponent, 16)?;
        let key = StylePassKey::from_components(
            &snapshot.key_id,
            &modulus,
            &exponent,
            snapshot.key_not_before_millis,
            snapshot.key_not_after_millis,
        )?
        .key;
        let prepared = decode(&snapshot.prepared_message, 256)?;
        if prepared.len() < 64 {
            return Err(CryptoError::InvalidRequest);
        }
        let result = BlindingResult {
            blind_message: BlindMessage(decode(&snapshot.blinded_message, modulus.len())?),
            secret: Secret(decode(&snapshot.inverse, modulus.len())?),
            msg_randomizer: Some(MessageRandomizer(
                prepared[..32]
                    .try_into()
                    .map_err(|_| CryptoError::InvalidRequest)?,
            )),
        };
        let encoded = decode(&snapshot.encoded_message, modulus.len())?;
        if encoded != recover_encoded(&key, &result.blind_message, &result.secret)? {
            return Err(CryptoError::InvalidRequest);
        }
        Ok(Self {
            snapshot,
            key,
            result,
        })
    }

    pub fn finalize(&self, blind_signature: &[u8]) -> Result<String> {
        checked_uint(&self.key, blind_signature, false)
            .map_err(|_| CryptoError::InvalidSignature)?;
        let prepared = decode(&self.snapshot.prepared_message, 256)?;
        let signature = self
            .key
            .finalize(
                &BlindSignature(blind_signature.to_vec()),
                &self.result,
                &prepared[32..],
            )
            .map_err(|_| CryptoError::InvalidSignature)?;
        Ok(format!(
            "v1.{}.{}.{}",
            self.snapshot.key_id,
            self.snapshot.prepared_message,
            B64.encode(signature)
        ))
    }
}
fn pass_message(key: &StylePassKey, expires_at_millis: u64, now_millis: u64) -> Result<Vec<u8>> {
    if now_millis < key.not_before
        || now_millis >= key.not_after
        || expires_at_millis <= now_millis
        || expires_at_millis > key.not_after
        || expires_at_millis / 1000 == 0
    {
        return Err(CryptoError::InvalidExpiry);
    }
    let mut nonce = [0u8; 32];
    getrandom::fill(&mut nonce).map_err(|_| CryptoError::RandomUnavailable)?;
    let mut message = b"govorun-style-pass-v1\0".to_vec();
    message.extend_from_slice(key.id.as_bytes());
    message.push(0);
    message.extend_from_slice(&(expires_at_millis / 1000).to_be_bytes());
    message.extend_from_slice(&nonce);
    Ok(message)
}

fn minimal(bytes: &[u8]) -> &[u8] {
    &bytes[bytes
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(bytes.len() - 1)..]
}
fn decode(value: &str, max_bytes: usize) -> Result<Vec<u8>> {
    if value.len() > max_bytes.div_ceil(3) * 4 {
        return Err(CryptoError::InvalidRequest);
    }
    let bytes = B64.decode(value).map_err(|_| CryptoError::InvalidRequest)?;
    if bytes.is_empty() || bytes.len() > max_bytes || B64.encode(&bytes) != value {
        return Err(CryptoError::InvalidRequest);
    }
    Ok(bytes)
}
fn checked_uint(key: &PublicKey, bytes: &[u8], nonzero: bool) -> Result<BoxedUint> {
    let raw: &rsa::RsaPublicKey = key.as_ref();
    if bytes.len() != raw.size() {
        return Err(CryptoError::InvalidRequest);
    }
    let value = BoxedUint::from_be_slice(bytes, raw.n().bits_precision())
        .map_err(|_| CryptoError::InvalidRequest)?;
    if value >= *raw.n().as_ref() || (nonzero && bool::from(value.is_zero())) {
        return Err(CryptoError::InvalidRequest);
    }
    Ok(value)
}
fn recover_encoded(key: &PublicKey, blinded: &[u8], inverse: &[u8]) -> Result<Vec<u8>> {
    let raw: &rsa::RsaPublicKey = key.as_ref();
    let blinded = checked_uint(key, blinded, true)?;
    let inverse = checked_uint(key, inverse, true)?;
    // A persisted inverse must remain invertible; its relation to the encoded
    // message detects mixed or corrupted journals before any issuance replay.
    if !bool::from(inverse.invert_mod(raw.n()).is_some()) {
        return Err(CryptoError::InvalidRequest);
    }
    let factor = BoxedMontyForm::new(inverse, raw.n_params())
        .pow(raw.e())
        .retrieve();
    Ok(blinded
        .mul_mod(
            &factor,
            &raw.n()
                .as_ref()
                .clone()
                .to_nz()
                .into_option()
                .ok_or(CryptoError::InvalidRequest)?,
        )
        .to_be_bytes()
        .to_vec())
}
