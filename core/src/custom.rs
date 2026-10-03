//! User-supplied models, imported from a local folder or a Hugging Face repository.
//!
//! Only sherpa-onnx *streaming transducers* (encoder, decoder, joiner, tokens) work with
//! the streaming pipeline; other architectures are recognized and refused by name.
//! An import is copied to `<root>/.staging-<id>`, checked with [`check`], then renamed
//! to `<root>/<id>` by [`commit`]. Its `model.toml` manifest holds a registry [`Model`].

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use serde::Deserialize;

use crate::download::{self, MARKER};
use crate::registry::{Model, ModelFile};
use crate::stt::{SAMPLE_RATE, SherpaTransducer, SttEngine};

pub const PREFIX: &str = "custom-";
const MANIFEST: &str = "model.toml";
const ROLES: [&str; 4] = ["encoder", "decoder", "joiner", "tokens"];
/// Word error rate allowed on the reference clip: one short sentence is a noisy measure.
const MAX_WER: f64 = 0.5;
/// Fraction of real time the model may spend decoding; above it, dictation lags.
const MAX_RTF: f64 = 0.8;

macro_rules! sample {
    ($lang:literal) => {
        (
            $lang,
            include_bytes!(concat!("../../data/samples/", $lang, ".wav")),
            include_str!(concat!("../../data/samples/", $lang, ".txt")),
        )
    };
}

/// Reference clips (16 kHz mono 16-bit WAV, CC0) with their transcript, per language.
const SAMPLES: &[(&str, &[u8], &str)] = &[
    sample!("de"),
    sample!("en"),
    sample!("es"),
    sample!("fr"),
    sample!("ru"),
];

pub fn is_custom(model: &Model) -> bool {
    model.id.starts_with(PREFIX)
}

/// Installed custom models, sorted by name.
pub fn list(root: &Path) -> Vec<Model> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut models: Vec<Model> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(PREFIX))
        .filter_map(|e| read_manifest(&e.path()).ok())
        .filter(|m| download::is_installed(root, m))
        .collect();
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn staging_dir(root: &Path, model: &Model) -> PathBuf {
    root.join(format!(".staging-{}", model.id))
}

pub fn read_manifest(dir: &Path) -> Result<Model, String> {
    let text = fs::read_to_string(dir.join(MANIFEST)).map_err(|e| format!("{MANIFEST}: {e}"))?;
    toml::from_str(&text).map_err(|e| format!("{MANIFEST}: {e}"))
}

/// Picks the four transducer files among `names` (paths relative to the model root), or
/// explains why the set does not form a supported model.
pub fn pick(names: &[String]) -> Result<[String; 4], String> {
    let base = |n: &String| n.rsplit('/').next().unwrap_or(n).to_lowercase();
    let onnx: Vec<&String> = names
        .iter()
        .filter(|n| base(n).ends_with(".onnx"))
        .collect();
    let has = |role: &str| onnx.iter().any(|n| base(n).starts_with(role));
    if names.iter().any(|n| base(n).contains("whisper")) {
        return Err("Whisper models only transcribe whole recordings, not a live stream".into());
    }
    if onnx.is_empty() {
        return Err("no .onnx file found: Diktu needs a sherpa-onnx (ONNX) model".into());
    }
    if has("encoder") && has("decoder") && !has("joiner") {
        return Err(
            "encoder-decoder model without a joiner (Whisper, Moonshine, SenseVoice…): \
                    only streaming transducers are supported"
                .into(),
        );
    }
    if !has("encoder") {
        return Err("single-file model (CTC, Paraformer…): \
                    only streaming transducers (encoder, decoder, joiner) are supported"
            .into());
    }
    let mut picked = ROLES.map(|_| String::new());
    for (slot, role) in picked.iter_mut().zip(ROLES) {
        let mut candidates: Vec<&String> = names
            .iter()
            .filter(|n| {
                let b = base(n);
                b.starts_with(role)
                    && (b.ends_with(".onnx") || (role == "tokens" && b.ends_with(".txt")))
            })
            .collect();
        // Full precision first, then the shortest name: variants add suffixes (chunk size…).
        candidates.sort_by_key(|n| (base(n).contains("int8"), n.len(), (*n).clone()));
        *slot = candidates
            .first()
            .map(|n| (*n).clone())
            .ok_or_else(|| format!("missing `{role}` file"))?;
    }
    Ok(picked)
}

