//! Transcribes French Common Voice clips (CC0) in simulated 100 ms streaming chunks.
//!
//! Needs a model on disk: `PARLOTTE_MODEL_DIR` (default: the app's install dir for
//! the default model). Run with `cargo test -p parlotte-core --test stt -- --ignored --nocapture`.

use std::path::PathBuf;
use std::time::Instant;

use parlotte_core::stt::{SAMPLE_RATE, SherpaTransducer, SttEngine};

/// Aggregate word error rate allowed over the test set.
const MAX_WER: f64 = 0.25;

fn model_dir() -> PathBuf {
    std::env::var_os("PARLOTTE_MODEL_DIR")
        .map(PathBuf::from)
        .expect("set PARLOTTE_MODEL_DIR to a downloaded sherpa-onnx transducer")
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

fn edit_distance(a: &[String], b: &[String]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for x in a {
        let mut prev = row[0];
        row[0] += 1;
        for (j, y) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(x != y)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

#[test]
fn edit_distance_counts_word_errors() {
    let w = |s: &str| words(s);
    assert_eq!(edit_distance(&w("le chat dort"), &w("le chat dort")), 0);
    assert_eq!(
        edit_distance(&w("le chat dort"), &w("le chien dort bien")),
        2
    );
    assert_eq!(edit_distance(&w("L’histoire"), &w("l'histoire")), 0);
}

#[test]
#[ignore = "needs a downloaded model (PARLOTTE_MODEL_DIR)"]
fn transcribes_french_within_wer_threshold() {
    let dir = model_dir();
    let files: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let mut engine = SherpaTransducer::load(&dir, files.iter().map(String::as_str), 1.2).unwrap();

    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let (mut errors, mut total, mut audio_s, mut busy_s) = (0, 0, 0.0, 0.0);
    for line in std::fs::read_to_string(data.join("trans.txt"))
        .unwrap()
        .lines()
    {
        let (name, expected) = line.split_once('|').unwrap();
        let wave =
            sherpa_onnx::Wave::read(data.join(format!("{name}.wav")).to_str().unwrap()).unwrap();
        assert_eq!(wave.sample_rate() as u32, SAMPLE_RATE);

        let start = Instant::now();
        let mut text = String::new();
        for chunk in wave.samples().chunks(SAMPLE_RATE as usize / 10) {
            let partial = engine.accept(chunk);
            if engine.is_endpoint() {
                text += &engine.finish_segment();
                text.push(' ');
            } else {
                assert!(!partial.contains('\u{FFFD}'));
            }
        }
        text += &engine.finish_segment();
        busy_s += start.elapsed().as_secs_f64();
        audio_s += wave.samples().len() as f64 / f64::from(SAMPLE_RATE);

        let (exp, got) = (words(expected), words(&text));
        let e = edit_distance(&exp, &got);
        println!(
            "{name}: {e}/{} errors\n  expected: {expected}\n  got:      {text}",
            exp.len()
        );
        errors += e;
        total += exp.len();
    }
    let wer = errors as f64 / total as f64;
    println!(
        "WER {:.1}% ({errors}/{total}), RTF {:.3}",
        wer * 100.0,
        busy_s / audio_s
    );
    assert!(wer <= MAX_WER, "WER {wer:.3} above {MAX_WER}");
}
