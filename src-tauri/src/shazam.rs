use crate::audio_engine::NowPlayingMetadata;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const SHAZAM_STATUS_EVENT: &str = "shazam-status";
const SHAZAM_RESULT_EVENT: &str = "shazam-result";
const SHAZAM_HISTORY_EVENT: &str = "shazam-history";
const HISTORY_FILE_NAME: &str = "shazam-history.json";
const HISTORY_LIMIT: usize = 200;
const RECOGNITION_TIMEOUT: Duration = Duration::from_secs(14);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognizedTrack {
    pub shazam_id: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub artwork_url: Option<String>,
    pub apple_music_url: Option<String>,
    pub web_url: Option<String>,
    pub recognized_at: u64,
    pub source_title: Option<String>,
    pub source_artist: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShazamStatusPayload {
    status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShazamResultPayload {
    kind: String,
    message: String,
    track: Option<RecognizedTrack>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShazamHistoryPayload {
    history: Vec<RecognizedTrack>,
}

#[derive(Default)]
struct AttemptState {
    id: u64,
    active: bool,
    source: Option<NowPlayingMetadata>,
}

pub struct ShazamManager {
    inner: Arc<ShazamInner>,
}

struct ShazamInner {
    app: AppHandle,
    history_path: PathBuf,
    history: Mutex<Vec<RecognizedTrack>>,
    attempt: Mutex<AttemptState>,
    identifying: AtomicBool,
    #[cfg(target_os = "macos")]
    bridge: Mutex<Option<MacBridge>>,
    #[cfg(target_os = "macos")]
    callback_context: Mutex<Option<Box<CallbackContext>>>,
}

impl ShazamManager {
    pub fn new(app: AppHandle) -> Result<Self, String> {
        let history_path = resolve_history_path(&app)?;
        let history = load_history(history_path.as_path())?;
        let inner = Arc::new(ShazamInner {
            app,
            history_path,
            history: Mutex::new(history),
            attempt: Mutex::new(AttemptState::default()),
            identifying: AtomicBool::new(false),
            #[cfg(target_os = "macos")]
            bridge: Mutex::new(None),
            #[cfg(target_os = "macos")]
            callback_context: Mutex::new(None),
        });

        #[cfg(target_os = "macos")]
        inner.initialize_bridge()?;

        Ok(Self { inner })
    }

    pub fn identify_now(&self, source: Option<NowPlayingMetadata>) -> Result<(), String> {
        self.inner.start_attempt(source)
    }

    pub fn ingest_audio(&self, samples: &[f32], channels: u16, sample_rate: u32) {
        self.inner.ingest_audio(samples, channels, sample_rate);
    }

    pub fn get_history(&self) -> Vec<RecognizedTrack> {
        match self.inner.history.lock() {
            Ok(history) => history.clone(),
            Err(_) => Vec::new(),
        }
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let mut history = self
            .inner
            .history
            .lock()
            .map_err(|_| "Shazam history state lock poisoned".to_string())?;
        history.clear();
        persist_history(self.inner.history_path.as_path(), history.as_slice())?;
        drop(history);
        self.inner.emit_history();
        Ok(())
    }
}

impl ShazamInner {
    fn start_attempt(self: &Arc<Self>, source: Option<NowPlayingMetadata>) -> Result<(), String> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = source;
            return Err("ShazamKit recognition is only available on macOS.".to_string());
        }

        #[cfg(target_os = "macos")]
        {
            let attempt_id = {
                let mut attempt = self
                    .attempt
                    .lock()
                    .map_err(|_| "Shazam attempt state lock poisoned".to_string())?;
                if attempt.active {
                    return Err("Song recognition is already in progress.".to_string());
                }
                attempt.id = attempt.id.saturating_add(1);
                attempt.active = true;
                attempt.source = source;
                attempt.id
            };

            self.identifying.store(true, Ordering::Release);
            if let Err(error) = self.with_bridge_mut(|bridge| bridge.start()) {
                self.identifying.store(false, Ordering::Release);
                if let Ok(mut attempt) = self.attempt.lock() {
                    attempt.active = false;
                    attempt.source = None;
                }
                return Err(error);
            }

            self.emit_status("listening");

            let weak = Arc::downgrade(self);
            std::thread::spawn(move || {
                std::thread::sleep(RECOGNITION_TIMEOUT);
                if let Some(inner) = weak.upgrade() {
                    inner.finish_timeout(attempt_id);
                }
            });

            Ok(())
        }
    }

    fn ingest_audio(&self, samples: &[f32], channels: u16, sample_rate: u32) {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (samples, channels, sample_rate);
        }

        #[cfg(target_os = "macos")]
        {
            if !self.identifying.load(Ordering::Acquire) {
                return;
            }
            if channels == 0 || samples.is_empty() {
                return;
            }
            let frame_count = samples.len() / channels as usize;
            if frame_count == 0 {
                return;
            }

            if let Err(error) = self.with_bridge_mut(|bridge| bridge.feed(samples, channels, sample_rate)) {
                self.finalize_error(error);
            }
        }
    }

    fn finish_timeout(&self, attempt_id: u64) {
        let should_finish = {
            let mut attempt = match self.attempt.lock() {
                Ok(attempt) => attempt,
                Err(_) => return,
            };
            if !attempt.active || attempt.id != attempt_id {
                false
            } else {
                attempt.active = false;
                attempt.source = None;
                true
            }
        };

        if !should_finish {
            return;
        }

        self.identifying.store(false, Ordering::Release);
        #[cfg(target_os = "macos")]
        self.stop_bridge();
        self.emit_status("idle");
        self.emit_result("noMatch", "No match found.", None);
    }

    fn finalize_no_match(&self) {
        if !self.take_active_attempt() {
            return;
        }
        self.identifying.store(false, Ordering::Release);
        #[cfg(target_os = "macos")]
        self.stop_bridge();
        self.emit_status("idle");
        self.emit_result("noMatch", "No match found.", None);
    }

    fn finalize_error(&self, message: String) {
        if !self.take_active_attempt() {
            return;
        }
        self.identifying.store(false, Ordering::Release);
        #[cfg(target_os = "macos")]
        self.stop_bridge();
        self.emit_status("idle");
        self.emit_result("error", &message, None);
    }

    fn finalize_match(&self, payload: BridgeMatchPayload) {
        let source = match self.take_active_attempt_with_source() {
            Some(source) => source,
            None => return,
        };

        self.identifying.store(false, Ordering::Release);
        #[cfg(target_os = "macos")]
        self.stop_bridge();
        self.emit_status("idle");

        let track = RecognizedTrack {
            shazam_id: payload.shazam_id,
            title: payload.title.unwrap_or_else(|| "Unknown Title".to_string()),
            artist: payload.artist.clone(),
            artwork_url: payload.artwork_url,
            apple_music_url: payload.apple_music_url,
            web_url: payload.web_url,
            recognized_at: epoch_seconds(),
            source_title: source.as_ref().map(|item| item.title.clone()),
            source_artist: source.as_ref().and_then(|item| item.artist.clone()),
        };

        let message = if let Some(artist) = track.artist.as_ref() {
            format!("Recognized: {} — {}", track.title, artist)
        } else {
            format!("Recognized: {}", track.title)
        };

        if let Err(error) = self.push_history(track.clone()) {
            self.emit_result("error", &error, None);
            return;
        }

        self.emit_result("match", &message, Some(track));
        self.emit_history();
    }

    fn push_history(&self, track: RecognizedTrack) -> Result<(), String> {
        let mut history = self
            .history
            .lock()
            .map_err(|_| "Shazam history state lock poisoned".to_string())?;

        let is_duplicate = history.iter().take(12).any(|item| {
            if let (Some(lhs), Some(rhs)) = (item.shazam_id.as_ref(), track.shazam_id.as_ref()) {
                return lhs == rhs;
            }

            item.title.eq_ignore_ascii_case(track.title.as_str())
                && item.artist.as_deref() == track.artist.as_deref()
                && item.recognized_at.saturating_add(180) >= track.recognized_at
        });

        if is_duplicate {
            return Ok(());
        }

        history.insert(0, track);
        if history.len() > HISTORY_LIMIT {
            history.truncate(HISTORY_LIMIT);
        }
        persist_history(self.history_path.as_path(), history.as_slice())
    }

    fn emit_status(&self, status: &str) {
        let payload = ShazamStatusPayload {
            status: status.to_string(),
        };
        if let Err(error) = self.app.emit(SHAZAM_STATUS_EVENT, payload) {
            eprintln!("[shazam] failed to emit status event: {error}");
        }
    }

    fn emit_result(&self, kind: &str, message: &str, track: Option<RecognizedTrack>) {
        let payload = ShazamResultPayload {
            kind: kind.to_string(),
            message: message.to_string(),
            track,
        };
        if let Err(error) = self.app.emit(SHAZAM_RESULT_EVENT, payload) {
            eprintln!("[shazam] failed to emit result event: {error}");
        }
    }

    fn emit_history(&self) {
        let history = match self.history.lock() {
            Ok(history) => history.clone(),
            Err(_) => Vec::new(),
        };

        let payload = ShazamHistoryPayload { history };
        if let Err(error) = self.app.emit(SHAZAM_HISTORY_EVENT, payload) {
            eprintln!("[shazam] failed to emit history event: {error}");
        }
    }

    fn take_active_attempt(&self) -> bool {
        let mut attempt = match self.attempt.lock() {
            Ok(attempt) => attempt,
            Err(_) => return false,
        };
        if !attempt.active {
            return false;
        }
        attempt.active = false;
        attempt.source = None;
        true
    }

    fn take_active_attempt_with_source(&self) -> Option<Option<NowPlayingMetadata>> {
        let mut attempt = self.attempt.lock().ok()?;
        if !attempt.active {
            return None;
        }
        attempt.active = false;
        Some(attempt.source.take())
    }
}