fn slug(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    s.split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn new_model(root: &Path, name: &str, lang: &str) -> Result<Model, String> {
    let slug = slug(name);
    if slug.is_empty() {
        return Err("the model needs a name".into());
    }
    if !is_lang_code(lang) {
        return Err(format!(
            "`{lang}` is not a language code (ISO 639-1, e.g. fr)"
        ));
    }
    let id = format!("{PREFIX}{slug}");
    if root.join(&id).exists() {
        return Err(format!("a model named “{name}” is already imported"));
    }
    Ok(Model {
        id,
        name: name.into(),
        langs: vec![lang.into()],
        engine: "sherpa-onnx-transducer".into(),
        license: "unknown (imported)".into(),
        repo: String::new(),
        revision: String::new(),
        files: Vec::new(),
    })
}

pub fn is_lang_code(lang: &str) -> bool {
    (2..=3).contains(&lang.len()) && lang.bytes().all(|b| b.is_ascii_lowercase())
}

fn write_staged(root: &Path, mut model: Model) -> Result<Model, String> {
    let dir = staging_dir(root, &model);
    for file in &mut model.files {
        let path = dir.join(&file.path);
        file.size = fs::metadata(&path)
            .map_err(|e| format!("{}: {e}", file.path))?
            .len();
        file.sha256 = download::hash_file(&path).map_err(|e| format!("{}: {e}", file.path))?;
    }
    if model.revision.is_empty() {
        // Content-addressed, so a local import has a stable revision too.
        model.revision = model.files[0].sha256.clone();
    }
    let manifest = toml::to_string(&model).map_err(|e| e.to_string())?;
    fs::write(dir.join(MANIFEST), manifest).map_err(|e| format!("{MANIFEST}: {e}"))?;
    Ok(model)
}

/// Copies a model found in the folder `src` to the staging area.
pub fn stage_folder(root: &Path, src: &Path, lang: &str) -> Result<Model, String> {
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut model = new_model(root, &name, lang)?;
    let names: Vec<String> = fs::read_dir(src)
        .map_err(|e| format!("{}: {e}", src.display()))?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file() || t.is_symlink()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let picked = pick(&names)?;
    let dir = staging_dir(root, &model);
    discard(root, &model);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for name in picked {
        if let Err(e) = fs::copy(src.join(&name), dir.join(&name)) {
            discard(root, &model);
            return Err(format!("copying {name}: {e}"));
        }
        model.files.push(ModelFile {
            path: name,
            size: 0,
            sha256: String::new(),
        });
    }
    let staged = write_staged(root, model.clone());
    if staged.is_err() {
        discard(root, &model);
    }
    staged
}

#[derive(Deserialize)]
struct HfModel {
    sha: String,
    siblings: Vec<HfFile>,
    #[serde(rename = "cardData", default)]
    card: Option<HfCard>,
}

#[derive(Deserialize)]
struct HfFile {
    rfilename: String,
    size: Option<u64>,
    lfs: Option<HfLfs>,
}

#[derive(Deserialize)]
struct HfLfs {
    sha256: String,
}

#[derive(Deserialize)]
struct HfCard {
    license: Option<serde_json::Value>,
}

/// `owner/name`, as typed or as a `https://huggingface.co/owner/name…` link.
pub fn parse_repo(input: &str) -> Result<String, String> {
    let s = input
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("huggingface.co/");
    let parts: Vec<&str> = s.split('/').take(2).collect();
    let ok = |p: &str| {
        !p.is_empty()
            && !p.starts_with('.')
            && p.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    match parts[..] {
        [owner, name] if ok(owner) && ok(name) => Ok(format!("{owner}/{name}")),
        _ => Err(format!(
            "“{}” is not a Hugging Face repository (owner/name)",
            input.trim()
        )),
    }
}

/// Downloads a model from the Hugging Face repository `repo` to the staging area,
/// pinned to its current commit. LFS files are verified against their SHA-256.
pub fn stage_hf(
    base_url: &str,
    root: &Path,
    repo: &str,
    lang: &str,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64),
) -> Result<Model, String> {
    let repo = parse_repo(repo)?;
    let mut model = new_model(root, repo.rsplit('/').next().unwrap(), lang)?;
    let url = format!("{base_url}/api/models/{repo}?blobs=true");
    let response = reqwest::blocking::Client::builder()
        .user_agent(concat!("diktu/", env!("CARGO_PKG_VERSION")))
        .build()
        .and_then(|c| c.get(&url).send())
        .map_err(|e| format!("network error: {e}"))?;
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Err(format!("{repo}: private or gated repository")),
        404 => return Err(format!("{repo}: no such repository")),
        s => return Err(format!("{repo}: HTTP {s}")),
    }
    let info: HfModel =
        serde_json::from_reader(response).map_err(|e| format!("{repo}: unexpected answer: {e}"))?;
    let names: Vec<String> = info.siblings.iter().map(|f| f.rfilename.clone()).collect();
    let picked = pick(&names)?;
    for name in &picked {
        if name.contains("..") || name.starts_with('/') {
            return Err(format!("{name}: unsafe path"));
        }
        let file = info.siblings.iter().find(|f| &f.rfilename == name).unwrap();
        model.files.push(ModelFile {
            path: name.clone(),
            size: file.size.unwrap_or(0),
            // Small files outside LFS only have a git SHA-1: `fetch` skips the check.
            sha256: file
                .lfs
                .as_ref()
                .map(|l| l.sha256.clone())
                .unwrap_or_default(),
        });
    }
    model.repo = repo.clone();
    model.revision = info.sha;
    model.license = match info.card.and_then(|c| c.license) {
        Some(serde_json::Value::String(l)) => l,
        _ => format!("see huggingface.co/{repo}"),
    };
    // `install` writes into `<root>/<id>`: point it at the staging directory.
    let mut staged = model.clone();
    staged.id = format!(".staging-{}", model.id);
    discard(root, &model);
    if let Err(e) = download::install(base_url, root, &staged, cancel, progress) {
        discard(root, &model);
        return Err(e.to_string());
    }
    let _ = fs::remove_file(staging_dir(root, &model).join(MARKER));
    write_staged(root, model)
}

