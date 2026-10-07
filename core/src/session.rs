//! Dictation state machine: `Idle → Listening → Finalizing → Idle`.

use std::sync::mpsc::Receiver;

use crate::emit::Emitter;
use crate::stt::{SAMPLE_RATE, SttEngine};

const SECOND: usize = SAMPLE_RATE as usize;
/// No new word for this long ends the dictation (nothing said, or only noise).
const NO_WORDS_TIMEOUT: usize = 6 * SECOND;
/// Safety cap on one dictation.
const MAX_DICTATION: usize = 300 * SECOND;
/// Shorter sounds (a click, a breath) do not count as speech.
const MIN_SPEECH: usize = SECOND / 5;

/// Source of start/stop requests (global shortcut, D-Bus action, tray menu).
pub trait Trigger {
    /// True when a toggle was requested since the last call.
    fn poll(&mut self) -> bool;
}

impl Trigger for Receiver<()> {
    fn poll(&mut self) -> bool {
        self.try_recv().is_ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    /// Microphone open, text typed as words become stable.
    Listening,
    /// Stop requested (toggle or end of speech): the last audio is fed, then the model flushed.
    Finalizing,
}

/// Energy voice activity detector with an adaptive noise floor.
///
/// The model cannot time the end of speech: it decodes in chunks (1.28 s for the default
/// model) and emits the final punctuation well into the silence.
#[derive(Default)]
struct Vad {
    floor: Option<f32>,
    speech: usize,
    silence: usize,
}

impl Vad {
    /// −50 dBFS: anything quieter is silence whatever the noise floor.
    const MIN_LEVEL: f32 = 0.003;
    /// Speech must be 12 dB above the noise floor.
    const MARGIN: f32 = 4.0;
    /// Highest initial floor (−40 dBFS), in case the first frame is already speech.
    const MAX_INITIAL_FLOOR: f32 = 0.01;

