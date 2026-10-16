//! Endpoint/proxy admission and physical recipient-token compatibility.
use super::configuration::Configuration;
use crate::{Error, Result};
use url::Url;

fn address(value: &str) -> Result<Url> {
    let value = value.trim();
    let url = Url::parse(value).map_err(|_| Error::InvalidRequest)?;
    let authority = value
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or(""));
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || authority.is_some_and(|text| text.contains('@'))
        || url.fragment().is_some()
    {
        return Err(Error::InvalidRequest);
    }
    Ok(url)
}
pub fn chat(value: &str) -> Result<Url> {
    let mut url = address(value)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Error::InvalidRequest);
    }
    url.set_path(&chat_path(url.path(), false));
    Ok(url)
}
pub(super) fn chat_path(path:&str, android_codec:bool) -> String {
    let path=path.trim_end_matches('/');
    if (if android_codec {path.to_owned()} else {path.to_ascii_lowercase()}).ends_with("/chat/completions") {path.into()}
    else if android_codec && path.ends_with("/chat") {format!("{path}/completions")}
    else {format!("{path}/chat/completions")}
}

pub fn proxy(value: &str) -> Result<Url> {
    let mut url = address(value)?;
    if !matches!(url.scheme(), "http" | "socks5")
        || url.query().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err(Error::InvalidRequest);
    }
    // URL normalizes explicit :80 away; retain it rather than choosing :8080.
    let authority = value
        .trim()
        .split_once("://")
        .ok_or(Error::InvalidRequest)?
        .1
        .trim_end_matches('/');
    let explicit = if authority.starts_with('[') {
        authority.contains("]:")
    } else {
        authority.contains(':')
    };
    let port = if explicit {
        url.port_or_known_default().ok_or(Error::InvalidRequest)?
    } else if url.scheme() == "socks5" {
        1080
    } else {
        8080
    };
    if port == 0 {
        return Err(Error::InvalidRequest);
    }
    url.set_port(Some(port))
        .map_err(|_| Error::InvalidRequest)?;
    url.set_path("");
    Ok(url)
}

#[derive(Clone, Copy)]
pub enum StoredFormat {
    Macos,
    Windows,
}
/// Preserve the existing token bytes; this is a codec, not a second consent rule.
pub fn recipient(config: &Configuration, format: StoredFormat) -> Result<String> {
    let endpoint = chat(&config.endpoint)?;
    let proxy = if config.proxy_enabled {
        Some(proxy(&config.proxy_url)?)
    } else {
        None
    };
    Ok(match format {
        StoredFormat::Macos => format!(
            "v1\n{}\n{}\n{}",
            config.provider.name(),
            endpoint,
            if proxy.is_some() {
                config.proxy_url.trim()
            } else {
                ""
            }
        ),
        StoredFormat::Windows => format!(
            "v1\n{}{}{}",
            if config.provider.name() == "ollama" {
                String::new()
            } else {
                format!("{}\n", config.provider.name())
            },
            endpoint,
            proxy.map(|value| format!("\n{value}")).unwrap_or_default()
        ),
    })
}
