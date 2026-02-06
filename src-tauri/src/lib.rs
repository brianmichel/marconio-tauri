use serde_json::Value;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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

    let json = response
        .json::<Value>()
        .await
        .map_err(|error| {
            let message = error.to_string();
            eprintln!("[nts_get] json error path={} err={}", path, message);
            message
        })?;

    eprintln!("[nts_get] success path={}", path);
    Ok(json)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, nts_get])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
