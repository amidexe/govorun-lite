use crate::{Error, Result};

/// One canonical form for public campaign names and Crockford support codes.
/// The submitted value is never retained in state or included in diagnostics.
pub(crate) fn normalize(value: &str) -> Result<String> {
    if value.chars().count() > 128 {
        return Err(Error::Http(400));
    }
    let canonical: String = value
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            if "‐‑‒–—−".contains(c) {
                '-'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect();
    if !canonical.starts_with("GVRN") {
        let bytes = canonical.as_bytes();
        if !(4..=24).contains(&bytes.len())
            || !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
            || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
            || !bytes
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(c))
        {
            return Err(Error::Http(400));
        }
        return Ok(canonical);
    }
    let compact = canonical.replace('-', "");
    if compact.len() != 12 {
        return Err(Error::Http(400));
    }
    let body: String = compact[4..]
        .chars()
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();
    if body.len() != 8
        || !body
            .bytes()
            .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c))
    {
        return Err(Error::Http(400));
    }
    Ok(format!("GVRN-{}-{}", &body[..4], &body[4..]))
}
