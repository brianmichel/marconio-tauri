use minimp3::{Decoder, Error as Mp3Error};
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::f32::consts::PI;
use std::io::BufReader;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use tauri::Emitter;
#[cfg(target_os = "windows")]
use tauri::Manager;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingMetadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeMediaControlPayload {
    action: String,
}

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

    fn peaking(sample_rate: f32, frequency_hz: f32, q: f32, gain_db: f32, channels: usize) -> Self {
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

struct Warble {
    sample_rate: f32,
    channels: usize,
    buffer: Vec<Vec<f32>>,
    write_index: usize,
    phase: f32,
    wow_rate_hz: f32,
    wow_depth_samples: f32,
    flutter_rate_hz: f32,
    flutter_depth_samples: f32,
    base_delay_samples: f32,
}

impl Warble {
    fn new(sample_rate: f32, channels: usize) -> Self {
        let sr = sample_rate.max(8_000.0);
        let ch = channels.max(1);
        let max_delay_ms = 8.0;
        let buffer_len = ((sr * max_delay_ms / 1000.0).ceil() as usize + 4).max(32);

        Self {
            sample_rate: sr,
            channels: ch,
            buffer: vec![vec![0.0; buffer_len]; ch],
            write_index: 0,
            phase: 0.0,
            wow_rate_hz: 0.52,
            wow_depth_samples: sr * (0.95 / 1000.0),
            flutter_rate_hz: 6.7,
            flutter_depth_samples: sr * (0.22 / 1000.0),
            base_delay_samples: sr * (3.9 / 1000.0),
        }
    }

    fn delay_samples(&self) -> f32 {
        let wow = (2.0 * PI * self.wow_rate_hz * self.phase).sin();
        let flutter = (2.0 * PI * self.flutter_rate_hz * self.phase + 0.7).sin();
        let raw = self.base_delay_samples
            + wow * self.wow_depth_samples
            + flutter * self.flutter_depth_samples;
        let max_delay = (self.buffer[0].len().saturating_sub(3)) as f32;
        raw.clamp(1.0, max_delay.max(1.0))
    }

    fn process(&mut self, channel: usize, input: f32) -> f32 {
        if channel >= self.channels {
            return input;
        }

        let len = self.buffer[channel].len();
        if len < 3 {
            return input;
        }

        let delay = self.delay_samples();
        let len_f = len as f32;
        let read_position = (self.write_index as f32 - delay).rem_euclid(len_f);
        let index_a = read_position.floor() as usize;
        let index_b = (index_a + 1) % len;
        let fraction = read_position - index_a as f32;

        let delayed = self.buffer[channel][index_a] * (1.0 - fraction)
            + self.buffer[channel][index_b] * fraction;
        self.buffer[channel][self.write_index] = input;
        delayed
    }

    fn advance_frame(&mut self) {
        self.write_index = (self.write_index + 1) % self.buffer[0].len();
        self.phase += 1.0 / self.sample_rate;
        if self.phase > 60.0 {
            self.phase = 0.0;
        }
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
    warble: Option<Warble>,
    warble_mix: f32,
    distortion_drive: f32,
    saturation_mix: f32,
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
            warble: None,
            warble_mix: 0.0,
            distortion_drive: 1.0,
            saturation_mix: 0.0,
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
        if self.sample_rate != next_sample_rate
            || self.channels != next_channels
            || self.preset != preset
        {
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
        self.warble = None;
        self.warble_mix = 0.0;
        self.distortion_drive = 1.0;
        self.saturation_mix = 0.0;
        self.compressor_threshold = 1.0;
        self.compressor_ratio = 1.0;
        self.makeup_gain = 1.0;

        let sr = self.sample_rate as f32;
        let channels = self.channels;
        match self.preset {
            AudioFxPreset::Clean => {}
            AudioFxPreset::Cassette => {
                self.high_pass = Some(Biquad::highpass(sr, 105.0, 0.75, channels));
                self.low_pass = Some(Biquad::lowpass(sr, 6400.0, 0.82, channels));
                self.mid_peak = Some(Biquad::peaking(sr, 2700.0, 1.35, -3.1, channels));
                self.warble = Some(Warble::new(sr, channels));
                self.warble_mix = 0.62;
                self.distortion_drive = 1.42;
                self.saturation_mix = 0.44;
                self.compressor_threshold = 0.67;
                self.compressor_ratio = 2.9;
                self.makeup_gain = 1.08;
            }
            AudioFxPreset::Bass => {
                self.high_pass = Some(Biquad::highpass(sr, 26.0, 0.707, channels));
                self.low_shelf = Some(Biquad::lowshelf(sr, 92.0, 0.9, 7.4, channels));
                self.mid_peak = Some(Biquad::peaking(sr, 180.0, 1.0, 4.0, channels));
                self.low_pass = Some(Biquad::lowpass(sr, 9300.0, 0.8, channels));
                self.distortion_drive = 1.36;
                self.saturation_mix = 0.36;
                self.compressor_threshold = 0.69;
                self.compressor_ratio = 2.7;
                self.makeup_gain = 1.1;
            }
            AudioFxPreset::Radio => {
                self.high_pass = Some(Biquad::highpass(sr, 360.0, 0.85, channels));
                self.low_pass = Some(Biquad::lowpass(sr, 3300.0, 0.85, channels));
                self.mid_peak = Some(Biquad::peaking(sr, 1750.0, 1.65, 6.8, channels));
                self.distortion_drive = 1.8;
                self.saturation_mix = 0.58;
                self.compressor_threshold = 0.6;
                self.compressor_ratio = 4.4;
                self.makeup_gain = 1.12;
            }
        }
    }

    fn process_buffer(&mut self, samples: &mut [f32]) {
        if self.preset == AudioFxPreset::Clean {
            return;
        }

        for frame in samples.chunks_mut(self.channels) {
            for (channel, sample) in frame.iter_mut().enumerate() {
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
                if let Some(warble) = self.warble.as_mut() {
                    let warped = warble.process(channel, value);
                    value = value + (warped - value) * self.warble_mix;
                }

                let drive = self.distortion_drive.max(0.001);
                let saturated = (value * drive).tanh() / drive;
                value = value + (saturated - value) * self.saturation_mix;
                value = self.compress(value);
                value *= self.makeup_gain;

                *sample = value.clamp(-1.0, 1.0);
            }

            if let Some(warble) = self.warble.as_mut() {
                warble.advance_frame();
            }
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
    now_playing: Option<NowPlayingMetadata>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    media_controls: Option<MediaControls>,
}

impl Default for PlaybackManager {
    fn default() -> Self {
        Self {
            worker: None,
            preset: Arc::new(AtomicU8::new(AudioFxPreset::Clean.as_u8())),
            now_playing: None,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            media_controls: None,
        }
    }
}

impl PlaybackManager {
    pub fn initialize_media_controls(&mut self, app: tauri::AppHandle) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            #[cfg(target_os = "windows")]
            let hwnd = get_main_window_hwnd(&app);
            #[cfg(target_os = "macos")]
            let hwnd = None;

            let mut controls = match MediaControls::new(PlatformConfig {
                display_name: "Marconio",
                dbus_name: "me.foureyes.marconio",
                hwnd,
            }) {
                Ok(controls) => controls,
                Err(error) => {
                    eprintln!("[audio] media controls init failed: {error}");
                    return;
                }
            };

            let event_app = app.clone();
            if let Err(error) = controls.attach(move |event| {
                if let Some(action) = map_media_control_action(event) {
                    let payload = NativeMediaControlPayload {
                        action: action.to_string(),
                    };
                    if let Err(emit_error) = event_app.emit("native-media-control", payload) {
                        eprintln!("[audio] media control emit failed: {emit_error}");
                    }
                }
            }) {
                eprintln!("[audio] media controls attach failed: {error}");
                return;
            }

            if let Err(error) = controls.set_playback(MediaPlayback::Stopped) {
                eprintln!("[audio] media controls initial playback failed: {error}");
            }

            self.media_controls = Some(controls);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = app;
    }

    pub fn set_preset(&self, preset: AudioFxPreset) {
        self.preset.store(preset.as_u8(), Ordering::Relaxed);
    }

    pub fn start_stream(&mut self, stream_url: String, now_playing: Option<NowPlayingMetadata>) {
        self.stop_stream();
        if let Some(metadata) = now_playing {
            self.now_playing = Some(metadata);
            self.sync_media_metadata();
        }

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
        self.sync_media_playback_state(true);
    }

    pub fn stop_stream(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.stop_tx.send(());
            thread::spawn(move || {
                let _ = worker.join_handle.join();
            });
        }
        self.sync_media_playback_state(false);
    }

    fn sync_media_metadata(&mut self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(controls) = self.media_controls.as_mut() {
            let metadata = self.now_playing.as_ref();
            let payload = MediaMetadata {
                title: metadata.map(|item| item.title.as_str()),
                artist: metadata.and_then(|item| item.artist.as_deref()),
                album: metadata.and_then(|item| item.album.as_deref()),
                cover_url: metadata.and_then(|item| item.artwork_url.as_deref()),
                duration: None,
            };
            if let Err(error) = controls.set_metadata(payload) {
                eprintln!("[audio] media controls metadata failed: {error}");
            }
        }
    }

    fn sync_media_playback_state(&mut self, is_playing: bool) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(controls) = self.media_controls.as_mut() {
            let playback = if is_playing {
                MediaPlayback::Playing { progress: None }
            } else {
                MediaPlayback::Paused { progress: None }
            };
            if let Err(error) = controls.set_playback(playback) {
                eprintln!("[audio] media controls playback failed: {error}");
            }
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn map_media_control_action(event: MediaControlEvent) -> Option<&'static str> {
    match event {
        MediaControlEvent::Play => Some("play"),
        MediaControlEvent::Pause => Some("pause"),
        MediaControlEvent::Toggle => Some("toggle"),
        MediaControlEvent::Stop => Some("stop"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn get_main_window_hwnd(app: &tauri::AppHandle) -> Option<*mut std::ffi::c_void> {
    let window = app.get_webview_window("main")?;
    match window.hwnd() {
        Ok(hwnd) => Some(hwnd.0 as *mut std::ffi::c_void),
        Err(error) => {
            eprintln!("[audio] unable to resolve main window hwnd: {error}");
            None
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
