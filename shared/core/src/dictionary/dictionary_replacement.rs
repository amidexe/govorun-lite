use crate::{dictionary_document, Error, Result};
use regex::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};
use std::sync::{Arc, Mutex, OnceLock};

struct Rule {
    source: String,
    compiled: OnceLock<Regex>,
    to: String,
    yo: bool,
}

impl Rule {
    fn pattern(&self) -> Result<&Regex> {
        if self.compiled.get().is_none() {
            let pattern = RegexBuilder::new(&self.source)
                .case_insensitive(true)
                .build()
                .map_err(|_| Error::InvalidRequest)?;
            let _ = self.compiled.set(pattern);
        }
        self.compiled.get().ok_or(Error::InvalidRequest)
    }
}

struct Compiled {
    raw: String,
    candidates: RegexSet,
    rules: Vec<Rule>,
}

// One bounded dictionary cache. Never emitted, persisted or sent to an account.
static CACHE: Mutex<Option<Arc<Compiled>>> = Mutex::new(None);
const MAX_RESULT_BYTES: usize = 8 * 1024 * 1024;

fn compile(raw: &str) -> Result<Arc<Compiled>> {
    let mut cache = CACHE.lock().map_err(|_| Error::InvalidRequest)?;
    if let Some(cached) = cache.as_ref() {
        if cached.raw == raw {
            return Ok(cached.clone());
        }
    }
    let mut rules = Vec::new();
    for entry in dictionary_document::parse(raw)? {
        for from in entry
            .from
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            rules.push(Rule {
                source: regex::escape(from),
                compiled: OnceLock::new(),
                to: entry.to.clone(),
                yo: from.to_lowercase() == "ё" && entry.to.to_lowercase() == "е",
            });
        }
    }
    let candidates = RegexSetBuilder::new(rules.iter().map(|rule| &rule.source))
        .case_insensitive(true)
        .size_limit(16 * 1024 * 1024)
        .build()
        .map_err(|_| Error::InvalidRequest)?;
    let compiled = Arc::new(Compiled {
        raw: raw.to_owned(),
        candidates,
        rules,
    });
    *cache = Some(compiled.clone());
    Ok(compiled)
}

pub(crate) fn apply(raw: &str, text: &str, enabled: bool) -> Result<String> {
    if !enabled || raw.trim().is_empty() || text.trim().is_empty() {
        return Ok(text.to_owned());
    }
    let compiled = compile(raw)?;
    let mut result = text.to_owned();
    let mut candidates = compiled.candidates.matches(&result);
    for (index, rule) in compiled.rules.iter().enumerate() {
        if !candidates.matched(index) {
            continue;
        }
        let next = if rule.yo {
            result.replace('ё', "е").replace('Ё', "Е")
        } else {
            replace(rule, &result)?
        };
        if next != result {
            // Earlier substitutions can introduce a later rule's source.
            candidates = compiled.candidates.matches(&next);
            result = next;
        }
    }
    let mut previous_space = false;
    Ok(result
        .trim()
        .chars()
        .filter(|c| {
            let keep = *c != ' ' || !previous_space;
            previous_space = *c == ' ';
            keep
        })
        .collect())
}

fn replace(rule: &Rule, text: &str) -> Result<String> {
    let mut result = String::new();
    let mut offset = 0;
    for found in rule.pattern()?.find_iter(text) {
        let before = text[..found.start()].chars().next_back();
        let after = text[found.end()..].chars().next();
        if before.is_some_and(char::is_alphanumeric) || after.is_some_and(char::is_alphanumeric) {
            continue;
        }
        if result.len() + found.start() - offset + rule.to.len() > MAX_RESULT_BYTES {
            return Err(Error::ResponseTooLarge);
        }
        result.push_str(&text[offset..found.start()]);
        result.push_str(&rule.to);
        offset = found.end();
    }
    if result.len() + text.len() - offset > MAX_RESULT_BYTES {
        return Err(Error::ResponseTooLarge);
    }
    result.push_str(&text[offset..]);
    Ok(result)
}
