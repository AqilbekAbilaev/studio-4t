// Shared persistence helper for the JSON stores.
//
// `atomic_write` writes to a sibling temp file then renames it over the target,
// so a crash or power loss mid-write can't leave a truncated/corrupt JSON file
// (a plain `fs::write` can). Each store additionally serializes its
// read-modify-write sequence with its own `std::sync::Mutex` to avoid lost
// updates when two commands touch the same file concurrently.

use crate::error::AppError;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

// Monotonic per-write sequence so two `atomic_write`s to the same target get
// distinct temp paths. The pid alone can't: every thread in this process shares
// it, so two concurrent writers would otherwise collide on one temp file.
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        match std::fs::create_dir_all(parent) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Io(e)),
        };
    }
    // Temp file beside the target so the rename stays on the same filesystem
    // (cross-device renames fail). pid guards against other processes, the
    // sequence against other threads in this one — together a unique temp path.
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("tmp.{}.{}", std::process::id(), seq));
    match std::fs::write(&tmp, content) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Io(e)),
    };
    match std::fs::rename(&tmp, path) {
        Ok(val) => val,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(AppError::Io(e));
        }
    };
    Ok(())
}
