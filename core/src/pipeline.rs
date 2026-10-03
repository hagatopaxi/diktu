//! Inference thread: microphone → dictation → bounded channel of text to type.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel};
use std::thread;
use std::time::Duration;

use crate::audio::Capture;
use crate::session::{Dictation, State, Trigger};
use crate::stt::SttEngine;

/// What the UI shows and plays.
#[derive(Debug)]
pub enum Notice {
    /// The microphone is open: play the start sound, turn the icon red.
    Started,
    Stopped,
    Error(String),
}

pub struct Handles {
    /// One message per start/stop request.
    pub toggle: Sender<()>,
    /// New engine (model or end-of-speech setting changed); swapped in once idle.
    pub engine: Sender<Box<dyn SttEngine>>,
    pub postprocess: Arc<AtomicBool>,
    /// Silence after speech that ends the dictation, in milliseconds.
    pub end_silence_ms: Arc<AtomicU32>,
}

/// Spawns the inference thread. Text goes to `text` (bounded), state changes to `notify`.
pub fn spawn(text: SyncSender<String>, notify: impl Fn(Notice) + Send + 'static) -> Handles {
    let (toggle, toggles) = channel();
    let (engine, engines) = channel();
    let handles = Handles {
        toggle,
        engine,
        postprocess: Arc::new(AtomicBool::new(true)),
        end_silence_ms: Arc::new(AtomicU32::new(1200)),
    };
    let (pp, silence) = (handles.postprocess.clone(), handles.end_silence_ms.clone());
    thread::Builder::new()
        .name("inference".into())
        .spawn(move || run(toggles, engines, &pp, &silence, &text, &notify))
        .expect("spawn inference thread");
    handles
}

fn run(
    mut toggles: Receiver<()>,
    engines: Receiver<Box<dyn SttEngine>>,
    postprocess: &AtomicBool,
    end_silence_ms: &AtomicU32,
    text: &SyncSender<String>,
    notify: &dyn Fn(Notice),
) {
    let mut dictation: Option<Dictation> = None;
    let mut pending = None;
    let mut capture: Option<Capture> = None;
    let mut audio = Vec::new();
    loop {
        if let Some(engine) = engines.try_iter().last().or(pending.take()) {
            match &mut dictation {
                None => dictation = Some(Dictation::new(engine, 1.2, true)),
                Some(d) => pending = d.set_engine(engine).err(),
            }
        }
        audio.clear();
        if let Some(c) = &mut capture {
            c.read(&mut audio);
        }
        let Some(d) = &mut dictation else {
            if toggles.poll() {
                notify(Notice::Error(crate::i18n::tr("no model installed")));
            }
            thread::sleep(Duration::from_millis(20));
            continue;
        };
        d.emitter.postprocess = postprocess.load(Ordering::Relaxed);
        d.end_silence = end_silence_ms.load(Ordering::Relaxed) as f32 / 1000.0;
        if let Some(t) = d.tick(&mut toggles, &audio) {
            // Blocks if injection lags: the ring buffer absorbs the audio meanwhile.
            if text.send(t).is_err() {
                return;
            }
        }
        match (d.state(), capture.is_some()) {
            (State::Listening, false) => match Capture::start() {
                Ok(c) => {
                    capture = Some(c);
                    notify(Notice::Started);
                }
                Err(e) => {
                    d.abort();
                    notify(Notice::Error(e));
                }
            },
            (State::Idle, true) => {
                capture = None;
                notify(Notice::Stopped);
            }
            _ => {}
        }
        thread::sleep(Duration::from_millis(20));
    }
}
