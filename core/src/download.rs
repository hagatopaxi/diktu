//! Resumable, verified model download from Hugging Face.
//!
//! Each file goes to `<path>.part` (resumed with `Range`), is hashed while streaming,
//! then renamed atomically. A model counts as installed only once the marker file,
//! written last, holds its revision.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

use crate::registry::{Model, ModelFile};

pub const HF_BASE_URL: &str = "https://huggingface.co";
const MARKER: &str = ".installed";

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Http(String),
    Io(io::Error),
    HashMismatch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Cancelled => write!(f, "download cancelled"),
            Error::Http(e) => write!(f, "network error: {e}"),
            Error::Io(e) => write!(f, "disk error: {e}"),
            Error::HashMismatch(p) => write!(f, "{p}: SHA-256 mismatch"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e.to_string())
    }
}

pub fn model_dir(root: &Path, model: &Model) -> PathBuf {
    root.join(&model.id)
}

pub fn is_installed(root: &Path, model: &Model) -> bool {
    fs::read_to_string(model_dir(root, model).join(MARKER)).is_ok_and(|r| r == model.revision)
}

pub fn remove(root: &Path, model: &Model) -> io::Result<()> {
    match fs::remove_dir_all(model_dir(root, model)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Downloads every missing file of `model` under `root/<id>`.
/// `progress(done, total)` is called in bytes; setting `cancel` stops at the next chunk
/// and keeps the `.part` files for a later resume.
pub fn install(
    base_url: &str,
    root: &Path,
    model: &Model,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), Error> {
    let dir = model_dir(root, model);
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("diktu/", env!("CARGO_PKG_VERSION")))
        .timeout(None)
        .build()?;
    let total = model.total_size();
    let mut done = 0;
    for file in &model.files {
        let url = format!(
            "{base_url}/{}/resolve/{}/{}",
            model.repo, model.revision, file.path
        );
        fetch(
            &client,
            &url,
            &dir.join(&file.path),
            file,
            cancel,
            &mut |n| progress(done + n, total),
        )?;
        done += file.size;
    }
    fs::write(dir.join(MARKER), &model.revision)?;
    progress(total, total);
    Ok(())
}

fn fetch(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &Path,
    file: &ModelFile,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<(), Error> {
    if dest.exists() && hash_file(dest)? == file.sha256 {
        return Ok(());
    }
    fs::create_dir_all(dest.parent().expect("model file has a parent dir"))?;
    let part = dest.with_file_name(format!(
        "{}.part",
        dest.file_name().unwrap().to_string_lossy()
    ));

    let mut hasher = Sha256::new();
    let mut have = 0;
    if part.exists() {
        have = hash_reader(File::open(&part)?, &mut hasher)?;
    }
    let mut response = client
        .get(url)
        .header(reqwest::header::RANGE, format!("bytes={have}-"))
        .send()?;
    let status = response.status();
    let mut out = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        OpenOptions::new().append(true).create(true).open(&part)?
    } else if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && have == file.size {
        File::open(&part)?
    } else if status.is_success() {
        hasher = Sha256::new();
        have = 0;
        File::create(&part)?
    } else {
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            // A `.part` longer than the file can never resume: start over next time.
            fs::remove_file(&part)?;
        }
        return Err(Error::Http(format!("{url}: HTTP {status}")));
    };

    let mut buf = vec![0; 64 * 1024];
    if status != reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let n = response
                .read(&mut buf)
                .map_err(|e| Error::Http(e.to_string()))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            hasher.update(&buf[..n]);
            have += n as u64;
            progress(have);
        }
    }
    out.sync_all()?;
    drop(out);

    if hex(&hasher.finalize()) != file.sha256 {
        fs::remove_file(&part)?;
        return Err(Error::HashMismatch(file.path.clone()));
    }
    fs::rename(&part, dest)?;
    Ok(())
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    hash_reader(File::open(path)?, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

fn hash_reader(mut reader: impl Read, hasher: &mut Sha256) -> io::Result<u64> {
    let mut buf = vec![0; 64 * 1024];
    let mut len = 0;
    loop {
        match reader.read(&mut buf)? {
            0 => return Ok(len),
            n => {
                hasher.update(&buf[..n]);
                len += n as u64;
            }
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
