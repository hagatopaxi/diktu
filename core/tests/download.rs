//! Model download against a local HTTP server: resume, bad hash, interruption, cancel.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use diktu_core::download::{self, Error};
use diktu_core::registry::{self, Model, ModelFile};
use sha2::{Digest, Sha256};

const LEN: usize = 200_000;

fn body() -> Vec<u8> {
    (0..LEN as u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect()
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn model(sha256: String) -> Model {
    Model {
        id: "test".into(),
        name: "Test".into(),
        langs: vec!["fr".into()],
        engine: "sherpa-onnx-transducer".into(),
        license: "CC0".into(),
        repo: "org/repo".into(),
        revision: "0".repeat(40),
        files: vec![ModelFile {
            path: "sub/model.bin".into(),
            size: LEN as u64,
            sha256,
        }],
    }
}

/// Serves `body()` on every path, honouring `Range: bytes=N-`. When `cut_at` is set,
/// the first response announces the full length but closes after that many bytes.
/// Returns the base URL and the `Range` header of every request.
fn serve(cut_at: Option<usize>) -> (String, Arc<Mutex<Vec<Option<String>>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let seen = ranges.clone();
    std::thread::spawn(move || {
        let body = body();
        for (n, stream) in listener.incoming().enumerate() {
            let mut stream = stream.unwrap();
            let mut range = None;
            let mut reader = BufReader::new(&stream);
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                    range = Some(v.trim().trim_end_matches('-').to_string());
                }
            }
            seen.lock().unwrap().push(range.clone());
            let start: usize = range.as_deref().map_or(0, |r| r.parse().unwrap());
            // Like Hugging Face, any Range request (even `bytes=0-`) gets a 206.
            let head = if start >= LEN {
                format!(
                    "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{LEN}\r\nContent-Length: 0\r\n\r\n"
                )
            } else if range.is_some() {
                format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{}/{LEN}\r\nContent-Length: {}\r\n\r\n",
                    LEN - 1,
                    LEN - start
                )
            } else {
                format!("HTTP/1.1 200 OK\r\nContent-Length: {LEN}\r\n\r\n")
            };
            stream.write_all(head.as_bytes()).unwrap();
            let end = match cut_at {
                Some(cut) if n == 0 => cut,
                _ => LEN,
            };
            let _ = stream.write_all(&body[start.min(end)..end]);
        }
    });
    (url, ranges)
}

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("download")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn installed_file(root: &std::path::Path) -> PathBuf {
    root.join("test/sub/model.bin")
}

#[test]
fn downloads_verifies_and_marks_installed() {
    let (url, _) = serve(None);
    let (root, m) = (tmp("ok"), model(sha(&body())));
    let mut last = (0, 0);
    download::install(&url, &root, &m, &AtomicBool::new(false), |d, t| {
        last = (d, t)
    })
    .unwrap();
    assert_eq!(last, (LEN as u64, LEN as u64));
    assert_eq!(std::fs::read(installed_file(&root)).unwrap(), body());
    assert!(!root.join("test/sub/model.bin.part").exists());
    assert!(download::is_installed(&root, &m));

    download::remove(&root, &m).unwrap();
    assert!(!download::is_installed(&root, &m));
    assert!(!root.join("test").exists());
}

#[test]
fn resumes_existing_part_with_range() {
    let (url, ranges) = serve(None);
    let (root, m) = (tmp("resume"), model(sha(&body())));
    std::fs::create_dir_all(root.join("test/sub")).unwrap();
    std::fs::write(root.join("test/sub/model.bin.part"), &body()[..1000]).unwrap();
    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert_eq!(*ranges.lock().unwrap(), [Some("1000".to_string())]);
    assert_eq!(std::fs::read(installed_file(&root)).unwrap(), body());
}