pub fn discard(root: &Path, model: &Model) {
    let _ = fs::remove_dir_all(staging_dir(root, model));
}

/// Moves a checked import to its final place; it becomes installed.
pub fn commit(root: &Path, model: &Model) -> Result<(), String> {
    let dir = staging_dir(root, model);
    fs::write(dir.join(MARKER), &model.revision).map_err(|e| e.to_string())?;
    fs::rename(&dir, root.join(&model.id)).map_err(|e| e.to_string())
}

/// Checks the model in `dir` as far as possible without a human: file integrity and
/// format, loading, silence handling, decoding speed, and the transcription of a
/// reference clip when one exists for its language. Returns a summary.
///
/// sherpa-onnx may abort the process on a malformed model: run this in a child process.
pub fn check(dir: &Path) -> Result<String, String> {
    let model = read_manifest(dir)?;
    for file in &model.files {
        let path = dir.join(&file.path);
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", file.path))?;
        if bytes.len() as u64 != file.size || download::hex_sha256(&bytes) != file.sha256 {
            return Err(format!("{}: changed since import", file.path));
        }
        if file.path.ends_with(".onnx") && bytes.first() != Some(&0x08) {
            // An ONNX ModelProto starts with its `ir_version` field (tag 0x08).
            return Err(format!("{}: not an ONNX file", file.path));
        }
    }
    let tokens = model
        .files
        .iter()
        .find(|f| f.path.rsplit('/').next().unwrap().starts_with("tokens"))
        .ok_or("missing tokens file")?;
    let vocab =
        check_tokens(&fs::read_to_string(dir.join(&tokens.path)).map_err(|e| e.to_string())?)?;

    let load = || SherpaTransducer::load(dir, model.files.iter().map(|f| f.path.as_str()));
    let mut engine = load()?;
    let mut report = vec![format!("{vocab} tokens")];

    let silence = vec![0.0; 2 * SAMPLE_RATE as usize];
    let text = transcribe(&mut engine, &silence);
    if text.split_whitespace().count() > 2 {
        return Err(format!(
            "transcribes silence as “{text}”: files likely mismatched"
        ));
    }

    let lang = model.langs.first().map(String::as_str).unwrap_or_default();
    let sample = SAMPLES.iter().find(|(l, ..)| *l == lang);
    let audio = match sample {
        Some((_, wav, _)) => wav_samples(wav)?,
        None => noise(3 * SAMPLE_RATE as usize),
    };
    let start = Instant::now();
    let text = transcribe(&mut engine, &audio);
    let rtf = start.elapsed().as_secs_f64() / (audio.len() as f64 / f64::from(SAMPLE_RATE));
    if rtf > MAX_RTF {
        return Err(format!(
            "too slow for live dictation (real-time factor {rtf:.2})"
        ));
    }
    report.push(format!("real-time factor {rtf:.2}"));
    match sample {
        Some((_, _, expected)) => {
            let (errors, total) = word_errors(expected, &text);
            let wer = errors as f64 / total.max(1) as f64;
            if wer > MAX_WER {
                return Err(format!(
                    "poor transcription of the {lang} test sentence ({:.0} % word errors): \
                     “{text}”. Wrong language or broken model?",
                    wer * 100.0
                ));
            }
            report.push(format!(
                "{:.0} % word errors on the {lang} test sentence",
                wer * 100.0
            ));
        }
        None => report.push(format!("accuracy not checked: no {lang} test sentence")),
    }
    Ok(report.join(", "))
}

