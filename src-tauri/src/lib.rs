mod audio_engine;
mod shazam;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod tray_icon;

use crate::audio_engine::{AudioFxPreset, NowPlayingMetadata, PlaybackManager};
use crate::shazam::{RecognizedTrack, ShazamManager};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

#[cfg(any(target_os = "macos", target_os = "windows"))]
const TRAY_ID: &str = "main";
#[cfg(any(target_os = "macos", target_os = "windows"))]
const TRAY_MENU_NOW_PLAYING_TITLE_ID: &str = "tray.now-playing-title";
#[cfg(any(target_os = "macos", target_os = "windows"))]
const TRAY_MENU_NOW_PLAYING_SUBTITLE_ID: &str = "tray.now-playing-subtitle";
#[cfg(any(target_os = "macos", target_os = "windows"))]
const TRAY_MENU_SETTINGS_ID: &str = "tray.settings";
#[cfg(any(target_os = "macos", target_os = "windows"))]
const TRAY_MENU_OPEN_ID: &str = "tray.open";
#[cfg(any(target_os = "macos", target_os = "windows"))]
const TRAY_MENU_QUIT_ID: &str = "tray.quit";

#[derive(Default)]
struct UiState {
    menu_bar_only: bool,
}

struct ShazamState {
    manager: Arc<ShazamManager>,
}

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

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn reveal_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.show();
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn set_tray_visible<R: tauri::Runtime>(app: &tauri::AppHandle<R>, visible: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(visible);
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn apply_menu_bar_mode<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let activation_policy = if enabled {
            ActivationPolicy::Accessory
        } else {
            ActivationPolicy::Regular
        };

        app.set_activation_policy(activation_policy)
            .map_err(|error| error.to_string())?;
        app.set_dock_visibility(!enabled)
            .map_err(|error| error.to_string())?;
        set_tray_visible(app, enabled);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(window) = app.get_webview_window("main") {
            window
                .set_skip_taskbar(enabled)
                .map_err(|error| error.to_string())?;
        }
        set_tray_visible(app, enabled);
    }

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn apply_menu_bar_mode<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    _enabled: bool,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn set_menu_bar_mode(
    enabled: bool,
    app: tauri::AppHandle,
    ui_state: tauri::State<'_, Mutex<UiState>>,
) -> Result<(), String> {
    apply_menu_bar_mode(&app, enabled)?;

    let mut state = ui_state
        .lock()
        .map_err(|_| "UI state lock poisoned".to_string())?;
    state.menu_bar_only = enabled;

    Ok(())
}

#[tauri::command]
fn shazam_identify_now(
    playback: tauri::State<'_, Mutex<PlaybackManager>>,
    shazam: tauri::State<'_, ShazamState>,
) -> Result<(), String> {
    let source = {
        let manager = playback
            .lock()
            .map_err(|_| "audio engine state lock poisoned".to_string())?;
        if !manager.is_stream_running() {
            return Err("Start playback before using song recognition.".to_string());
        }
        manager.now_playing()
    };

    shazam.manager.identify_now(source)
}

#[tauri::command]
fn shazam_get_history(shazam: tauri::State<'_, ShazamState>) -> Result<Vec<RecognizedTrack>, String> {
    Ok(shazam.manager.get_history())
}

