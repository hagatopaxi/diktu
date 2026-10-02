//! Streaming speech-to-text engines.

use std::path::Path;

use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};

/// Sample rate every engine consumes (mono f32).
pub const SAMPLE_RATE: u32 = 16_000;

/// A streaming recognizer fed with 16 kHz mono audio, one segment at a time.
pub trait SttEngine: Send {
    /// Feeds audio and returns the current hypothesis for the segment.
    fn accept(&mut self, samples: &[f32]) -> String;
    /// True when the engine's endpoint rules consider the segment over.
    fn is_endpoint(&self) -> bool;
    /// Flushes the model and returns the final segment text, then starts a new segment.
    fn finish_segment(&mut self) -> String;
}

/// sherpa-onnx streaming transducer (encoder/decoder/joiner + tokens), greedy search.
pub struct SherpaTransducer {
    recognizer: OnlineRecognizer,
    stream: OnlineStream,
}

impl SherpaTransducer {
    /// Loads a model from `dir`; files are matched by their `encoder`, `decoder`,
    /// `joiner` and `tokens` name prefixes. `endpoint_silence` is in seconds.
    pub fn load<'a>(
        dir: &Path,
        files: impl IntoIterator<Item = &'a str>,
        endpoint_silence: f32,
    ) -> Result<Self, String> {
        let files: Vec<&str> = files.into_iter().collect();
        let find = |prefix: &str| {
            files
                .iter()
                .find(|f| f.starts_with(prefix))
                .map(|f| dir.join(f).to_string_lossy().into_owned())
                .ok_or_else(|| format!("no `{prefix}*` file in model"))
        };
        let mut config = OnlineRecognizerConfig::default();
        config.model_config.transducer.encoder = Some(find("encoder")?);
        config.model_config.transducer.decoder = Some(find("decoder")?);
        config.model_config.transducer.joiner = Some(find("joiner")?);
        config.model_config.tokens = Some(find("tokens")?);
        config.model_config.num_threads = 2;
        config.decoding_method = Some("greedy_search".into());
        config.enable_endpoint = true;
        // Rule 1 (no speech at all) is left to the session's own silence timeout.
        config.rule1_min_trailing_silence = 10.0;
        config.rule2_min_trailing_silence = endpoint_silence;
        config.rule3_min_utterance_length = 20.0;
        let recognizer = OnlineRecognizer::create(&config)
            .ok_or_else(|| format!("sherpa-onnx could not load model in {}", dir.display()))?;
        let stream = recognizer.create_stream();
        Ok(Self { recognizer, stream })
    }

    fn decode(&self) -> String {
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
        }
        self.recognizer
            .get_result(&self.stream)
            .map(|r| r.text)
            .unwrap_or_default()
    }
}

impl SttEngine for SherpaTransducer {
    fn accept(&mut self, samples: &[f32]) -> String {
        self.stream.accept_waveform(SAMPLE_RATE as i32, samples);
        self.decode()
    }

    fn is_endpoint(&self) -> bool {
        self.recognizer.is_endpoint(&self.stream)
    }

    fn finish_segment(&mut self) -> String {
        // Tail padding lets the model emit the last tokens still in its context window.
        self.stream
            .accept_waveform(SAMPLE_RATE as i32, &[0.0; SAMPLE_RATE as usize * 3 / 10]);
        self.stream.input_finished();
        let text = self.decode();
        self.stream = self.recognizer.create_stream();
        text
    }
}
