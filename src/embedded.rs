//! Embedded vPIC data, decompressed to disk on first run.
//!
//! When the `embedded` feature is on, the crate ships zstd-compressed `.bin.zst`
//! files plus uncompressed `.fst` indexes inside its source tree. On
//! [`crate::Decoder::new`] we lazily decompress the blobs to the resolved data
//! directory if the version stamp doesn't match.
//!
//! The CI cron refreshes these files monthly; bumping the patch version of the
//! crate automatically invalidates the on-disk cache because the [`VERSION`]
//! string changes.

use std::fs;
use std::io::Write;
use std::path::Path;

use fs2::FileExt;

include!(concat!(env!("OUT_DIR"), "/embedded_data.rs"));

/// Decompress + install the embedded data set into `dir` if not already current.
///
/// Race-safe across concurrent processes: an exclusive `flock` on
/// `<dir>/.install.lock` serialises extraction so two processes never write
/// the same file in parallel. After the lock is acquired we re-check the
/// VERSION stamp — if a peer already finished, we return without reinstalling.
pub(crate) fn ensure_installed(dir: &Path) -> crate::Result<()> {
    let stamp = dir.join("VERSION");
    if let Ok(existing) = fs::read_to_string(&stamp) {
        if existing.trim() == VERSION {
            return Ok(());
        }
    }
    fs::create_dir_all(dir)?;

    // OS-level lock to prevent concurrent extraction from peer processes.
    // Drops on scope exit (or on process death — kernel cleanup).
    let lock_path = dir.join(".install.lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)?;
    lock.lock_exclusive()?;

    // Re-check stamp after acquiring the lock — a peer may have finished
    // while we were waiting.
    if let Ok(existing) = fs::read_to_string(&stamp) {
        if existing.trim() == VERSION {
            let _ = FileExt::unlock(&lock);
            return Ok(());
        }
    }

    for (name, payload, compressed) in FILES {
        let path = dir.join(name);
        let bytes: std::borrow::Cow<'_, [u8]> = if *compressed {
            let decoded = zstd::decode_all(*payload).map_err(crate::Error::Io)?;
            std::borrow::Cow::Owned(decoded)
        } else {
            std::borrow::Cow::Borrowed(*payload)
        };
        let mut f = fs::File::create(&path)?;
        f.write_all(&bytes)?;
        f.sync_all().ok();
    }
    fs::write(&stamp, VERSION)?;
    let _ = FileExt::unlock(&lock);
    Ok(())
}