#[tauri::command]
fn shazam_clear_history(shazam: tauri::State<'_, ShazamState>) -> Result<(), String> {
    shazam.manager.clear_history()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[tauri::command]
fn set_tray_preset(slot: Option<u8>, app: tauri::AppHandle) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let (rgba, w, h) = match slot {
        Some(n) => tray_icon::cached_preset_icon(n),
        None => tray_icon::cached_idle_icon(),
    };
    let icon = tauri::image::Image::new(rgba.as_slice(), *w, *h);
    tray.set_icon(Some(icon)).map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        let _ = tray.set_icon_as_template(true);
    }

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[tauri::command]
fn set_tray_preset(_slot: Option<u8>) -> Result<(), String> {
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    title: Option<&str>,
    subtitle: Option<&str>,
) -> Result<tauri::menu::Menu<R>, String> {
    let title_text = match title {
        Some(t) => t.to_string(),
        None => "Not Playing".to_string(),
    };

    let title_item =
        MenuItemBuilder::with_id(TRAY_MENU_NOW_PLAYING_TITLE_ID, title_text)
            .enabled(false)
            .build(app)
            .map_err(|e| e.to_string())?;

    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let settings_item =
        MenuItemBuilder::with_id(TRAY_MENU_SETTINGS_ID, "Settings\u{2026}")
            .build(app)
            .map_err(|e| e.to_string())?;

    let open_item = MenuItemBuilder::with_id(TRAY_MENU_OPEN_ID, "Open Marconio")
        .build(app)
        .map_err(|e| e.to_string())?;

    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let quit_item = MenuItemBuilder::with_id(TRAY_MENU_QUIT_ID, "Quit")
        .build(app)
        .map_err(|e| e.to_string())?;

    let mut builder = MenuBuilder::new(app);

    builder = builder.item(&title_item);

    if let Some(sub) = subtitle {
        let subtitle_item =
            MenuItemBuilder::with_id(TRAY_MENU_NOW_PLAYING_SUBTITLE_ID, sub)
                .enabled(false)
                .build(app)
                .map_err(|e| e.to_string())?;
        builder = builder.item(&subtitle_item);
    }

    builder
        .items(&[&sep1, &settings_item, &open_item, &sep2, &quit_item])
        .build()
        .map_err(|e| e.to_string())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[tauri::command]
fn update_tray_menu(
    title: Option<String>,
    subtitle: Option<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let menu = build_tray_menu(&app, title.as_deref(), subtitle.as_deref())?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[tauri::command]
fn update_tray_menu(_title: Option<String>, _subtitle: Option<String>) -> Result<(), String> {
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn setup_tray<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let menu = build_tray_menu(app, None, None)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Marconio")
        .on_menu_event(|app, event| {
            if event.id() == TRAY_MENU_OPEN_ID {
                reveal_main_window(app);
                return;
            }

            if event.id() == TRAY_MENU_SETTINGS_ID {
                reveal_main_window(app);
                let _ = app.emit("tray-open-settings", ());
                return;
            }

            if event.id() == TRAY_MENU_QUIT_ID {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                reveal_main_window(tray.app_handle());
            }
        });

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app).map_err(|error| error.to_string())?;
    set_tray_visible(app, false);

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(PlaybackManager::default()))
        .manage(Mutex::new(UiState::default()))
        .setup(|app| {
            let shazam_manager = Arc::new(
                ShazamManager::new(app.handle().clone())
                    .map_err(|error| format!("[shazam] init failed: {error}"))?,
            );
            app.manage(ShazamState {
                manager: Arc::clone(&shazam_manager),
            });

            let playback_state = app.state::<Mutex<PlaybackManager>>();
            match playback_state.lock() {
                Ok(mut manager) => {
                    manager.initialize_media_controls(app.handle().clone());
                    manager.set_audio_frame_tap(Some(Arc::new({
                        let shazam_manager = Arc::clone(&shazam_manager);
                        move |samples, channels, sample_rate| {
                            shazam_manager.ingest_audio(samples, channels, sample_rate);
                        }
                    })));
                }
                Err(_) => {
                    eprintln!("[audio] unable to initialize media controls: state lock poisoned")
                }
            }

            #[cfg(any(target_os = "macos", target_os = "windows"))]
            setup_tray(&app.handle())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let hide_instead_of_close = {
                        let ui_state = window.state::<Mutex<UiState>>();
                        ui_state
                            .lock()
                            .map(|state| state.menu_bar_only)
                            .unwrap_or(false)
                    };

                    if hide_instead_of_close {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            nts_get,
            start_native_stream,
            stop_native_stream,
            set_audio_fx_preset,
            set_menu_bar_mode,
            set_tray_preset,
            update_tray_menu,
            shazam_identify_now,
            shazam_get_history,
            shazam_clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
