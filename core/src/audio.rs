//! Microphone capture: cpal callback → lock-free ring buffer → 16 kHz mono f32.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapRb};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::stt::SAMPLE_RATE;

/// An open microphone stream. Dropping it closes the device.
pub struct Capture {
    _stream: cpal::Stream,
    consumer: HeapCons<f32>,
    resampler: To16k,
    raw: Vec<f32>,
}

impl Capture {
    /// Opens the default input device with its preferred configuration.
    pub fn start() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or("aucun micro trouvé")?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        let rate = config.sample_rate();
        let channels = usize::from(config.channels());
        // Two seconds of slack between the audio and inference threads.
        let (producer, consumer) = HeapRb::<f32>::new(rate as usize * 2).split();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => open::<f32>(&device, &config.into(), channels, producer),
            cpal::SampleFormat::I16 => open::<i16>(&device, &config.into(), channels, producer),
            cpal::SampleFormat::I32 => open::<i32>(&device, &config.into(), channels, producer),
            cpal::SampleFormat::U16 => open::<u16>(&device, &config.into(), channels, producer),
            other => return Err(format!("format audio non géré : {other}")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            consumer,
            resampler: To16k::new(rate)?,
            raw: vec![0.0; rate as usize / 10],
        })
    }

    /// Appends everything captured since the last call, at 16 kHz mono, to `out`.
    pub fn read(&mut self, out: &mut Vec<f32>) {
        loop {
            let n = self.consumer.pop_slice(&mut self.raw);
            if n == 0 {
                break;
            }
            self.resampler.push(&self.raw[..n], out);
        }
    }
}

fn open<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    mut producer: ringbuf::HeapProd<f32>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            *config,
            // Real-time callback: no allocation, no lock; drops samples if the ring is full.
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks_exact(channels) {
                    let sum: f32 = frame.iter().map(|s| s.to_sample::<f32>()).sum();
                    let _ = producer.try_push(sum / channels as f32);
                }
            },
            |err| eprintln!("parlotte: flux audio : {err}"),
            None,
        )
        .map_err(|e| e.to_string())
}

/// Mono resampler from the device rate to [`SAMPLE_RATE`], fed with arbitrary slices.
pub struct To16k {
    fft: Option<Fft<f32>>,
    pending: Vec<f32>,
    output: Vec<f32>,
}

impl To16k {
    pub fn new(rate: u32) -> Result<Self, String> {
        if rate == SAMPLE_RATE {
            return Ok(Self {
                fft: None,
                pending: Vec::new(),
                output: Vec::new(),
            });
        }
        // 20 ms input chunks keep the added latency negligible.
        let fft = Fft::new(
            rate as usize,
            SAMPLE_RATE as usize,
            rate as usize / 50,
            1,
            FixedSync::Input,
        )
        .map_err(|e| e.to_string())?;
        let output = vec![0.0; fft.output_frames_max()];
        Ok(Self {
            fft: Some(fft),
            pending: Vec::new(),
            output,
        })
    }

    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let Some(fft) = &mut self.fft else {
            out.extend_from_slice(input);
            return;
        };
        self.pending.extend_from_slice(input);
        let mut start = 0;
        while self.pending.len() - start >= fft.input_frames_next() {
            let n_in = fft.input_frames_next();
            let n_out = self.output.len();
            let chunk = InterleavedSlice::new(&self.pending[start..start + n_in], 1, n_in).unwrap();
            let mut dest = InterleavedSlice::new_mut(&mut self.output, 1, n_out).unwrap();
            let (_, written) = fft
                .process_into_buffer(&chunk, &mut dest, None)
                .expect("buffers sized from the resampler");
            out.extend_from_slice(&self.output[..written]);
            start += n_in;
        }
        self.pending.drain(..start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_48k_sine_to_16k() {
        let tone = |rate: f32, n: usize| -> Vec<f32> {
            (0..n)
                .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate).sin())
                .collect()
        };
        let mut r = To16k::new(48_000).unwrap();
        let mut out = Vec::new();
        // Odd-sized slices, like a real callback would deliver.
        for chunk in tone(48_000.0, 48_000).chunks(333) {
            r.push(chunk, &mut out);
        }
        assert!((15_000..=16_000).contains(&out.len()), "{}", out.len());
        // Past the resampler's start-up delay, the signal matches a 16 kHz 440 Hz sine up to a phase shift.
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        assert!((rms(&out[2000..]) - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.05);
        let crossings = out[2000..12_000]
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        assert!((270..=282).contains(&crossings), "{crossings}"); // 440 Hz over 10 000 samples ≈ 275

        let mut same = To16k::new(16_000).unwrap();
        let mut out = Vec::new();
        same.push(&[0.5; 10], &mut out);
        assert_eq!(out, [0.5; 10]);
    }
}