    fn push(&mut self, audio: &[f32]) {
        for frame in audio.chunks(SECOND * 3 / 100) {
            let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
            // Follows drops at once, rises slowly (~15 s) so speech does not raise it.
            let floor = self.floor.get_or_insert(rms.min(Self::MAX_INITIAL_FLOOR));
            *floor = if rms < *floor {
                rms
            } else {
                *floor + (rms - *floor) * 0.002
            };
            if rms > (*floor * Self::MARGIN).max(Self::MIN_LEVEL) {
                self.speech += frame.len();
                self.silence = 0;
            } else {
                self.silence += frame.len();
            }
        }
    }
}

pub struct Dictation {
    engine: Box<dyn SttEngine>,
    pub emitter: Emitter,
    /// Seconds of silence after speech that end the dictation.
    pub end_silence: f32,
    state: State,
    vad: Vad,
    words: String,
    since_words: usize,
    fed: usize,
}

impl Dictation {
    pub fn new(engine: Box<dyn SttEngine>, end_silence: f32, postprocess: bool) -> Self {
        Self {
            engine,
            emitter: Emitter::new(postprocess),
            end_silence,
            state: State::Idle,
            vad: Vad::default(),
            words: String::new(),
            since_words: 0,
            fed: 0,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// Swaps the engine; refused (engine handed back) unless idle.
    pub fn set_engine(&mut self, engine: Box<dyn SttEngine>) -> Result<(), Box<dyn SttEngine>> {
        if self.state != State::Idle {
            return Err(engine);
        }
        self.engine = engine;
        Ok(())
    }

    /// Back to idle without typing anything (e.g. the microphone failed to open).
    pub fn abort(&mut self) {
        if self.state != State::Idle {
            self.engine.finish_segment();
            self.emitter.finish("");
            self.state = State::Idle;
        }
    }

    /// One step: feeds `audio` (16 kHz mono captured since the last step), then applies a
    /// pending toggle. Returns the text to type.
    pub fn tick(&mut self, trigger: &mut dyn Trigger, audio: &[f32]) -> Option<String> {
        let mut out = None;
        match self.state {
            State::Idle => {}
            State::Listening => {
                let hypothesis = self.engine.accept(audio);
                out = self.emitter.partial(&hypothesis);
                self.vad.push(audio);
                self.track_words(&hypothesis, audio.len());
                if self.end_of_speech() {
                    self.state = State::Finalizing;
                }
            }
            State::Finalizing => {
                self.engine.accept(audio);
                let text = self.engine.finish_segment();
                out = self.emitter.finish(&text);
                self.state = State::Idle;
            }
        }
        if trigger.poll() {
            self.state = match self.state {
                State::Idle => {
                    self.vad = Vad::default();
                    self.words.clear();
                    self.since_words = 0;
                    self.fed = 0;
                    State::Listening
                }
                State::Listening | State::Finalizing => State::Finalizing,
            };
        }
        out
    }

    fn track_words(&mut self, hypothesis: &str, fed: usize) {
        let words: String = hypothesis
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        if words != self.words {
            self.words = words;
            self.since_words = 0;
        } else {
            self.since_words += fed;
        }
        self.fed += fed;
    }

    fn end_of_speech(&self) -> bool {
        let end_silence = (self.end_silence * SECOND as f32) as usize;
        (self.vad.speech >= MIN_SPEECH && self.vad.silence >= end_silence)
            || self.since_words >= NO_WORDS_TIMEOUT
            || self.fed >= MAX_DICTATION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stt::SAMPLE_RATE;

    const CHUNK: usize = SAMPLE_RATE as usize / 10;

    /// One word per loud 100 ms chunk.
    #[derive(Default)]
    struct FakeEngine {
        words: usize,
    }

    impl SttEngine for FakeEngine {
        fn accept(&mut self, samples: &[f32]) -> String {
            self.words += samples
                .chunks(CHUNK)
                .filter(|c| c.iter().any(|s| s.abs() > 0.1))
                .count();
            (1..=self.words)
                .map(|i| format!("mot{i}"))
                .collect::<Vec<_>>()
                .join(" ")
        }
        fn finish_segment(&mut self) -> String {
            let text = self.accept(&[]);
            *self = Self::default();
            text
        }
    }

    /// Toggles at the given tick indices.
    struct Script(Vec<usize>, usize);

    impl Trigger for Script {
        fn poll(&mut self) -> bool {
            self.1 += 1;
            self.0.contains(&(self.1 - 1))
        }
    }

    /// Deterministic white noise of the given amplitude.
    fn noise(amplitude: f32, seed: u32) -> Vec<f32> {
        let mut x = seed.wrapping_mul(2_654_435_761) | 1;
        (0..CHUNK)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x as f32 / u32::MAX as f32 * 2.0 - 1.0) * amplitude
            })
            .collect()
    }

    fn speech() -> Vec<f32> {
        noise(0.5, 1)
    }

    /// Room noise around −60 dBFS, not digital zero.
    fn silence() -> Vec<f32> {
        noise(0.002, 2)
    }

    /// Runs `audio` chunks through a fresh dictation; returns typed text and states seen.
    fn run(toggles: &[usize], audio: &[Vec<f32>]) -> (String, Vec<State>) {
        let mut d = Dictation::new(Box::new(FakeEngine::default()), 1.2, true);
        let mut trigger = Script(toggles.to_vec(), 0);
        let mut typed = String::new();
        let mut states = vec![d.state()];
        // Like the pipeline: no audio while idle (microphone closed).
        let mut feed = |d: &mut Dictation, chunk: &[f32], typed: &mut String| {
            let audio = if d.state() == State::Idle {
                &[][..]
            } else {
                chunk
            };
            *typed += &d.tick(&mut trigger, audio).unwrap_or_default();
            if states.last() != Some(&d.state()) {
                states.push(d.state());
            }
        };
        for chunk in audio {
            feed(&mut d, chunk, &mut typed);
        }
        (typed, states)
    }

