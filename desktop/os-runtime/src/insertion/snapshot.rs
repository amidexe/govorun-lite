//! Bounded opaque IPC to a local clipboard helper. Memory-only; no serde,
//! Debug, files, diagnostics, renderer projection, or network transport.
pub const LIMIT: usize = 32 * 1024 * 1024;
pub struct Snapshot {
    pub(super) revision: i64,
    pub(super) skipped: bool,
    pub(super) items: Vec<Vec<(String, Vec<u8>)>>,
}
impl Snapshot {
    pub fn from_parts(
        revision: i64,
        skipped: bool,
        items: Vec<Vec<(String, Vec<u8>)>>,
    ) -> Result<Self, &'static str> {
        let value = Self {
            revision,
            skipped,
            items,
        };
        value.encode()?;
        Ok(value)
    }
    pub fn revision(&self) -> i64 {
        self.revision
    }
    pub fn skipped_unavailable(&self) -> bool {
        self.skipped
    }
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if self.revision < 0 || self.items.len() > 128 {
            return Err("clipboard_snapshot_invalid");
        }
        let mut out = b"GOVCLIP1".to_vec();
        out.extend_from_slice(&self.revision.to_le_bytes());
        out.push(u8::from(self.skipped));
        out.extend_from_slice(&(self.items.len() as u32).to_le_bytes());
        for item in &self.items {
            if item.len() > 256 {
                return Err("clipboard_snapshot_invalid");
            }
            out.extend_from_slice(&(item.len() as u32).to_le_bytes());
            for (index, (kind, data)) in item.iter().enumerate() {
                if item[..index].iter().any(|(name, _)| name == kind) {
                    return Err("clipboard_snapshot_invalid");
                }
                if kind.is_empty() || kind.len() > 4096 {
                    return Err("clipboard_snapshot_invalid");
                }
                for bytes in [kind.as_bytes(), data.as_slice()] {
                    if bytes.len() > LIMIT
                        || out.len().saturating_add(4).saturating_add(bytes.len()) > LIMIT
                    {
                        return Err("clipboard_snapshot_too_large");
                    }
                    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(bytes);
                }
            }
        }
        Ok(out)
    }
    pub fn decode(input: &[u8]) -> Result<Self, &'static str> {
        if input.len() > LIMIT {
            return Err("clipboard_snapshot_too_large");
        }
        let mut read = Reader(input);
        if read.take(8)? != b"GOVCLIP1" {
            return Err("clipboard_snapshot_invalid");
        }
        let revision = i64::from_le_bytes(read.take(8)?.try_into().unwrap());
        if revision < 0 {
            return Err("clipboard_snapshot_invalid");
        }
        let skipped = match read.take(1)?[0] {
            0 => false,
            1 => true,
            _ => return Err("clipboard_snapshot_invalid"),
        };
        let count = read.count(128)?;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let count = read.count(256)?;
            let mut item = Vec::with_capacity(count);
            for _ in 0..count {
                let len = read.count(4096)?;
                let kind = std::str::from_utf8(read.take(len)?)
                    .map_err(|_| "clipboard_snapshot_invalid")?
                    .to_owned();
                if kind.is_empty() || item.iter().any(|(name, _)| name == &kind) {
                    return Err("clipboard_snapshot_invalid");
                }
                let len = read.count(LIMIT)?;
                item.push((kind, read.take(len)?.to_vec()));
            }
            items.push(item);
        }
        if !read.0.is_empty() {
            return Err("clipboard_snapshot_invalid");
        }
        Ok(Self {
            revision,
            skipped,
            items,
        })
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], &'static str> {
        if count > self.0.len() {
            return Err("clipboard_snapshot_invalid");
        }
        let (head, tail) = self.0.split_at(count);
        self.0 = tail;
        Ok(head)
    }
    fn count(&mut self, max: usize) -> Result<usize, &'static str> {
        let count = u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as usize;
        if count > max {
            return Err("clipboard_snapshot_invalid");
        }
        Ok(count)
    }
}
