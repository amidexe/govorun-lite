//! The child resolves lazy providers on its main thread; the host enforces one
//! second over startup/read/exit. Its pipe is bounded and never persisted.
use super::snapshot::{Snapshot, LIMIT};
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub fn read(executable: &Path, name: &str) -> Result<Snapshot, &'static str> {
    let deadline = Instant::now() + Duration::from_secs(1);
    let child = Command::new(executable)
        .args(["--clipboard-snapshot", name])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "clipboard_helper_unavailable")?;
    let mut child = OwnedChild(child);
    let pipe = child
        .0
        .stdout
        .take()
        .ok_or("clipboard_helper_unavailable")?;
    let (send, receive) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let mut out = Vec::new();
        let result = pipe.take((LIMIT + 1) as u64).read_to_end(&mut out);
        let _ = send.send(if result.is_ok() && out.len() <= LIMIT {
            Ok(out)
        } else {
            Err("clipboard_snapshot_too_large")
        });
    });
    let mut output = None;
    let result = loop {
        match receive.try_recv() {
            Ok(value) => output = Some(value),
            Err(mpsc::TryRecvError::Disconnected) if output.is_none() => {
                break Err("clipboard_helper_unavailable")
            }
            _ => {}
        }
        match child.0.try_wait() {
            Ok(Some(status)) if !status.success() => break Err("clipboard_helper_unavailable"),
            Ok(Some(_)) if output.is_some() => {
                break output
                    .take()
                    .unwrap()
                    .and_then(|bytes| Snapshot::decode(&bytes))
            }
            Err(_) => break Err("clipboard_helper_unavailable"),
            _ => {}
        }
        if Instant::now() >= deadline {
            break Err("clipboard_helper_timeout");
        }
        thread::sleep(Duration::from_millis(5));
    };
    // Child only owns this pipe: killing it unblocks the reader. No detached
    // helper, lazy SDK object, or provider wait remains in the recording host.
    drop(child);
    let _ = reader.join();
    result
}
