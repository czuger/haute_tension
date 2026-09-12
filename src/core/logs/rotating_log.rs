//! Opening a log file that rotates: the standard handler, and the one thing it
//! does not do.
//!
//! Setting the file aside by size and keeping a few archives behind it is all
//! this log needs. What a plain file handle does not do is create the directory
//! it writes into, and `logs/` is not versioned: a fresh clone has none, and a
//! `git clean -fdx` takes it back off a working one.
//!
//! Making it once, before handing the handler over, is not enough: the handler
//! opens its file again at every rotation, and a directory that went away under
//! a running server would then cost an error per line written for as long as the
//! server ran. Hence [`RotatingLog`], which makes the directory every time it
//! opens the file.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;

/// 512 KB per file and five archives behind it: this log carries whole payloads,
/// and yesterday's run should still be there today.
pub const MAX_BYTES: u64 = 512 * 1024;
pub const FILES_KEPT: usize = 5;

/// A rotating log file that makes its directory before opening it.
///
/// Every open goes through [`RotatingLog::open_file`]: the first one, the one
/// after each rotation, and the one that follows a directory going missing. So
/// the directory is remade wherever it went, and the log picks up again by
/// itself rather than losing every line until a restart.
#[derive(Debug)]
pub struct RotatingLog {
    path: PathBuf,
    max_bytes: u64,
    files_kept: usize,
    stream: Option<File>,
    written: u64,
}

impl RotatingLog {
    /// Open a rotating log file, creating its directory if it is missing.
    ///
    /// # Arguments
    ///
    /// * `path` - The current file; the archives are `path.1`, `path.2`, …
    /// * `max_bytes` - The size beyond which the file is set aside.
    /// * `files_kept` - How many archives are kept behind it.
    pub fn open(path: impl Into<PathBuf>, max_bytes: u64, files_kept: usize) -> Self {
        let mut log = Self {
            path: path.into(),
            max_bytes,
            files_kept,
            stream: None,
            written: 0,
        };
        log.open_file();
        log
    }

    /// The file this log writes into.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write one line, rotating first when it would not fit.
    ///
    /// A line that cannot be written is dropped rather than reported: a log that
    /// printed its own trouble would print it once per request.
    pub fn emit(&mut self, line: &str) {
        let record = format!("{line}\n");
        if self.should_rollover(record.len() as u64) {
            self.rollover();
        }
        if self.stream.is_none() {
            self.open_file();
        }
        let Some(stream) = self.stream.as_mut() else {
            return;
        };
        if stream.write_all(record.as_bytes()).is_err() {
            // The directory went away under us; the next line reopens it.
            self.stream = None;
            self.open_file();
            if let Some(stream) = self.stream.as_mut() {
                let _ = stream.write_all(record.as_bytes());
            }
        }
        self.written += record.len() as u64;
    }

    /// The line as the log is read: the time, then the line.
    pub fn format(&self, message: &str) -> String {
        format!("{}  {message}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    }

    /// Whether the next record would take the file past its size.
    fn should_rollover(&self, record: u64) -> bool {
        self.max_bytes > 0 && self.written + record > self.max_bytes
    }

    /// Set the file aside, shifting the archives behind it along by one.
    fn rollover(&mut self) {
        self.stream = None;
        if self.files_kept > 0 {
            for number in (1..=self.files_kept).rev() {
                let source = self.archive(number - 1);
                let destination = self.archive(number);
                if number == self.files_kept {
                    let _ = fs::remove_file(&destination);
                }
                if source.exists() {
                    let _ = fs::rename(&source, &destination);
                }
            }
        } else {
            let _ = fs::remove_file(&self.path);
        }
        self.open_file();
    }

    /// The name of one archive, `0` being the current file itself.
    fn archive(&self, number: usize) -> PathBuf {
        if number == 0 {
            return self.path.clone();
        }
        let mut name = self.path.clone().into_os_string();
        name.push(format!(".{number}"));
        PathBuf::from(name)
    }

    /// Open the file, its directory made first.
    fn open_file(&mut self) {
        if let Some(directory) = self.path.parent() {
            let _ = fs::create_dir_all(directory);
        }
        self.stream = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok();
        self.written = fs::metadata(&self.path).map(|file| file.len()).unwrap_or(0);
    }
}

/// Open a rotating log file with the sizes this application keeps.
pub fn open_the_log(path: impl Into<PathBuf>) -> RotatingLog {
    RotatingLog::open(path, MAX_BYTES, FILES_KEPT)
}
