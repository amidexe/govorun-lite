//! CurrentUser DPAPI with the exact pre-refactor entropy. Buffers never leave this port.
use super::secure_files::{Cryptography, Error};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

pub struct Dpapi {
    entropy: Vec<u8>,
}
impl Dpapi {
    pub fn new(entropy: &str) -> Self {
        Self {
            entropy: entropy.as_bytes().to_vec(),
        }
    }
}
impl Cryptography for Dpapi {
    fn protect(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        self.apply(raw, true)
    }
    fn unprotect(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        self.apply(raw, false)
    }
}
impl Dpapi {
    fn apply(&self, raw: &[u8], protect: bool) -> Result<Vec<u8>, Error> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: raw.len().try_into().map_err(|_| Error::Invalid)?,
            pbData: raw.as_ptr().cast_mut(),
        };
        let entropy = CRYPT_INTEGER_BLOB {
            cbData: self.entropy.len().try_into().map_err(|_| Error::Invalid)?,
            pbData: self.entropy.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let success = unsafe {
            if protect {
                CryptProtectData(
                    &input,
                    std::ptr::null(),
                    &entropy,
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    std::ptr::null_mut(),
                    &entropy,
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
        };
        if success == 0 {
            return Err(Error::Unavailable);
        }
        let bytes = if output.cbData == 0 {
            Vec::new()
        } else {
            if output.pbData.is_null() {
                return Err(Error::Unavailable);
            }
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() }
        };
        if !output.pbData.is_null() {
            unsafe {
                LocalFree(output.pbData.cast());
            }
        }
        Ok(bytes)
    }
}
