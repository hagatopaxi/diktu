//! Dictates French Common Voice clips (CC0) in simulated 100 ms streaming chunks
//! through the real engine, emitter and end-of-speech detection.
//!
//! Needs a model on disk: `PARLOTTE_MODEL_DIR`, or by default the one fetched by the
//! ignored download test. Run with `cargo test -p parlotte-core -- --ignored --nocapture`.

use std::path::PathBuf;
use std::time::Instant;

use parlotte_core::session::{Dictation, State, Trigger};
use parlotte_core::stt::{SAMPLE_RATE, SherpaTransducer};

/// Aggregate word error rate allowed over the test set.
const MAX_WER: f64 = 0.25;
const END_SILENCE: f32 = 1.2;
const CHUNK: usize = SAMPLE_RATE as usize / 10;

fn model_dir() -> PathBuf {
    std::env::var_os("PARLOTTE_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let model = parlotte_core::registry::default_for("fr").unwrap();
            PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                .join("models")
                .join(model.id)
        })
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

/// Fires once, on the first poll.
struct Once(bool);

impl Trigger for Once {
    fn poll(&mut self) -> bool {
        std::mem::replace(&mut self.0, false)
    }
}

#[test]
#[ignore = "needs a downloaded model (see module doc)"]
fn dictates_french_within_wer_threshold_and_stops_on_silence() {
    let dir = model_dir();
    let files: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| !f.starts_with('.'))
        .collect();
    let engine = SherpaTransducer::load(&dir, files.iter().map(String::as_str)).unwrap();
    let mut dictation = Dictation::new(Box::new(engine), END_SILENCE, true);

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
        // Quiet room noise (about -60 dBFS) after the clip.
        let mut x = 1u32;
        let room: Vec<f32> = (0..3 * SAMPLE_RATE)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (x as f32 / u32::MAX as f32 - 0.5) * 0.004
            })
            .collect();

        let mut typed = dictation.tick(&mut Once(true), &[]).unwrap_or_default();
        let start = Instant::now();
        for chunk in wave.samples().chunks(CHUNK) {
            typed += &dictation.tick(&mut Once(false), chunk).unwrap_or_default();
            assert_eq!(
                dictation.state(),
                State::Listening,
                "{name}: stopped while speaking"
            );
        }
        busy_s += start.elapsed().as_secs_f64();
        let mut silence = 0.0;
        for chunk in room.chunks(CHUNK) {
            typed += &dictation.tick(&mut Once(false), chunk).unwrap_or_default();
            silence += 0.1;
            if dictation.state() == State::Idle {
                break;
            }
        }
        // The clips end with ~0.4 s of silence of their own.
        assert_eq!(dictation.state(), State::Idle, "{name}: still listening");
        assert!(
            silence <= END_SILENCE + 0.3,
            "{name}: stopped after {silence:.1} s"
        );
        audio_s += wave.samples().len() as f64 / f64::from(SAMPLE_RATE);

        let (exp, got) = (words(expected), words(&typed));
        let e = edit_distance(&exp, &got);
        println!(
            "{name}: {e}/{} errors, stopped {silence:.1} s after the clip\n  expected: {expected}\n  typed:    {typed:?}",
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
