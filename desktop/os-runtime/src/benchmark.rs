//! Recorded, hash-pinned benchmark measurements. No microphone, product
//! workflow, journal, UI projection or private text is owned by this adapter.
pub use crate::pinned_asr::Device;
use crate::{
    pinned_asr::Session,
    recognition::{Cancellation, Failure},
};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Instant};
pub const SHORT_SHA: &str = "d8aaaa18a5098d7c6de0595ae7ac1e64cacd0d4022af3595213bdaf23be77e69";
pub const LONG_SHA: &str = "1868ece0195dfa9fc2394be24865d1133c8452a8292a397db45ba8c3ed9e01e3";
pub struct Timing {
    pub load_ms: f64,
    pub cold_ms: f64,
    pub warm_ms: f64,
    pub long_ms: f64,
    pub short_audio_ms: f64,
    pub long_audio_ms: f64,
    pub backend: String,
}
pub fn discover(runtime: &Path) -> Result<Vec<Device>, &'static str> {
    Session::devices(runtime).map_err(classify_runtime)
}
fn classify_runtime(code: &'static str) -> &'static str {
    if matches!(
        code,
        "recognition_runtime_unavailable"
            | "recognition_runtime_invalid"
            | "recognition_runtime_version_mismatch"
            | "recognition_runtime_abi_mismatch"
    ) {
        "runtime_incompatible"
    } else {
        "initialization_failed"
    }
}
pub fn fixture(bytes: &[u8], expected: &str) -> Result<Vec<f32>, &'static str> {
    if bytes.len() <= 44
        || bytes.len() > 8 * 1024 * 1024
        || bytes.len() % 2 != 0
        || format!("{:x}", Sha256::digest(bytes)) != expected
    {
        return Err("samples_unavailable");
    }
    // The packaged Android fixtures have the same fixed PCM16 mono/16kHz header.
    if &bytes[0..4] != b"RIFF"
        || &bytes[8..16] != b"WAVEfmt "
        || bytes[20..24] != [1, 0, 1, 0]
        || bytes[24..28] != 16000u32.to_le_bytes()
        || bytes[34..36] != [16, 0]
        || &bytes[36..40] != b"data"
    {
        return Err("samples_unavailable");
    }
    Ok(bytes[44..]
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect())
}
pub fn run(
    model: &Path,
    runtime: &Path,
    fixtures: &Path,
    device: &Device,
    cancel: &Cancellation,
    mut progress: impl FnMut(&'static str),
) -> Result<Timing, &'static str> {
    let check = || {
        if cancel.is_cancelled() {
            Err("recording_cancelled")
        } else {
            Ok(())
        }
    };
    check()?;
    progress("fixtures");
    let read = |name: &str, hash: &str| {
        let bytes = std::fs::read(fixtures.join(name)).map_err(|_| "samples_unavailable")?;
        fixture(&bytes, hash)
    };
    let short = read("benchmark.wav", SHORT_SHA)?;
    let long = read("benchmark_long.wav", LONG_SHA)?;
    check()?;
    if !model.is_file() {
        return Err("model_missing");
    }
    progress("model");
    let start = Instant::now();
    let mut session = Session::load_selected(model, runtime, device).map_err(classify_runtime)?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.0;
    check()?;
    let decode = |session: &mut Session, pcm: &[f32]| {
        let text = session.decode_selected(pcm, cancel).map_err(|failure| {
            if failure == Failure::Cancelled {
                "recording_cancelled"
            } else {
                "recognition_failed"
            }
        })?;
        if text.trim().is_empty() {
            return Err("recognition_failed");
        }
        Ok(())
    };
    progress("cold");
    decode(&mut session, &short)?;
    let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut measure = |session: &mut Session, pcm: &[f32], stage| {
        check()?;
        progress(stage);
        let start = Instant::now();
        decode(session, pcm)?;
        Ok::<f64, &'static str>(start.elapsed().as_secs_f64() * 1000.0)
    };
    let warm_ms = measure(&mut session, &short, "warm")?;
    let long_ms = measure(&mut session, &long, "long")?;
    check()?;
    if session.backend.to_ascii_lowercase() != device.backend {
        return Err("recognition_failed");
    }
    Ok(Timing {
        load_ms,
        cold_ms,
        warm_ms,
        long_ms,
        short_audio_ms: short.len() as f64 / 16.0,
        long_audio_ms: long.len() as f64 / 16.0,
        backend: device.backend.clone(),
    })
}