    #[test]
    fn silence_only_stops_without_typing() {
        let (typed, states) = run(&[0], &vec![silence(); 58]);
        assert_eq!(states, [State::Idle, State::Listening]);
        // Nothing said for 6 s, then one flush step.
        let (typed_after, states) = run(&[0], &vec![silence(); 63]);
        assert_eq!((typed.as_str(), typed_after.as_str()), ("", ""));
        assert_eq!(
            states,
            [
                State::Idle,
                State::Listening,
                State::Finalizing,
                State::Idle
            ]
        );
    }

    #[test]
    fn steady_background_noise_is_not_speech() {
        // A loud fan: the noise floor adapts, short silence after speech still ends it.
        let fan = |seed| noise(0.05, seed);
        let mut audio: Vec<_> = (0..20).map(fan).collect();
        audio.extend(vec![speech(); 3]);
        audio.extend((20..33).map(fan));
        let (typed, states) = run(&[0], &audio);
        assert_eq!(
            states,
            [
                State::Idle,
                State::Listening,
                State::Finalizing,
                State::Idle
            ]
        );
        assert!(typed.ends_with("mot3. "), "{typed}");
    }

    #[test]
    fn speech_keeps_listening_and_types_stable_words() {
        let mut audio = vec![silence()];
        audio.extend(vec![speech(); 5]);
        let (typed, states) = run(&[0], &audio);
        assert_eq!(typed, "Mot1 mot2 mot3 mot4 ");
        assert_eq!(states, [State::Idle, State::Listening]);
    }

    #[test]
    fn speech_then_silence_finalizes_with_the_rest() {
        let mut audio = vec![silence()];
        audio.extend(vec![speech(); 3]);
        audio.extend(vec![silence(); 11]);
        let (typed, states) = run(&[0], &audio);
        assert_eq!(states, [State::Idle, State::Listening]);
        assert_eq!(typed, "Mot1 mot2 ");

        audio.extend(vec![silence(); 2]); // 1.2 s of silence reached, then one flush step
        let (typed, states) = run(&[0], &audio);
        assert_eq!(
            states,
            [
                State::Idle,
                State::Listening,
                State::Finalizing,
                State::Idle
            ]
        );
        assert_eq!(typed, "Mot1 mot2 mot3. ");
    }

    #[test]
    fn second_toggle_stops_and_flushes() {
        let mut audio = vec![silence()];
        audio.extend(vec![speech(); 3]);
        audio.extend(vec![silence(); 3]);
        // Toggle again while speaking (tick 2).
        let (typed, states) = run(&[0, 2], &audio);
        assert_eq!(
            states,
            [
                State::Idle,
                State::Listening,
                State::Finalizing,
                State::Idle
            ]
        );
        assert_eq!(typed, "Mot1 mot2 mot3. ");
    }

    #[test]
    fn engine_swap_waits_for_idle_and_abort_types_nothing() {
        let mut d = Dictation::new(Box::new(FakeEngine::default()), 1.2, true);
        assert!(d.set_engine(Box::new(FakeEngine::default())).is_ok());
        d.tick(&mut Script(vec![0], 0), &[]);
        assert_eq!(d.state(), State::Listening);
        d.tick(&mut Script(vec![], 0), &speech());
        assert!(d.set_engine(Box::new(FakeEngine::default())).is_err());
        d.abort();
        assert_eq!(d.state(), State::Idle);
        // A new dictation does not resume the aborted one.
        d.tick(&mut Script(vec![0], 0), &[]);
        let typed = d.tick(&mut Script(vec![], 0), &[speech(), speech()].concat());
        assert_eq!(typed.as_deref(), Some("Mot1 "));
    }

    #[test]
    fn endless_speech_is_capped() {
        // New words every chunk, and an end-of-silence setting never reached: only the cap ends it.
        let mut d = Dictation::new(Box::new(FakeEngine::default()), 1e6, true);
        d.tick(&mut Script(vec![0], 0), &[]);
        let chunk = speech();
        for _ in 1..MAX_DICTATION / CHUNK {
            d.tick(&mut Script(vec![], 0), &chunk);
        }
        assert_eq!(d.state(), State::Listening);
        d.tick(&mut Script(vec![], 0), &chunk);
        assert_eq!(d.state(), State::Finalizing);
    }
}