fn resolve_history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("unable to resolve app data directory: {error}"))?;
    fs::create_dir_all(app_data_dir.as_path())
        .map_err(|error| format!("unable to create app data directory: {error}"))?;
    app_data_dir.push(HISTORY_FILE_NAME);
    Ok(app_data_dir)
}

fn load_history(path: &Path) -> Result<Vec<RecognizedTrack>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "unable to read Shazam history from {}: {error}",
                path.display()
            ))
        }
    };

    serde_json::from_str::<Vec<RecognizedTrack>>(contents.as_str()).map_err(|error| {
        format!(
            "unable to parse Shazam history from {}: {error}",
            path.display()
        )
    })
}

fn persist_history(path: &Path, history: &[RecognizedTrack]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "unable to resolve parent directory for history path {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "unable to create history directory {}: {error}",
            parent.display()
        )
    })?;

    let bytes = serde_json::to_vec_pretty(history)
        .map_err(|error| format!("unable to serialize Shazam history: {error}"))?;
    fs::write(path, bytes).map_err(|error| {
        format!(
            "unable to write Shazam history to {}: {error}",
            path.display()
        )
    })
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug)]
struct BridgeMatchPayload {
    shazam_id: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    artwork_url: Option<String>,
    apple_music_url: Option<String>,
    web_url: Option<String>,
}

