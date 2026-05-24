use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{Datelike, NaiveDate};
use crossbeam_channel::Sender;
use rayon::prelude::*;

#[derive(Debug)]
pub enum SortEvent {
    Progress { done: usize, total: usize },
    Done { ok: usize, errors: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SortMode {
    Copy,
    Move,
}

fn extract_date(path: &Path) -> Option<NaiveDate> {
    // Try EXIF DateTimeOriginal first
    if let Ok(file) = std::fs::File::open(path) {
        let mut bufreader = std::io::BufReader::new(file);
        let exif_reader = exif::Reader::new();
        if let Ok(exif) = exif_reader.read_from_container(&mut bufreader) {
            if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                let s = field.display_value().to_string();
                // Format: "2024:01:15 12:34:56"
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
                    return Some(dt.date());
                }
            }
        }
    }

    // Fallback: file modification time
    if let Ok(meta) = std::fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            let dt: chrono::DateTime<chrono::Local> = mtime.into();
            return Some(dt.date_naive());
        }
    }

    None
}

fn unique_path(base: PathBuf) -> PathBuf {
    if !base.exists() {
        return base;
    }
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = base.extension().and_then(|s| s.to_str());
    let parent = base.parent().unwrap_or(Path::new("."));
    let mut i = 1u32;
    loop {
        let name = match ext {
            Some(e) => format!("{}_{}.{}", stem, i, e),
            None => format!("{}_{}", stem, i),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
        i += 1;
    }
}

pub fn sort_photos(files: Vec<PathBuf>, dest: PathBuf, mode: SortMode, tx: Sender<SortEvent>) {
    let total = files.len();

    let done = Arc::new(AtomicUsize::new(0));
    let ok = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    // Cache dirs we've already created — avoids redundant syscalls when
    // many photos share the same date (the common case for camera imports).
    let created_dirs: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

    files.par_iter().for_each(|src| {
        let date = extract_date(src).unwrap_or_else(|| chrono::Local::now().date_naive());
        let filename = src.file_name().and_then(|s| s.to_str()).unwrap_or("photo");
        let target_dir = dest
            .join(date.format("%Y").to_string())
            .join(date.format("%B").to_string())
            .join(date.day().to_string());

        let result = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Only call create_dir_all once per unique directory.
            let is_new = created_dirs.lock().unwrap().insert(target_dir.clone());
            if is_new {
                std::fs::create_dir_all(&target_dir)?;
            }

            let base = target_dir.join(filename);
            let target = unique_path(base);
            match mode {
                SortMode::Copy => { std::fs::copy(src, &target).map(|_| ())?; }
                SortMode::Move => std::fs::rename(src, &target).or_else(|_| {
                    // rename fails across filesystems — fall back to copy+delete
                    std::fs::copy(src, &target).map(|_| ())?;
                    std::fs::remove_file(src)
                })?,
            }
            Ok(())
        })();

        match result {
            Ok(_) => { ok.fetch_add(1, Ordering::Relaxed); }
            Err(_) => { errors.fetch_add(1, Ordering::Relaxed); }
        }

        // crossbeam Sender is Sync — no mutex needed.
        let d = done.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = tx.send(SortEvent::Progress { done: d, total });
    });

    let _ = tx.send(SortEvent::Done {
        ok: ok.load(Ordering::Relaxed),
        errors: errors.load(Ordering::Relaxed),
    });
}
