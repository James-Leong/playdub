mod inference;
mod subtitles;

use inference::{AnalysisResult, AnalyzeVideoRequest, ExportSubtitlesRequest};
use serde::Serialize;
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    app_name: &'static str,
    version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisProgress {
    generation_id: u64,
    progress: f64,
    stage: &'static str,
    message: &'static str,
}

#[tauri::command]
fn runtime_info() -> RuntimeInfo {
    RuntimeInfo {
        app_name: "Playdub",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn authorize_media_path(app: tauri::AppHandle, media_path: String) -> Result<String, String> {
    let canonical =
        inference::validate_media_path(&media_path).map_err(|error| error.to_string())?;
    app.asset_protocol_scope()
        .allow_file(&canonical)
        .map_err(|error| format!("Could not authorize the selected media file: {error}"))?;
    Ok(canonical.to_string_lossy().into_owned())
}

#[tauri::command]
async fn analyze_video(
    app: tauri::AppHandle,
    request: AnalyzeVideoRequest,
) -> Result<AnalysisResult, String> {
    let cache_directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Could not resolve the application cache directory: {error}"))?;

    let generation_id = request.generation_id;
    let progress_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        inference::analyze_video(request, &cache_directory, |stage, progress, message| {
            let _ = progress_app.emit(
                "analysis-progress",
                AnalysisProgress {
                    generation_id,
                    progress,
                    stage,
                    message,
                },
            );
        })
    })
    .await
    .map_err(|error| format!("The local analysis task stopped unexpectedly: {error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_subtitles(request: ExportSubtitlesRequest) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || inference::export_subtitles(&request))
        .await
        .map_err(|error| format!("The subtitle export task stopped unexpectedly: {error}"))?
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the Playdub desktop application.
///
/// # Panics
///
/// Panics when the platform runtime cannot initialize or the application event loop fails.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            runtime_info,
            authorize_media_path,
            analyze_video,
            export_subtitles
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Playdub");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_metadata_matches_the_package() {
        assert_eq!(
            runtime_info(),
            RuntimeInfo {
                app_name: "Playdub",
                version: env!("CARGO_PKG_VERSION"),
            }
        );
    }
}
