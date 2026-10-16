//! Foundation access to the exact existing Defaults domain; no profile migration.
use super::{
    defaults_documents::{Cell, Changes, Domain, Snapshot},
    file_documents::Error,
};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
    AnyThread,
};
use objc2_foundation::{
    NSBundle, NSData, NSDictionary, NSJSONReadingOptions, NSJSONSerialization,
    NSJSONWritingOptions, NSMutableDictionary, NSNumber, NSString, NSUserDefaults,
};

static DOMAIN_WRITER: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub struct Defaults {
    domain: String,
}
impl Defaults {
    pub fn new(domain: String) -> Result<Self, Error> {
        if domain.is_empty() || domain.len() > 256 {
            return Err(Error::Invalid);
        }
        Ok(Self { domain })
    }
    fn defaults(&self) -> Result<Retained<NSUserDefaults>, Error> {
        let domain = NSString::from_str(&self.domain);
        if NSBundle::mainBundle()
            .bundleIdentifier()
            .is_some_and(|id| id.to_string() == self.domain)
        {
            return Ok(NSUserDefaults::standardUserDefaults());
        }
        NSUserDefaults::initWithSuiteName(NSUserDefaults::alloc(), Some(&domain))
            .ok_or(Error::Unavailable)
    }
    fn persistent(&self, defaults: &NSUserDefaults) -> Retained<NSDictionary<NSString, AnyObject>> {
        defaults
            .persistentDomainForName(&NSString::from_str(&self.domain))
            .unwrap_or_else(NSDictionary::dictionary)
    }
}
fn scalar(value: &AnyObject) -> Option<serde_json::Value> {
    let encoded = unsafe {
        NSJSONSerialization::dataWithJSONObject_options_error(
            value,
            NSJSONWritingOptions::FragmentsAllowed,
        )
    }
    .ok()?;
    let value: serde_json::Value =
        serde_json::from_slice(unsafe { encoded.as_bytes_unchecked() }).ok()?;
    if value.is_string() || value.is_number() || value.is_boolean() {
        Some(value)
    } else {
        None
    }
}
fn object(cell: Cell) -> Result<Retained<AnyObject>, Error> {
    match cell {
        Cell::Data(bytes) => Ok(NSData::with_bytes(&bytes).into_super().into_super()),
        Cell::Scalar(value) => {
            let bytes = serde_json::to_vec(&value).map_err(|_| Error::Invalid)?;
            NSJSONSerialization::JSONObjectWithData_options_error(
                &NSData::with_bytes(&bytes),
                NSJSONReadingOptions::FragmentsAllowed,
            )
            .map_err(|_| Error::Invalid)
        }
        Cell::Other => Err(Error::Invalid),
    }
}
impl Domain for Defaults {
    fn read(&self) -> Result<Snapshot, Error> {
        let defaults = self.defaults()?;
        let persistent = self.persistent(&defaults);
        let mut values = Snapshot::new();
        let keys = persistent.allKeys();
        for index in 0..keys.count() {
            let key = keys.objectAtIndex(index);
            let value = persistent.objectForKey(&key).ok_or(Error::Unavailable)?;
            let cell = if let Some(data) = value.downcast_ref::<NSData>() {
                // Only document data is bounded/interpreted; unrelated objects
                // remain in the original Foundation dictionary when writing.
                if data.len() > 8 * 1_048_576 {
                    Cell::Data(vec![0; 8 * 1_048_576 + 1])
                } else {
                    Cell::Data(unsafe { data.as_bytes_unchecked() }.to_vec())
                }
            } else if let Some(text) = value.downcast_ref::<NSString>() {
                Cell::Scalar(serde_json::json!(text.to_string()))
            } else if let Some(value) = value
                .downcast_ref::<NSNumber>()
                .and_then(|_| scalar(&value))
            {
                Cell::Scalar(value)
            } else {
                Cell::Other
            };
            values.insert(key.to_string(), cell);
        }
        Ok(values)
    }
    fn write(&self, changes: Changes) -> Result<(), Error> {
        // Validate/allocate every replacement before the single domain write.
        let converted: Result<Vec<_>, Error> = changes
            .into_iter()
            .map(|(key, cell)| Ok((NSString::from_str(&key), cell.map(object).transpose()?)))
            .collect();
        let converted = converted?;
        // Every document and protected marker merges into the latest domain
        // under this gate, so concurrent disjoint writes cannot drop each other.
        let _guard = DOMAIN_WRITER.lock().map_err(|_| Error::Unavailable)?;
        let defaults = self.defaults()?;
        let persistent = self.persistent(&defaults);
        let next = NSMutableDictionary::dictionaryWithDictionary(&persistent);
        for (key, value) in converted {
            if let Some(value) = value {
                unsafe {
                    next.setObject_forKey(&value, ProtocolObject::from_ref(&*key));
                }
            } else {
                next.removeObjectForKey(&key);
            }
        }
        unsafe {
            defaults.setPersistentDomain_forName(&next, &NSString::from_str(&self.domain));
        }
        if defaults.synchronize() {
            Ok(())
        } else {
            Err(Error::Unavailable)
        }
    }
    fn integer(&self, key: &str) -> i64 {
        self.defaults()
            .map(|d| d.integerForKey(&NSString::from_str(key)) as i64)
            .unwrap_or(0)
    }
    fn boolean(&self, key: &str) -> bool {
        self.defaults()
            .is_ok_and(|d| d.boolForKey(&NSString::from_str(key)))
    }
}