#[test]
fn rejects_invalid_hash_and_discards_data() {
    let (url, _) = serve(None);
    let (root, m) = (tmp("badhash"), model("0".repeat(64)));
    let err = download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {});
    assert!(matches!(err, Err(Error::HashMismatch(_))), "{err:?}");
    assert!(!installed_file(&root).exists());
    assert!(!root.join("test/sub/model.bin.part").exists());
    assert!(!download::is_installed(&root, &m));
}

#[test]
fn interrupted_download_resumes_on_retry() {
    let (url, ranges) = serve(Some(70_000));
    let (root, m) = (tmp("interrupt"), model(sha(&body())));
    let err = download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {});
    assert!(matches!(err, Err(Error::Http(_))), "{err:?}");
    assert!(!download::is_installed(&root, &m));
    let kept = std::fs::metadata(root.join("test/sub/model.bin.part"))
        .unwrap()
        .len();
    assert!(kept > 0 && kept <= 70_000, "{kept}");

    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert_eq!(ranges.lock().unwrap()[1], Some(kept.to_string()));
    assert_eq!(std::fs::read(installed_file(&root)).unwrap(), body());
}

#[test]
fn cancel_stops_and_keeps_part() {
    let (url, _) = serve(None);
    let (root, m) = (tmp("cancel"), model(sha(&body())));
    let cancel = AtomicBool::new(false);
    let err = download::install(&url, &root, &m, &cancel, |_, _| {
        cancel.store(true, Ordering::Relaxed)
    });
    assert!(matches!(err, Err(Error::Cancelled)), "{err:?}");
    assert!(root.join("test/sub/model.bin.part").exists());
    assert!(!download::is_installed(&root, &m));
}

#[test]
fn installed_file_is_kept_and_corrupt_file_is_replaced() {
    let (url, ranges) = serve(None);
    let (root, m) = (tmp("existing"), model(sha(&body())));
    std::fs::create_dir_all(root.join("test/sub")).unwrap();
    std::fs::write(installed_file(&root), body()).unwrap();
    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert!(
        ranges.lock().unwrap().is_empty(),
        "no request for a valid file"
    );

    std::fs::write(installed_file(&root), b"corrupt").unwrap();
    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert_eq!(ranges.lock().unwrap().len(), 1);
    assert_eq!(std::fs::read(installed_file(&root)).unwrap(), body());
}

#[test]
fn complete_part_is_verified_without_downloading_again() {
    let (url, ranges) = serve(None);
    let (root, m) = (tmp("complete"), model(sha(&body())));
    std::fs::create_dir_all(root.join("test/sub")).unwrap();
    std::fs::write(root.join("test/sub/model.bin.part"), body()).unwrap();
    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert_eq!(*ranges.lock().unwrap(), [Some(LEN.to_string())]);
    assert_eq!(std::fs::read(installed_file(&root)).unwrap(), body());
}

#[test]
fn oversized_part_is_dropped_then_download_restarts() {
    let (url, _) = serve(None);
    let (root, m) = (tmp("oversized"), model(sha(&body())));
    let part = root.join("test/sub/model.bin.part");
    std::fs::create_dir_all(part.parent().unwrap()).unwrap();
    std::fs::write(&part, vec![0; LEN + 10]).unwrap();
    let err = download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {});
    assert!(matches!(err, Err(Error::Http(_))), "{err:?}");
    assert!(!part.exists());
    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert!(download::is_installed(&root, &m));
}

#[test]
fn other_revision_is_not_installed() {
    let (url, _) = serve(None);
    let (root, mut m) = (tmp("revision"), model(sha(&body())));
    download::install(&url, &root, &m, &AtomicBool::new(false), |_, _| {}).unwrap();
    m.revision = "1".repeat(40);
    assert!(!download::is_installed(&root, &m));
}

#[test]
#[ignore = "downloads the default model (~71 MB) from Hugging Face"]
fn downloads_default_model_from_hugging_face() {
    let m = registry::default_for("fr").unwrap();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("models");
    download::install(
        download::HF_BASE_URL,
        &root,
        &m,
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    assert!(download::is_installed(&root, &m));
}
