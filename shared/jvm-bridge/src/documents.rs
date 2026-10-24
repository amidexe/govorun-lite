//! Android documents in the shared base. Every key of an old SharedPreferences file
//! belongs to one document of `android-storage.json`; old files are only read, through
//! the Kotlin callback, when the base moves them. JNI only converts values.
use govorun_document_store::{base_unavailable, Cell, Db, Error, Legacy, Rows};
use jni::{
    objects::{GlobalRef, JObject, JString, JValue},
    sys::{jboolean, jlong, jstring},
    JNIEnv, JavaVM,
};
use serde_json::{json, Map, Value};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    ptr,
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

struct Rule {
    document: &'static str,
    exact: Vec<&'static str>,
    prefix: Vec<&'static str>,
}
struct File {
    name: &'static str,
    rules: Vec<Rule>,
    default: &'static str,
}
struct Table {
    files: Vec<File>,
    documents: Vec<&'static str>,
    consent: Vec<&'static str>,
}

fn leaked(value: &Value) -> &'static str {
    Box::leak(value.as_str().expect("android storage table").to_owned().into_boxed_str())
}
fn strings(value: &Value) -> Vec<&'static str> {
    value.as_array().map(|items| items.iter().map(leaked).collect()).unwrap_or_default()
}
fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        let raw: Value = serde_json::from_str(include_str!("../../core/contracts/android-storage.json"))
            .expect("bundled android storage table");
        let files: Vec<File> = raw["files"].as_object().expect("android storage files").iter().map(|(name, spec)| File {
            name: Box::leak(name.clone().into_boxed_str()),
            rules: spec["rules"].as_array().into_iter().flatten().map(|rule| Rule {
                document: leaked(&rule["document"]),
                exact: strings(&rule["exact"]),
                prefix: strings(&rule["prefix"]),
            }).collect(),
            default: leaked(&spec["default"]),
        }).collect();
        let mut documents: Vec<&'static str> = files.iter()
            .flat_map(|file| file.rules.iter().map(|rule| rule.document).chain([file.default])).collect();
        documents.sort_unstable();
        documents.dedup();
        Table { files, documents, consent: strings(&raw["consent"]) }
    })
}
impl Table {
    fn document(&self, file: &str, key: &str) -> Option<&'static str> {
        let file = self.files.iter().find(|candidate| candidate.name == file)?;
        Some(file.rules.iter()
            .find(|rule| rule.exact.contains(&key) || rule.prefix.iter().any(|prefix| key.starts_with(prefix)))
            .map_or(file.default, |rule| rule.document))
    }
}