#[cfg(target_os = "macos")]
enum BridgeEvent {
    Match(BridgeMatchPayload),
    NoMatch,
    Error(String),
}

#[cfg(target_os = "macos")]
struct CallbackContext {
    tx: std::sync::mpsc::Sender<BridgeEvent>,
}

#[cfg(target_os = "macos")]
impl ShazamInner {
    fn initialize_bridge(self: &Arc<Self>) -> Result<(), String> {
        let (tx, rx) = std::sync::mpsc::channel::<BridgeEvent>();
        let callback_context = Box::new(CallbackContext { tx });
        let user_data = callback_context.as_ref() as *const CallbackContext as *mut std::ffi::c_void;
        let bridge = unsafe { MacBridge::create(shazam_bridge_callback, user_data)? };

        {
            let mut context_slot = self
                .callback_context
                .lock()
                .map_err(|_| "Shazam callback context lock poisoned".to_string())?;
            *context_slot = Some(callback_context);
        }

        {
            let mut bridge_slot = self
                .bridge
                .lock()
                .map_err(|_| "Shazam bridge state lock poisoned".to_string())?;
            *bridge_slot = Some(bridge);
        }

        let weak = Arc::downgrade(self);
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                let Some(inner) = weak.upgrade() else {
                    break;
                };

                match event {
                    BridgeEvent::Match(payload) => inner.finalize_match(payload),
                    BridgeEvent::NoMatch => inner.finalize_no_match(),
                    BridgeEvent::Error(message) => inner.finalize_error(message),
                }
            }
        });

        Ok(())
    }

    fn with_bridge_mut<T>(
        &self,
        f: impl FnOnce(&mut MacBridge) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut bridge = self
            .bridge
            .lock()
            .map_err(|_| "Shazam bridge lock poisoned".to_string())?;
        let bridge = bridge
            .as_mut()
            .ok_or_else(|| "Shazam bridge is not initialized".to_string())?;
        f(bridge)
    }

    fn stop_bridge(&self) {
        let mut bridge = match self.bridge.lock() {
            Ok(bridge) => bridge,
            Err(_) => return,
        };
        if let Some(bridge) = bridge.as_mut() {
            bridge.stop();
        }
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn shazam_bridge_callback(
    event_type: i32,
    title: *const std::ffi::c_char,
    artist: *const std::ffi::c_char,
    artwork_url: *const std::ffi::c_char,
    apple_music_url: *const std::ffi::c_char,
    web_url: *const std::ffi::c_char,
    error_message: *const std::ffi::c_char,
    user_data: *mut std::ffi::c_void,
) {
    if user_data.is_null() {
        return;
    }
    let context = &*(user_data as *const CallbackContext);

    let event = match event_type {
        ffi::SHAZAM_BRIDGE_EVENT_MATCH => BridgeEvent::Match(BridgeMatchPayload {
            shazam_id: None,
            title: cstring_to_string(title),
            artist: cstring_to_string(artist),
            artwork_url: cstring_to_string(artwork_url),
            apple_music_url: cstring_to_string(apple_music_url),
            web_url: cstring_to_string(web_url),
        }),
        ffi::SHAZAM_BRIDGE_EVENT_NO_MATCH => BridgeEvent::NoMatch,
        ffi::SHAZAM_BRIDGE_EVENT_ERROR => {
            let message = cstring_to_string(error_message)
                .unwrap_or_else(|| "ShazamKit failed to identify the track.".to_string());
            BridgeEvent::Error(message)
        }
        _ => return,
    };

    let _ = context.tx.send(event);
}

#[cfg(target_os = "macos")]
fn cstring_to_string(value: *const std::ffi::c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    Some(
        unsafe { std::ffi::CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(target_os = "macos")]
struct MacBridge {
    raw: *mut std::ffi::c_void,
}

#[cfg(target_os = "macos")]
unsafe impl Send for MacBridge {}

#[cfg(target_os = "macos")]
impl MacBridge {
    unsafe fn create(
        callback: ffi::ShazamBridgeCallback,
        user_data: *mut std::ffi::c_void,
    ) -> Result<Self, String> {
        let mut error_ptr: *mut std::ffi::c_char = std::ptr::null_mut();
        let raw = ffi::shazam_bridge_create(callback, user_data, &mut error_ptr);
        if raw.is_null() {
            return Err(consume_bridge_error(error_ptr));
        }
        Ok(Self { raw })
    }

    fn start(&mut self) -> Result<(), String> {
        let mut error_ptr: *mut std::ffi::c_char = std::ptr::null_mut();
        let ok = unsafe { ffi::shazam_bridge_start(self.raw, &mut error_ptr) };
        if ok {
            Ok(())
        } else {
            Err(consume_bridge_error(error_ptr))
        }
    }

    fn feed(&mut self, samples: &[f32], channels: u16, sample_rate: u32) -> Result<(), String> {
        if channels == 0 {
            return Ok(());
        }
        if samples.len() % channels as usize != 0 {
            return Err("audio frame data was not aligned with channel count".to_string());
        }

        let frame_count = (samples.len() / channels as usize) as u32;
        if frame_count == 0 {
            return Ok(());
        }

        let mut error_ptr: *mut std::ffi::c_char = std::ptr::null_mut();
        let ok = unsafe {
            ffi::shazam_bridge_feed(
                self.raw,
                samples.as_ptr(),
                frame_count,
                channels as u32,
                sample_rate as f64,
                &mut error_ptr,
            )
        };

        if ok {
            Ok(())
        } else {
            Err(consume_bridge_error(error_ptr))
        }
    }

    fn stop(&mut self) {
        unsafe {
            ffi::shazam_bridge_stop(self.raw);
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for MacBridge {
    fn drop(&mut self) {
        unsafe {
            ffi::shazam_bridge_destroy(self.raw);
        }
    }
}

#[cfg(target_os = "macos")]
fn consume_bridge_error(error_ptr: *mut std::ffi::c_char) -> String {
    if error_ptr.is_null() {
        return "Unknown Shazam bridge error".to_string();
    }

    let message = unsafe { std::ffi::CStr::from_ptr(error_ptr) }
        .to_string_lossy()
        .into_owned();

    unsafe {
        ffi::shazam_bridge_free_error(error_ptr);
    }

    message
}

#[cfg(target_os = "macos")]
mod ffi {
    pub const SHAZAM_BRIDGE_EVENT_MATCH: i32 = 1;
    pub const SHAZAM_BRIDGE_EVENT_NO_MATCH: i32 = 2;
    pub const SHAZAM_BRIDGE_EVENT_ERROR: i32 = 3;

    pub type ShazamBridgeCallback = unsafe extern "C" fn(
        event_type: i32,
        title: *const std::ffi::c_char,
        artist: *const std::ffi::c_char,
        artwork_url: *const std::ffi::c_char,
        apple_music_url: *const std::ffi::c_char,
        web_url: *const std::ffi::c_char,
        error_message: *const std::ffi::c_char,
        user_data: *mut std::ffi::c_void,
    );

    unsafe extern "C" {
        pub fn shazam_bridge_create(
            callback: ShazamBridgeCallback,
            user_data: *mut std::ffi::c_void,
            error_out: *mut *mut std::ffi::c_char,
        ) -> *mut std::ffi::c_void;

        pub fn shazam_bridge_start(
            bridge: *mut std::ffi::c_void,
            error_out: *mut *mut std::ffi::c_char,
        ) -> bool;

        pub fn shazam_bridge_feed(
            bridge: *mut std::ffi::c_void,
            samples: *const f32,
            frame_count: u32,
            channels: u32,
            sample_rate: f64,
            error_out: *mut *mut std::ffi::c_char,
        ) -> bool;

        pub fn shazam_bridge_stop(bridge: *mut std::ffi::c_void);
        pub fn shazam_bridge_destroy(bridge: *mut std::ffi::c_void);
        pub fn shazam_bridge_free_error(error_message: *mut std::ffi::c_char);
    }
}
