//! Host-owned application information. Native modules supply only build facts.
use super::dto::dto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

dto!(NativeBuildFacts {
    version: String,
    platformBuild: String,
    buildID: String
});
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFacts {
    pub platform: String,
    pub build_facts: NativeBuildFacts,
}

dto!(ApplicationLicense {
    id: String,
    name: String,
    spdx: String,
    copyright: String
});
dto!(AboutSnapshot {
    version: String,
    buildID: String,
    native_version: String,
    platform_build: String,
    nativeBuildID: String,
    key_storage: String,
    licenses: Vec<ApplicationLicense>,
});
dto!(AboutOpenLink { id: String });

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct License {
    #[serde(flatten)]
    info: ApplicationLicense,
    url: String,
}
fn catalog(platform: &str) -> Result<Vec<License>, &'static str> {
    let mut catalogs: BTreeMap<String, Vec<License>> =
        serde_json::from_str(include_str!("licenses.json")).map_err(|_| "invalid_about")?;
    catalogs.remove(platform).ok_or("invalid_about")
}

pub fn snapshot(
    runtime: &Value,
    version: &str,
    build_id: &str,
) -> Result<AboutSnapshot, &'static str> {
    let platform = runtime["platform"].as_str().ok_or("invalid_about")?;
    let facts = &runtime["buildFacts"];
    let fact = |name: &str| -> Result<String, &'static str> {
        facts[name]
            .as_str()
            .filter(|value| value.len() <= 1024)
            .map(str::to_owned)
            .ok_or("invalid_about")
    };
    Ok(AboutSnapshot {
        version: version.into(),
        buildID: build_id.into(),
        native_version: fact("version")?,
        platform_build: fact("platformBuild")?,
        nativeBuildID: fact("buildID")?,
        key_storage: match platform {
            "macos" => "keychain",
            "windows" => "dpapi",
            _ => return Err("invalid_about"),
        }
        .into(),
        licenses: catalog(platform)?
            .into_iter()
            .map(|item| item.info)
            .collect(),
    })
}

pub fn link(platform: &str, id: &str) -> Result<String, &'static str> {
    // Validate the platform even for project links. Never accept renderer URLs.
    let catalog = catalog(platform)?;
    if id.len() > 64 {
        return Err("invalid_parameters");
    }
    let project = match id {
        "legal" => Some("https://govorun.club/legal"),
        "author" => Some("https://github.com/amidexe"),
        "source-v1" => Some("https://github.com/amidexe/govorun-osx"),
        "tauri" => Some("https://github.com/tauri-apps/tauri"),
        "svelte" => Some("https://github.com/sveltejs/svelte"),
        _ => None,
    };
    project
        .map(str::to_owned)
        .or_else(|| {
            catalog
                .into_iter()
                .find(|item| item.info.id == id)
                .map(|item| item.url)
        })
        .ok_or("invalid_parameters")
}

pub fn command(
    runtime: &Value,
    version: &str,
    build_id: &str,
    method: &str,
    params: Value,
    open: impl FnOnce(&str) -> Result<(), &'static str>,
) -> Result<Value, &'static str> {
    match method {
        "about.snapshot" if params.as_object().is_some_and(|params| params.is_empty()) => {
            serde_json::to_value(snapshot(runtime, version, build_id)?).map_err(|_| "invalid_about")
        }
        "about.openLink" => {
            let action: AboutOpenLink =
                serde_json::from_value(params).map_err(|_| "invalid_parameters")?;
            open(&link(
                runtime["platform"].as_str().ok_or("invalid_about")?,
                &action.id,
            )?)?;
            Ok(Value::Null)
        }
        _ => Err("invalid_parameters"),
    }
}

/// One host feature path, exercised with an in-process agent and fake OS opener.
pub async fn invoke(
    agent: &crate::agent::AgentClient,
    version: &str,
    build_id: &str,
    method: &str,
    params: Value,
    open: impl FnOnce(&str) -> Result<(), &'static str> + Send + 'static,
) -> Result<Value, &'static str> {
    let facts: RuntimeFacts = agent
        .call(
            crate::agent::Request {
                capability: crate::agent::Capability::named("runtime.snapshot")?,
                params: serde_json::json!({}),
            },
            std::time::Duration::from_secs(20),
            &crate::agent::Cancellation::default(),
        )
        .await?;
    let runtime = serde_json::to_value(facts).map_err(|_| "invalid_about")?;
    let (version, build_id, method) = (version.to_owned(), build_id.to_owned(), method.to_owned());
    tokio::task::spawn_blocking(move || {
        command(&runtime, &version, &build_id, &method, params, open)
    })
    .await
    .map_err(|_| "link_unavailable")?
}
