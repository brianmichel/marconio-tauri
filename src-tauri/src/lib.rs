mod audio_engine;

use crate::audio_engine::{AudioFxPreset, NowPlayingMetadata, PlaybackManager};
use serde_json::Value;
use std::sync::Mutex;
use tauri::Manager;

#[tauri::command]
async fn nts_get(path: &str) -> Result<Value, String> {
    eprintln!("[nts_get] start path={}", path);

    if path != "live" && path != "mixtapes" {
        let message = format!("unsupported NTS path: {path}");
        eprintln!("[nts_get] {}", message);
        return Err(message);
    }

    let url = format!("https://nts.live/api/v2/{path}");
    eprintln!("[nts_get] requesting {}", url);
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| {
            let message = error.to_string();
            eprintln!("[nts_get] request error path={} err={}", path, message);
            message
        })?;

    let status = response.status();
    eprintln!("[nts_get] status path={} status={}", path, status.as_u16());
    if !status.is_success() {
        let message = format!("NTS request failed with status {}", status.as_u16());
        eprintln!("[nts_get] {}", message);
        return Err(message);
    }

    let json = response.json::<Value>().await.map_err(|error| {
        let message = error.to_string();
        eprintln!("[nts_get] json error path={} err={}", path, message);
        message
    })?;

    eprintln!("[nts_get] success path={}", path);
    Ok(json)
}

#[tauri::command]
fn start_native_stream(
    stream_url: String,
    now_playing: Option<NowPlayingMetadata>,
    playback: tauri::State<'_, Mutex<PlaybackManager>>,
) -> Result<(), String> {
    let mut manager = playback
        .lock()
        .map_err(|_| "audio engine state lock poisoned".to_string())?;
    manager.start_stream(stream_url, now_playing);
    Ok(())
}

#[tauri::command]
fn stop_native_stream(playback: tauri::State<'_, Mutex<PlaybackManager>>) -> Result<(), String> {
    let mut manager = playback
        .lock()
        .map_err(|_| "audio engine state lock poisoned".to_string())?;
    manager.stop_stream();
    Ok(())
}

#[tauri::command]
fn set_audio_fx_preset(
    preset: String,
    playback: tauri::State<'_, Mutex<PlaybackManager>>,
) -> Result<(), String> {
    let parsed = AudioFxPreset::from_str(&preset)
        .ok_or_else(|| format!("unsupported audio fx preset: {preset}"))?;
    let manager = playback
        .lock()
        .map_err(|_| "audio engine state lock poisoned".to_string())?;
    manager.set_preset(parsed);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(PlaybackManager::default()))
        .setup(|app| {
            let state = app.state::<Mutex<PlaybackManager>>();
            match state.lock() {
                Ok(mut manager) => manager.initialize_media_controls(app.handle().clone()),
                Err(_) => {
                    eprintln!("[audio] unable to initialize media controls: state lock poisoned")
                }
            }
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            nts_get,
            start_native_stream,
            stop_native_stream,
            set_audio_fx_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
