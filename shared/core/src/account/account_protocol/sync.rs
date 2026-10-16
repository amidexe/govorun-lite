//! Synchronization selects named operations, never caller-selected URLs or headers.
use crate::{content_sync, preference_model::InputChanges, AccountSession, Error};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "operation",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Command {
    #[serde(rename = "syncContentRead")]
    ContentRead {
        session_generation: u64,
        etag: Option<String>,
    },
    #[serde(rename = "syncContentWrite")]
    ContentWrite {
        session_generation: u64,
        document: content_sync::Document,
        etag: String,
    },
    #[serde(rename = "syncPreferencesRead")]
    PreferencesRead { session_generation: u64 },
    #[serde(rename = "syncPreferencesPatch")]
    PreferencesPatch {
        session_generation: u64,
        patch: Patch,
    },
    #[serde(rename = "syncStatisticsWrite")]
    StatisticsWrite {
        session_generation: u64,
        request: Value,
    },
    #[serde(rename = "syncStatisticsDelete")]
    StatisticsDelete {
        session_generation: u64,
        confirmed_reset: bool,
    },
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Patch {
    schema_version: u32,
    base_revision: i64,
    changes: Value,
}

pub(super) fn request(params: Value) -> Result<Value, Error> {
    let command: Command = serde_json::from_value(params).map_err(|_| Error::InvalidRequest)?;
    let (epoch, method, path, body, etag) = match command {
        Command::ContentRead {
            session_generation,
            etag,
        } => (
            session_generation,
            "GET",
            "/v1/sync-document",
            Value::Null,
            etag,
        ),
        Command::ContentWrite {
            session_generation,
            document,
            etag,
        } => {
            content_sync::validate(&document)?;
            (
                session_generation,
                "PUT",
                "/v1/sync-document",
                serde_json::to_value(document).map_err(|_| Error::InvalidRequest)?,
                Some(etag),
            )
        }
        Command::PreferencesRead { session_generation } => (
            session_generation,
            "GET",
            "/v1/preferences",
            Value::Null,
            None,
        ),
        Command::PreferencesPatch {
            session_generation,
            patch,
        } => {
            let changes: InputChanges =
                serde_json::from_value(patch.changes.clone()).map_err(|_| Error::InvalidRequest)?;
            // Preserve the existing workflow's exact portable wire; legacy aliases
            // may be decoded locally, but cannot become additional upload fields.
            if patch.schema_version != 1
                || patch.base_revision < 0
                || changes.normalize()?.wire()? != patch.changes
            {
                return Err(Error::InvalidRequest);
            }
            (
                session_generation,
                "PATCH",
                "/v1/preferences",
                json!({"schemaVersion":1,"baseRevision":patch.base_revision,"changes":patch.changes}),
                None,
            )
        }
        Command::StatisticsWrite {
            session_generation,
            request,
        } => {
            crate::statistics_sync::validate_upload(request.clone())?;
            (session_generation, "PUT", "/v3/stats/days", request, None)
        }
        Command::StatisticsDelete {
            session_generation,
            confirmed_reset,
        } => {
            if !confirmed_reset {
                return Err(Error::InvalidRequest);
            }
            (session_generation, "DELETE", "/v1/stats", Value::Null, None)
        }
    };
    Ok(json!({"sessionGeneration":epoch,"method":method,"path":path,"body":body,"etag":etag}))
}
pub(super) async fn dispatch(core: &AccountSession, params: Value) -> Result<Value, Error> {
    super::http(core, request(params)?).await
}
