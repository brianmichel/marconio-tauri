use minimp3::{Decoder, Error as Mp3Error};
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::f32::consts::PI;
use std::io::BufReader;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFxPreset {
    Clean,
    Cassette,
    Bass,
    Radio,
}

impl AudioFxPreset {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "clean" => Some(Self::Clean),
            "cassette" => Some(Self::Cassette),
            "bass" => Some(Self::Bass),
            "radio" => Some(Self::Radio),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Clean => 0,
            Self::Cassette => 1,
            Self::Bass => 2,
            Self::Radio => 3,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Cassette,
            2 => Self::Bass,
            3 => Self::Radio,
            _ => Self::Clean,
        }
    }

}

struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: Vec<f32>,
    z2: Vec<f32>,
}

impl Biquad {
    fn new_normalized(
        b0: f32,
        b1: f32,
        b2: f32,
        a0: f32,
        a1: f32,
        a2: f32,
        channels: usize,
    ) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: vec![0.0; channels],
            z2: vec![0.0; channels],
        }
    }

    fn process(&mut self, channel: usize, x: f32) -> f32 {
        let y = self.b0 * x + self.z1[channel];
        self.z1[channel] = self.b1 * x - self.a1 * y + self.z2[channel];
        self.z2[channel] = self.b2 * x - self.a2 * y;
        y
    }

    fn lowpass(sample_rate: f32, cutoff_hz: f32, q: f32, channels: usize) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.45);
        let w0 = 2.0 * PI * cutoff / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        Self::new_normalized(b0, b1, b2, a0, a1, a2, channels)
    }

    fn highpass(sample_rate: f32, cutoff_hz: f32, q: f32, channels: usize) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.45);
        let w0 = 2.0 * PI * cutoff / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        Self::new_normalized(b0, b1, b2, a0, a1, a2, channels)
    }

    fn peaking(
        sample_rate: f32,
        frequency_hz: f32,
        q: f32,
        gain_db: f32,
        channels: usize,
    ) -> Self {
        let frequency = frequency_hz.clamp(20.0, sample_rate * 0.45);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        Self::new_normalized(b0, b1, b2, a0, a1, a2, channels)
    }

    fn lowshelf(
        sample_rate: f32,
        frequency_hz: f32,
        slope: f32,
        gain_db: f32,
        channels: usize,
    ) -> Self {
        let frequency = frequency_hz.clamp(20.0, sample_rate * 0.45);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let s = slope.max(0.1);
        let alpha = (sin_w0 / 2.0) * (((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt());
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
        Self::new_normalized(b0, b1, b2, a0, a1, a2, channels)
    }
}

struct FxProcessor {
    preset: AudioFxPreset,
    sample_rate: u32,
    channels: usize,
    high_pass: Option<Biquad>,
    low_pass: Option<Biquad>,
    mid_peak: Option<Biquad>,
    low_shelf: Option<Biquad>,
    distortion_drive: f32,
    compressor_threshold: f32,
    compressor_ratio: f32,
    makeup_gain: f32,
}

impl FxProcessor {
    fn new() -> Self {
        let mut processor = Self {
            preset: AudioFxPreset::Clean,
            sample_rate: 44_100,
            channels: 2,
            high_pass: None,
            low_pass: None,
            mid_peak: None,
            low_shelf: None,
            distortion_drive: 1.0,
            compressor_threshold: 1.0,
            compressor_ratio: 1.0,
            makeup_gain: 1.0,
        };
        processor.rebuild_chain();
        processor
    }

    fn configure(&mut self, sample_rate: u32, channels: usize, preset: AudioFxPreset) {
        let next_sample_rate = sample_rate.max(8_000);
        let next_channels = channels.max(1);
        if self.sample_rate != next_sample_rate || self.channels != next_channels || self.preset != preset {
            self.sample_rate = next_sample_rate;
            self.channels = next_channels;
            self.preset = preset;
            self.rebuild_chain();
        }
    }

    fn rebuild_chain(&mut self) {
        self.high_pass = None;
        self.low_pass = None;
        self.mid_peak = None;
        self.low_shelf = None;
        self.distortion_drive = 1.0;
        self.compressor_threshold = 1.0;
        self.compressor_ratio = 1.0;
        self.makeup_gain = 1.0;

        let sr = self.sample_rate as f32;
        let channels = self.channels;
        match self.preset {
            AudioFxPreset::Clean => {}
            AudioFxPreset::Cassette => {
                self.high_pass = Some(Biquad::highpass(sr, 220.0, 0.707, channels));
                self.low_pass = Some(Biquad::lowpass(sr, 3200.0, 0.707, channels));
                self.mid_peak = Some(Biquad::peaking(sr, 3000.0, 2.3, -8.5, channels));
                self.distortion_drive = 3.8;
                self.compressor_threshold = 0.30;
                self.compressor_ratio = 5.0;
                self.makeup_gain = 1.6;
            }
            AudioFxPreset::Bass => {
                self.high_pass = Some(Biquad::highpass(sr, 26.0, 0.707, channels));
                self.low_shelf = Some(Biquad::lowshelf(sr, 82.0, 1.0, 22.0, channels));
                self.mid_peak = Some(Biquad::peaking(sr, 190.0, 1.6, 14.0, channels));
                self.low_pass = Some(Biquad::lowpass(sr, 1450.0, 0.707, channels));
                self.distortion_drive = 5.4;
                self.compressor_threshold = 0.23;
                self.compressor_ratio = 8.5;
                self.makeup_gain = 1.95;
            }
            AudioFxPreset::Radio => {
                self.high_pass = Some(Biquad::highpass(sr, 950.0, 0.8, channels));
                self.low_pass = Some(Biquad::lowpass(sr, 1450.0, 0.8, channels));
                self.mid_peak = Some(Biquad::peaking(sr, 1180.0, 3.4, 17.0, channels));
                self.distortion_drive = 7.0;
                self.compressor_threshold = 0.18;
                self.compressor_ratio = 12.0;
                self.makeup_gain = 2.4;
            }
        }
    }

    fn process_buffer(&mut self, samples: &mut [f32]) {
        if self.preset == AudioFxPreset::Clean {
            return;
        }

        for (index, sample) in samples.iter_mut().enumerate() {
            let channel = index % self.channels;
            let mut value = *sample;

            if let Some(filter) = self.high_pass.as_mut() {
                value = filter.process(channel, value);
            }
            if let Some(filter) = self.low_shelf.as_mut() {
                value = filter.process(channel, value);
            }
            if let Some(filter) = self.mid_peak.as_mut() {
                value = filter.process(channel, value);
            }
            if let Some(filter) = self.low_pass.as_mut() {
                value = filter.process(channel, value);
            }

            value = (value * self.distortion_drive).tanh();
            value = self.compress(value);
            value *= self.makeup_gain;

            *sample = value.clamp(-1.0, 1.0);
        }
    }

    fn compress(&self, sample: f32) -> f32 {
        let threshold = self.compressor_threshold.max(0.0001);
        let ratio = self.compressor_ratio.max(1.0);
        let sign = sample.signum();
        let magnitude = sample.abs();
        if magnitude <= threshold {
            return sample;
        }
        let compressed = threshold + (magnitude - threshold) / ratio;
        sign * compressed
    }
}

struct PlaybackWorker {
    stop_tx: Sender<()>,
    join_handle: JoinHandle<()>,
}

pub struct PlaybackManager {
    worker: Option<PlaybackWorker>,
    preset: Arc<AtomicU8>,
}

impl Default for PlaybackManager {
    fn default() -> Self {
        Self {
            worker: None,
            preset: Arc::new(AtomicU8::new(AudioFxPreset::Clean.as_u8())),
        }
    }
}

impl PlaybackManager {
    pub fn set_preset(&self, preset: AudioFxPreset) {
        self.preset.store(preset.as_u8(), Ordering::Relaxed);
    }

    pub fn start_stream(&mut self, stream_url: String) {
        self.stop_stream();

        let preset = Arc::clone(&self.preset);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let join_handle = thread::spawn(move || {
            if let Err(error) = run_stream_worker(stream_url, preset, stop_rx) {
                eprintln!("[audio] worker exited with error: {}", error);
            }
        });

        self.worker = Some(PlaybackWorker {
            stop_tx,
            join_handle,
        });
    }

    pub fn stop_stream(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.stop_tx.send(());
            thread::spawn(move || {
                let _ = worker.join_handle.join();
            });
        }
    }
}

