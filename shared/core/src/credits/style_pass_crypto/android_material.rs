//! Codec for Android's current preparedMessage/salt/inverse journal. No store
//! rewrite or native cryptography: replay uses the pinned RFC implementation.
use super::*;
use blind_rsa_signatures::reexports::rsa::rand_core::{TryCryptoRng, TryRng};
use std::collections::VecDeque;

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Material {
    pub prepared_message: String,
    pub salt: String,
    pub inverse: String,
}

struct MaterialRng {
    tape: Option<VecDeque<Vec<u8>>>,
    salt: Option<Vec<u8>>,
    calls: usize,
    valid: bool,
}
impl TryCryptoRng for MaterialRng {}
impl TryRng for MaterialRng {
    type Error = std::convert::Infallible;
    fn try_next_u32(&mut self) -> std::result::Result<u32, Self::Error> {
        self.valid = false; DefaultRng.try_next_u32()
    }
    fn try_next_u64(&mut self) -> std::result::Result<u64, Self::Error> {
        self.valid = false; DefaultRng.try_next_u64()
    }
    fn try_fill_bytes(&mut self, out: &mut [u8]) -> std::result::Result<(), Self::Error> {
        if let Some(tape) = &mut self.tape {
            match tape.pop_front() {
                Some(bytes) if bytes.len() == out.len() => out.copy_from_slice(&bytes),
                _ => { self.valid = false; DefaultRng.try_fill_bytes(out)?; }
            }
        } else {
            DefaultRng.try_fill_bytes(out)?;
            if self.calls == 1 { self.salt = Some(out.to_vec()); }
        }
        self.calls += 1;
        Ok(())
    }
}

pub(crate) fn prepare(key: &StylePassKey, expires: u64, now: u64) -> Result<(BlindRequest, Material)> {
    let mut rng = MaterialRng { tape: None, salt: None, calls: 0, valid: true };
    let request = key.blind(&pass_message(key, expires, now)?, &mut rng)?;
    let salt = rng.salt.filter(|v| v.len() == 48).ok_or(CryptoError::InvalidRequest)?;
    if !rng.valid { return Err(CryptoError::InvalidRequest); }
    let snapshot = request.snapshot();
    let material = Material { prepared_message: snapshot.prepared_message, salt: B64.encode(salt), inverse: snapshot.inverse };
    Ok((request, material))
}

pub(crate) fn restore(key: &StylePassKey, material: Material) -> Result<BlindRequest> {
    let raw: &rsa::RsaPublicKey = key.key.as_ref();
    let prepared = decode(&material.prepared_message, 256)?;
    let salt = decode(&material.salt, 48)?;
    if prepared.len() < 64 || salt.len() != 48 { return Err(CryptoError::InvalidRequest); }
    let inverse = checked_uint(&key.key, &decode(&material.inverse, 1024)?, true)?;
    let factor = inverse.invert_mod(raw.n()).into_option().ok_or(CryptoError::InvalidRequest)?;
    let mut rng = MaterialRng { tape: Some(VecDeque::from([
        prepared[..32].to_vec(), salt, factor.to_le_bytes().to_vec()
    ])), salt: None, calls: 0, valid: true };
    let request = key.blind(&prepared[32..], &mut rng)?;
    if !rng.valid || rng.calls != 3 || rng.tape.is_some_and(|v| !v.is_empty()) ||
        request.snapshot().inverse != material.inverse { return Err(CryptoError::InvalidRequest); }
    Ok(request)
}