/// Feeds `samples` in 100 ms chunks, as the microphone would, and returns the final text.
fn transcribe(engine: &mut SherpaTransducer, samples: &[f32]) -> String {
    for chunk in samples.chunks(SAMPLE_RATE as usize / 10) {
        engine.accept(chunk);
    }
    engine.finish_segment().trim().to_owned()
}

/// Validates a `symbol id` per line vocabulary with ids 0..n; returns n.
fn check_tokens(text: &str) -> Result<usize, String> {
    let mut ids = Vec::new();
    for (i, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let id = line
            .rsplit_once(char::is_whitespace)
            .and_then(|(_, id)| id.parse::<usize>().ok())
            .ok_or_else(|| format!("tokens line {}: expected “symbol id”", i + 1))?;
        ids.push(id);
    }
    ids.sort_unstable();
    if ids.len() < 2 || ids.iter().enumerate().any(|(i, &id)| i != id) {
        return Err("tokens: ids must run from 0 without gaps".into());
    }
    Ok(ids.len())
}

/// 16 kHz mono 16-bit PCM samples of a WAV file.
fn wav_samples(wav: &[u8]) -> Result<Vec<f32>, String> {
    let mut rest = wav
        .get(12..)
        .filter(|_| wav.starts_with(b"RIFF"))
        .ok_or("not a WAV file")?;
    let mut format_ok = false;
    while rest.len() >= 8 {
        let len = u32::from_le_bytes(rest[4..8].try_into().unwrap()) as usize;
        let body = rest.get(8..8 + len).ok_or("truncated WAV")?;
        match &rest[..4] {
            b"fmt " => {
                let u16_at = |i: usize| u16::from_le_bytes([body[i], body[i + 1]]);
                let rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
                format_ok =
                    u16_at(0) == 1 && u16_at(2) == 1 && rate == SAMPLE_RATE && u16_at(14) == 16;
            }
            b"data" if format_ok => {
                return Ok(body
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| f32::from(i16::from_le_bytes(*b)) / 32768.0)
                    .collect());
            }
            _ => {}
        }
        rest = &rest[(8 + len + len % 2).min(rest.len())..];
    }
    Err("WAV must be 16 kHz mono 16-bit PCM".into())
}