impl Drop for PlaybackManager {
    fn drop(&mut self) {
        self.stop_stream();
    }
}

fn run_stream_worker(
    stream_url: String,
    preset: Arc<AtomicU8>,
    stop_rx: Receiver<()>,
) -> Result<(), String> {
    eprintln!("[audio] opening stream {}", stream_url);
    let response = reqwest::blocking::Client::new()
        .get(&stream_url)
        .send()
        .map_err(|error| format!("stream request failed: {}", error))?;

    if !response.status().is_success() {
        return Err(format!(
            "stream request failed with status {}",
            response.status().as_u16()
        ));
    }

    let reader = BufReader::new(response);
    let mut decoder = Decoder::new(reader);
    let (_stream, stream_handle) =
        OutputStream::try_default().map_err(|error| format!("output stream error: {}", error))?;
    let sink = Sink::try_new(&stream_handle).map_err(|error| format!("sink error: {}", error))?;
    sink.play();

    let mut processor = FxProcessor::new();

    loop {
        match stop_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }

        let frame = match decoder.next_frame() {
            Ok(frame) => frame,
            Err(Mp3Error::Eof) => break,
            Err(Mp3Error::InsufficientData) => {
                thread::sleep(Duration::from_millis(8));
                continue;
            }
            Err(error) => {
                eprintln!("[audio] decoder error: {}", error);
                thread::sleep(Duration::from_millis(8));
                continue;
            }
        };

        let preset_value = AudioFxPreset::from_u8(preset.load(Ordering::Relaxed));
        let channels = frame.channels.max(1);
        let sample_rate = frame.sample_rate.max(8_000) as u32;

        let mut processed = frame
            .data
            .into_iter()
            .map(|sample| sample as f32 / i16::MAX as f32)
            .collect::<Vec<f32>>();

        processor.configure(sample_rate, channels, preset_value);
        processor.process_buffer(&mut processed);

        sink.append(SamplesBuffer::new(channels as u16, sample_rate, processed));

        while sink.len() > 24 {
            match stop_rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    sink.stop();
                    return Ok(());
                }
                Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    sink.stop();
    Ok(())
}
