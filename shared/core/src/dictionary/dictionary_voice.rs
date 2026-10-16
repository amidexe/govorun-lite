use crate::dictionary_document::pair;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

fn spaces(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn normalize(raw: &str) -> String {
    spaces(raw)
        .trim_end_matches(['.', ',', '!', '?', ';', ':', '…', ' '])
        .to_owned()
}

pub(crate) fn variants(raw: &str) -> Vec<String> {
    static SEPARATOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[,;\n\r]+|[.!?…]+(?:\s|$)").expect("constant voice separator")
    });
    unique(SEPARATOR.split(raw).map(normalize), "")
}

fn unique(values: impl IntoIterator<Item = String>, exclude: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let exclude = normalize(exclude).to_lowercase();
    values
        .into_iter()
        .filter(|v| !v.is_empty() && v.to_lowercase() != exclude && seen.insert(v.to_lowercase()))
        .collect()
}

pub(crate) fn append(existing: Vec<String>, recognized: &str, correct: &str) -> Vec<String> {
    unique(
        existing
            .iter()
            .flat_map(|s| variants(s))
            .chain(variants(recognized)),
        correct,
    )
}

pub(crate) fn mappings(raw: &str, correct: &str, captured: Vec<String>) -> String {
    let correct = spaces(correct);
    if correct.is_empty() {
        return raw.to_owned();
    }
    let variants = unique(
        captured
            .iter()
            .flat_map(|s| variants(s))
            .filter(|v| !v.contains('=')),
        &correct,
    );
    upsert(raw, variants, &correct, false)
}

pub(crate) fn text_mapping(raw: &str, recognized: &str, replacement: &str) -> String {
    let source = normalize(recognized);
    if source.is_empty() || source.contains('=') {
        return raw.to_owned();
    }
    upsert(
        raw,
        unique(source.split(',').map(normalize), ""),
        &spaces(replacement),
        true,
    )
}

fn upsert(raw: &str, variants: Vec<String>, replacement: &str, bare_delete: bool) -> String {
    if variants.is_empty() {
        return raw.to_owned();
    }
    let replacements: HashMap<_, _> = variants.iter().map(|v| (v.to_lowercase(), v)).collect();
    let mut replaced = HashSet::new();
    let render = |v: &str| {
        if bare_delete && replacement.is_empty() {
            v.to_owned()
        } else {
            format!("{v}={replacement}")
        }
    };
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut result = Vec::new();
    if !raw.is_empty() {
        for line in normalized.split('\n') {
            let Some((from, old_target)) = pair(line) else {
                result.push(line.to_owned());
                continue;
            };
            let aliases: Vec<_> = from
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .collect();
            if !aliases
                .iter()
                .any(|v| replacements.contains_key(&v.to_lowercase()))
            {
                result.push(line.to_owned());
                continue;
            }
            for alias in aliases {
                let key = alias.to_lowercase();
                match replacements.get(&key) {
                    Some(variant) => {
                        if replaced.insert(key) {
                            result.push(render(variant));
                        }
                    }
                    None => result.push(format!("{alias}={old_target}")),
                }
            }
        }
    }
    for variant in variants {
        if !replaced.contains(&variant.to_lowercase()) {
            result.push(render(&variant));
        }
    }
    while result.last().is_some_and(|s| s.trim().is_empty()) {
        result.pop();
    }
    result.join("\n")
}