struct AndroidLegacy {
    vm: JavaVM,
    callback: GlobalRef,
    /// Old files are read once per process: nothing in this version writes them, and
    /// the shared file feeds nine documents.
    read: Mutex<HashMap<String, Map<String, Value>>>,
}
impl AndroidLegacy {
    fn read(&self, file: &str) -> Result<Map<String, Value>, Error> {
        if let Some(values) = self.read.lock().map_err(|_| Error::Unavailable)?.get(file) {
            return Ok(values.clone());
        }
        let values = self.read_now(file)?;
        self.read.lock().map_err(|_| Error::Unavailable)?.insert(file.to_owned(), values.clone());
        Ok(values)
    }
    /// The values an old file holds now, as typed JSON; an absent file holds none.
    fn read_now(&self, file: &str) -> Result<Map<String, Value>, Error> {
        let mut env = self.vm.attach_current_thread().map_err(|_| Error::Unavailable)?;
        let name = env.new_string(file).map_err(|_| Error::Unavailable)?;
        let result = env.call_method(self.callback.as_obj(), "legacy", "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&JObject::from(name))]);
        if env.exception_check().unwrap_or(true) {
            let _ = env.exception_clear();
            return Err(Error::Unavailable);
        }
        let text: String = result.and_then(|value| value.l())
            .and_then(|value| env.get_string(&JString::from(value)).map(Into::into))
            .map_err(|_| Error::Unavailable)?;
        match serde_json::from_str(&text) {
            Ok(Value::Object(values)) => Ok(values),
            _ => Err(Error::Unavailable),
        }
    }
    fn notice(&self, code: &str) {
        let Ok(mut env) = self.vm.attach_current_thread() else { return };
        if let Ok(code) = env.new_string(code) {
            let _ = env.call_method(self.callback.as_obj(), "notice", "(Ljava/lang/String;)V",
                &[JValue::Object(&JObject::from(code))]);
        }
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}
impl Legacy for AndroidLegacy {
    fn sources(&self) -> Vec<&'static str> {
        table().documents.clone()
    }
    fn rows(&self, source: &str) -> Result<Rows, Error> {
        let mut rows = Rows::new();
        for file in &table().files {
            if file.default != source && !file.rules.iter().any(|rule| rule.document == source) {
                continue;
            }
            for (key, value) in self.read(file.name)? {
                if table().document(file.name, &key) == Some(source) {
                    rows.insert(format!("{}/{key}", file.name), Cell::Scalar(value));
                }
            }
        }
        Ok(rows)
    }
    fn holds_consent(&self, row: &str) -> bool {
        table().consent.iter().any(|prefix| row.starts_with(prefix))
    }
    fn fresh(&self) -> Vec<(&'static str, String, Cell)> {
        // The lost sync choice is asked once again for any account, as desktop does.
        vec![("sync.enrollment", "govorun_account_sync_enrollment_v1/requested_any".into(), Cell::Scalar(json!({"b":true})))]
    }
    fn empty_after_move_is_damage(&self) -> bool {
        true
    }
}

fn load(db: &Db<AndroidLegacy>, file: &str) -> Value {
    let Some(file) = table().files.iter().find(|candidate| candidate.name == file) else {
        return json!({"error":"document_unknown"});
    };
    let mut sources: Vec<&str> = file.rules.iter().map(|rule| rule.document).chain([file.default]).collect();
    sources.sort_unstable();
    sources.dedup();
    let prefix = format!("{}/", file.name);
    match db.rows(&sources) {
        Ok(rows) => {
            let values: Map<String, Value> = rows.into_iter().filter_map(|(name, cell)| match cell {
                Cell::Scalar(value) => name.strip_prefix(&prefix).map(|key| (key.to_owned(), value)),
                _ => None,
            }).collect();
            json!({"available":true, "values":values})
        }
        Err(_) => json!({"available":false, "values":{}}),
    }
}
fn write(db: &Db<AndroidLegacy>, changes: &Value) -> Value {
    let mut batch = Vec::new();
    for change in changes.as_array().into_iter().flatten() {
        let (Some(file), Some(key)) = (change["file"].as_str(), change["key"].as_str()) else {
            return json!({"error":"document_invalid"});
        };
        let Some(document) = table().document(file, key) else { return json!({"error":"document_unknown"}) };
        let value = change.get("value").filter(|value| !value.is_null()).cloned().map(Cell::Scalar);
        batch.push((document, format!("{file}/{key}"), value));
    }
    match db.write(&batch) {
        Ok(()) => json!({"written":true}),
        Err(_) => json!({"error":"document_unavailable"}),
    }
}

fn text(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Option<String> {
    env.get_string(value).ok().map(Into::into)
}
fn reply(env: &mut JNIEnv<'_>, value: Option<Value>) -> jstring {
    let value = value.unwrap_or_else(|| json!({"error":"document_invalid"}));
    env.new_string(value.to_string()).map(|value| value.into_raw()).unwrap_or(ptr::null_mut())
}
fn base(handle: jlong) -> Option<&'static Db<AndroidLegacy>> {
    // Handles are never freed: one base per application process.
    (handle != 0).then(|| unsafe { &*(handle as *const Db<AndroidLegacy>) })
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedDocumentsNative_open(
    mut env: JNIEnv, _receiver: JObject, path: JString, callback: JObject,
) -> jlong {
    catch_unwind(AssertUnwindSafe(|| -> Option<jlong> {
        let path = PathBuf::from(text(&mut env, &path)?);
        let legacy = AndroidLegacy { vm: env.get_java_vm().ok()?, callback: env.new_global_ref(callback).ok()?,
            read: Mutex::default() };
        let notice = AndroidLegacy { vm: env.get_java_vm().ok()?, callback: legacy.callback.clone(), read: Mutex::default() };
        let db = Db::new(path, legacy, Some(Box::new(move |code| notice.notice(code))));
        Some(Box::into_raw(Box::new(db)) as jlong)
    })).ok().flatten().unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedDocumentsNative_load(
    mut env: JNIEnv, _receiver: JObject, handle: jlong, file: JString,
) -> jstring {
    let value = catch_unwind(AssertUnwindSafe(|| Some(load(base(handle)?, &text(&mut env, &file)?)))).ok().flatten();
    reply(&mut env, value)
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedDocumentsNative_write(
    mut env: JNIEnv, _receiver: JObject, handle: jlong, changes: JString,
) -> jstring {
    let value = catch_unwind(AssertUnwindSafe(|| {
        let changes: Value = serde_json::from_str(&text(&mut env, &changes)?).ok()?;
        Some(write(base(handle)?, &changes))
    })).ok().flatten();
    reply(&mut env, value)
}

#[no_mangle]
pub extern "system" fn Java_com_govorun_lite_shared_SharedDocumentsNative_unavailable(
    _env: JNIEnv, _receiver: JObject,
) -> jboolean {
    u8::from(base_unavailable())
}