/// Quiet deterministic noise, for languages without a reference clip.
fn noise(len: usize) -> Vec<f32> {
    let mut x = 1u32;
    (0..len)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x as f32 / u32::MAX as f32 - 0.5) * 0.01
        })
        .collect()
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace(['’', '\'', '-'], " ")
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Word-level edit distance between a reference and a hypothesis, and the reference length.
pub fn word_errors(reference: &str, hypothesis: &str) -> (usize, usize) {
    let (a, b) = (words(reference), words(hypothesis));
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for x in &a {
        let mut prev = row[0];
        row[0] += 1;
        for (j, y) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(x != y)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    (row[b.len()], a.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_transducer_files_preferring_full_precision() {
        let picked = pick(&names(&[
            "README.md",
            "encoder-epoch-99-avg-1.int8.onnx",
            "encoder-epoch-99-avg-1.onnx",
            "encoder-epoch-99-avg-1-chunk-16.onnx",
            "decoder-epoch-99-avg-1.onnx",
            "joiner-epoch-99-avg-1.int8.onnx",
            "tokens.txt",
            "test_wavs/0.wav",
        ]))
        .unwrap();
        assert_eq!(
            picked,
            [
                "encoder-epoch-99-avg-1.onnx",
                "decoder-epoch-99-avg-1.onnx",
                "joiner-epoch-99-avg-1.int8.onnx",
                "tokens.txt"
            ]
        );
        assert_eq!(
            pick(&names(&[
                "exp/encoder.onnx",
                "exp/decoder.onnx",
                "exp/joiner.onnx",
                "tokens.txt"
            ]))
            .unwrap()[0],
            "exp/encoder.onnx"
        );
    }

    #[test]
    fn refuses_other_architectures() {
        for set in [
            &[
                "base-encoder.onnx",
                "base-decoder.onnx",
                "base-tokens.txt",
                "whisper.md",
            ][..],
            &["encoder_model.onnx", "decoder_model.onnx", "tokens.txt"],
            &["model.int8.onnx", "tokens.txt"],
            &["model.safetensors", "config.json"],
            &["encoder.onnx", "decoder.onnx", "joiner.onnx"],
        ] {
            assert!(pick(&names(set)).is_err(), "{set:?}");
        }
    }

    #[test]
    fn validates_inputs() {
        assert_eq!(
            parse_repo(" https://huggingface.co/a-b/c.d/tree/main ").unwrap(),
            "a-b/c.d"
        );
        assert!(parse_repo("justname").is_err());
        assert!(parse_repo("../x").is_err());
        assert!(is_lang_code("fr") && !is_lang_code("FR") && !is_lang_code("fr-FR"));
        assert_eq!(slug("Kroko FR_v2 !"), "kroko-fr-v2");
        assert_eq!(check_tokens("<blk> 0\n▁le 1\na 2\n").unwrap(), 3);
        assert!(check_tokens("<blk> 0\na 2\n").is_err());
        assert!(check_tokens("garbage\n").is_err());
    }

    #[test]
    fn reference_clips_decode() {
        for (lang, wav, text) in SAMPLES {
            let samples = wav_samples(wav).unwrap();
            assert!(samples.len() > SAMPLE_RATE as usize, "{lang}");
            assert!(!text.trim().is_empty());
        }
        assert!(wav_samples(b"RIFF....WAVEjunk").is_err());
    }

    #[test]
    fn counts_word_errors() {
        assert_eq!(word_errors("le chat dort", "le chat dort"), (0, 3));
        assert_eq!(word_errors("le chat dort", "le chien dort bien"), (2, 3));
        assert_eq!(word_errors("L’histoire", "l'histoire").0, 0);
    }
}
