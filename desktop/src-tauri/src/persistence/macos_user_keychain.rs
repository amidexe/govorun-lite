//! Read old user-created server keys. Account/session migration is deliberately absent.
use core_foundation::{
    base::{CFType, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::CFDictionary,
    string::{CFString, CFStringRef},
};
use security_framework_sys::{item::*, keychain_item::*};
use std::{cell::Cell, sync::Mutex};


#[link(name = "Security", kind = "framework")]
extern "C" {
    fn SecKeychainGetUserInteractionAllowed(allowed: *mut u8) -> i32;
    fn SecKeychainSetUserInteractionAllowed(allowed: u8) -> i32;
    static kSecUseAuthenticationUIFail: CFStringRef;
    static kSecUseDataProtectionKeychain: CFStringRef;
    static kSecMatchLimitOne: CFStringRef;
    static kSecAttrAccessible: CFStringRef;
    static kSecAttrAccessibleAfterFirstUnlock: CFStringRef;
}
const MISSING: i32 = -25300;

const DECODE: i32 = -26275;
const ENTITLEMENT: i32 = -34018;
static INTERACTION: Mutex<()> = Mutex::new(());
thread_local! { static INSIDE_INTERACTION: Cell<bool> = const { Cell::new(false) }; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Denied,
    Unavailable,
}
fn error(status: i32) -> Error {
    match status {
        DECODE => Error::Invalid,
        -25293 | -128 | -25308 => Error::Denied,
        _ => Error::Unavailable,
    }
}
fn blocking(status: i32) -> bool {
    matches!(status, -25293 | -128 | -25308)
}
fn constant(key: CFStringRef) -> CFString {
    unsafe { CFString::wrap_under_get_rule(key) }
}
struct Restore(u8);
impl Drop for Restore {
    fn drop(&mut self) {
        unsafe {
            SecKeychainSetUserInteractionAllowed(self.0);
        }
        INSIDE_INTERACTION.set(false);
    }
}
pub(crate) fn within<T>(allow: bool, work: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    // Reentrant calls cannot wait for their own guard or change its SDK mode.
    if INSIDE_INTERACTION.get() { return Err(Error::Unavailable); }
    // All callers run on a blocking worker. Contention is not a failed read:
    // serialize SDK interaction modes rather than briefly hiding the account.
    let _guard = INTERACTION.lock().map_err(|_| Error::Unavailable)?;
    let mut previous = 0;
    if unsafe { SecKeychainGetUserInteractionAllowed(&mut previous) } != 0 {
        return Err(Error::Unavailable);
    }
    let _restore = Restore(previous);
    INSIDE_INTERACTION.set(true);
    if unsafe { SecKeychainSetUserInteractionAllowed(u8::from(allow)) } != 0 {
        return Err(Error::Unavailable);
    }
    work()
}
pub(crate) fn query(
    service: &str,
    account: &str,
    allow: bool,
    sync: Option<CFType>,
) -> Vec<(CFString, CFType)> {
    let mut fields = vec![
        (
            constant(unsafe { kSecClass }),
            constant(unsafe { kSecClassGenericPassword }).as_CFType(),
        ),
        (
            constant(unsafe { kSecAttrService }),
            CFString::new(service).as_CFType(),
        ),
        (
            constant(unsafe { kSecAttrAccount }),
            CFString::new(account).as_CFType(),
        ),
    ];
    if !allow {
        fields.push((
            constant(unsafe { kSecUseAuthenticationUI }),
            constant(unsafe { kSecUseAuthenticationUIFail }).as_CFType(),
        ));
    }
    if let Some(sync) = sync {
        fields.push((constant(unsafe { kSecAttrSynchronizable }), sync));
    }
    fields
}
fn dictionary(fields: &[(CFString, CFType)]) -> CFDictionary<CFString, CFType> {
    CFDictionary::from_CFType_pairs(fields)
}
fn read_one(mut fields: Vec<(CFString, CFType)>) -> Result<Option<String>, i32> {
    fields.push((
        constant(unsafe { kSecReturnData }),
        CFBoolean::true_value().as_CFType(),
    ));
    fields.push((
        constant(unsafe { kSecMatchLimit }),
        constant(unsafe { kSecMatchLimitOne }).as_CFType(),
    ));
    let query = dictionary(&fields);
    let mut output = std::ptr::null();
    let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut output) };
    if status == MISSING {
        return Ok(None);
    }
    if status != 0 {
        return Err(status);
    }
    if output.is_null() {
        return Err(DECODE);
    }
    let output = unsafe { CFType::wrap_under_create_rule(output) };
    if output.type_of() != CFData::type_id() {
        return Err(DECODE);
    }
    let data = unsafe { CFData::wrap_under_get_rule(output.as_CFTypeRef().cast()) };
    String::from_utf8(data.bytes().to_vec())
        .map(Some)
        .map_err(|_| DECODE)
}
pub struct Keychain;
impl Keychain {
    pub fn read(
        service: &str,
        account: &str,
        allow: bool,
        llm: bool,
    ) -> Result<Option<String>, Error> {
        within(allow, || {
            let mut last = MISSING;
            for sync in if llm {
                vec![None, Some(CFBoolean::true_value().as_CFType())]
            } else {
                vec![None]
            } {
                let synced = sync.is_some();
                match read_one(query(service, account, allow, sync)) {
                    Ok(Some(raw)) => return Ok(Some(raw)),
                    Ok(None) => {}
                    Err(ENTITLEMENT) if synced => {}
                    Err(status) if blocking(status) => return Err(error(status)),
                    Err(status) => last = status,
                }
            }
            if last == MISSING {
                Ok(None)
            } else {
                Err(error(last))
            }
        })
    }
}
