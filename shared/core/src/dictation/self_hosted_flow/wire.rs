//! Three existing desktop protocols; only user text/prompt/key reach the endpoint.
use super::{
    configuration::{Configuration, Provider},
    endpoint,
};
use crate::{Error, Result};
use serde_json::{json, Value};
use url::Url;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Chat,
    Responses,
    Gemini,
}
pub struct Request {
    pub endpoint: Url,
    pub protocol: Protocol,
    pub key_header: Option<(&'static str, String)>,
    pub body: Value,
}
pub fn protocol(config: &Configuration) -> Result<Protocol> {
    Ok(if config.provider == Provider::Gemini {
        Protocol::Gemini
    } else if config.provider == Provider::Openai
        && endpoint::chat(&config.endpoint)?.host_str() == Some("api.openai.com")
    {
        Protocol::Responses
    } else {
        Protocol::Chat
    })
}
pub fn request(config: &Configuration, key: Option<&str>, text: &str) -> Result<Request> {
    let config = config.clone().checked()?;
    let key = key.map(str::trim).filter(|key| !key.is_empty());
    if key.is_some_and(|key| key.len() > 16384 || key.contains(['\r', '\n']))
        || config.requires_key()? && key.is_none()
    {
        return Err(Error::InvalidRequest);
    }
    let protocol = protocol(&config)?;
    let mut endpoint = endpoint::chat(&config.endpoint)?;
    if protocol != Protocol::Chat {
        let path = endpoint.path()[..endpoint.path().len() - "/chat/completions".len()]
            .trim_end_matches('/')
            .to_owned();
        if protocol == Protocol::Responses {
            endpoint.set_path(&format!("{path}/responses"));
        } else {
            endpoint.set_path(&format!("{path}/models/"));
            endpoint
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest)?
                .pop_if_empty()
                .push(&format!("{}:generateContent", config.model));
        }
    }
    let key_header = key.map(|key| {
        if protocol == Protocol::Gemini {
            ("x-goog-api-key", key.to_owned())
        } else {
            ("Authorization", format!("Bearer {key}"))
        }
    });
    let body = match protocol {
        Protocol::Chat => {
            json!({"model":config.model,"stream":false,"messages":[{"role":"system","content":config.prompt},{"role":"user","content":text}]})
        }
        Protocol::Responses => {
            json!({"model":config.model,"instructions":config.prompt,"input":text,"stream":false,"store":false})
        }
        Protocol::Gemini => {
            json!({"systemInstruction":{"parts":[{"text":config.prompt}]},"contents":[{"role":"user","parts":[{"text":text}]}],
            "generationConfig":{"thinkingConfig":{"thinkingBudget":0}}})
        }
    };
    Ok(Request {
        endpoint,
        protocol,
        key_header,
        body,
    })
}
pub fn response(protocol: Protocol, body: &Value) -> Result<String> {
    let output = match protocol {
        Protocol::Chat => body["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_owned),
        Protocol::Responses => body["output_text"]
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .map(str::to_owned)
            .or_else(|| {
                Some(
                    body["output"]
                        .as_array()?
                        .iter()
                        .filter(|value| value["type"] == "message")
                        .filter_map(|value| value["content"].as_array())
                        .flatten()
                        .filter(|part| part["type"] == "output_text")
                        .filter_map(|part| part["text"].as_str())
                        .collect::<String>(),
                )
            }),
        Protocol::Gemini => body["candidates"][0]["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter(|part| part["thought"] != true)
                    .filter_map(|part| part["text"].as_str())
                    .collect::<String>()
            }),
    };
    output
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .ok_or(Error::InvalidResponse)
}
