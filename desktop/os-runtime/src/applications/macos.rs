use super::Choice;
use objc2::{msg_send, rc::autoreleasepool};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSBundle, NSString};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};
fn bundle(path: &Path, names: &mut BTreeMap<String, String>) {
    let Some(path_string) = path.to_str() else {
        return;
    };
    let Some(bundle) = NSBundle::bundleWithPath(&NSString::from_str(path_string)) else {
        return;
    };
    let Some(id) = bundle.bundleIdentifier() else {
        return;
    };
    let id = id.to_string();
    if id.is_empty() || id.encode_utf16().count() > 512 || id.chars().any(char::is_control) {
        return;
    }
    let mut name = None;
    for key in ["CFBundleDisplayName", "CFBundleName"] {
        if let Some(value) = bundle.objectForInfoDictionaryKey(&NSString::from_str(key)) {
            if let Some(value) = value.downcast_ref::<NSString>() {
                name = Some(value.to_string());
                break;
            }
        }
    }
    let name = name.unwrap_or_else(|| {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    if !name.is_empty() && name.encode_utf16().count() <= 512 && !name.chars().any(char::is_control)
    {
        names.insert(id, name);
    }
}
pub fn discover(saved: &[Choice]) -> Vec<Choice> {
    autoreleasepool(|_| {
        let mut names = BTreeMap::new();
        for choice in saved {
            if choice.id.trim().is_empty()
                || choice.name.trim().is_empty()
                || choice.id.encode_utf16().count() > 512
                || choice.name.encode_utf16().count() > 512
                || choice.id.chars().any(char::is_control)
                || choice.name.chars().any(char::is_control)
            {
                continue;
            }
            names
                .entry(choice.id.clone())
                .or_insert(choice.name.clone());
        }
        for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {
            let regular: isize = unsafe { msg_send![&*app, activationPolicy] };
            if regular != 0 {
                continue;
            }
            if let Some(url) = app.bundleURL() {
                if let Some(path) = url.path() {
                    bundle(Path::new(&path.to_string()), &mut names);
                }
            }
        }
        let mut roots = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join("Applications"));
        }
        let mut visited = HashSet::new();
        while let Some(root) = roots.pop() {
            let Ok(canonical) = root.canonicalize() else {
                continue;
            };
            if !visited.insert(canonical) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                match path.extension().and_then(|value| value.to_str()) {
                    Some("app") => bundle(&path, &mut names),
                    _ if entry.file_type().is_ok_and(|kind| kind.is_dir())
                        && !NSWorkspace::sharedWorkspace()
                            .isFilePackageAtPath(&NSString::from_str(&path.to_string_lossy())) =>
                    {
                        roots.push(path)
                    }
                    _ => {}
                }
            }
        }
        let mut result: Vec<_> = names
            .into_iter()
            .map(|(id, name)| Choice { id, name })
            .collect();
        result.sort_by(|left, right| {
            NSString::from_str(&left.name)
                .localizedCaseInsensitiveCompare(&NSString::from_str(&right.name))
                .cmp(&objc2_foundation::NSComparisonResult::Same)
        });
        result.truncate(4096);
        result
    })
}
