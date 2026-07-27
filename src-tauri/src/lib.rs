mod inference;
mod media_server;
mod playback;
mod runtime;
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisSegments<'a> {
    generation_id: u64,
    segments: &'a [subtitles::SubtitleSegment],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeLocations {
    ffmpeg: Option<runtime::RuntimeLocation>,
    whisper_cli: Option<runtime::RuntimeLocation>,
    whisper_model: Option<String>,
    argos_translate: Option<runtime::RuntimeLocation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizedMedia {
    path: String,
    playback_url: String,
}

#[tauri::command]
fn runtime_info() -> RuntimeInfo {
    RuntimeInfo {
        app_name: "Playdub",
        version: env!("CARGO_PKG_VERSION"),
    }
}

fn resolve_runtime_locations() -> RuntimeLocations {
    RuntimeLocations {
        ffmpeg: runtime::system_executable("ffmpeg"),
        whisper_cli: runtime::system_executable("whisper-cli"),
        whisper_model: runtime::whisper_model(),
        argos_translate: runtime::system_executable("argos-translate"),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn runtime_locations() -> RuntimeLocations {
    resolve_runtime_locations()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn authorize_media_path(
    app: tauri::AppHandle,
    media_path: String,
) -> Result<AuthorizedMedia, String> {
    let canonical =
        inference::validate_media_path(&media_path).map_err(|error| error.to_string())?;
    let playback_url = app
        .state::<media_server::MediaServer>()
        .authorize(&canonical)
        .map_err(|error| format!("Could not prepare local media streaming: {error}"))?;
    Ok(AuthorizedMedia {
        path: canonical.to_string_lossy().into_owned(),
        playback_url,
    })
}

#[tauri::command]
async fn analyze_video(
    app: tauri::AppHandle,
    mut request: AnalyzeVideoRequest,
) -> Result<AnalysisResult, String> {
    let locations = resolve_runtime_locations();
    if request.ffmpeg_path.trim().is_empty() {
        request.ffmpeg_path = locations
            .ffmpeg
            .map(|location| location.path)
            .ok_or_else(|| "FFmpeg is not installed on this device.".to_owned())?;
    }
    if request.whisper_cli_path.trim().is_empty() {
        request.whisper_cli_path = locations
            .whisper_cli
            .map(|location| location.path)
            .ok_or_else(|| "whisper-cli is not installed on this device.".to_owned())?;
    }
    if request.whisper_model_path.trim().is_empty() {
        request.whisper_model_path = locations.whisper_model.ok_or_else(|| {
            "Set PLAYDUB_WHISPER_MODEL to a local Whisper model file before starting Playdub."
                .to_owned()
        })?;
    }
    if request
        .argos_translate_path
        .as_deref()
        .is_none_or(|path| path.trim().is_empty())
    {
        request.argos_translate_path = locations.argos_translate.map(|location| location.path);
    }

    let cache_directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Could not resolve the application cache directory: {error}"))?;

    let generation_id = request.generation_id;
    let progress_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        inference::analyze_video(
            request,
            &cache_directory,
            |stage, progress, message| {
                let _ = progress_app.emit(
                    "analysis-progress",
                    AnalysisProgress {
                        generation_id,
                        progress,
                        stage,
                        message,
                    },
                );
            },
            |segments| {
                let _ = progress_app.emit(
                    "analysis-segments",
                    AnalysisSegments {
                        generation_id,
                        segments,
                    },
                );
            },
        )
    })
    .await
    .map_err(|error| format!("The local analysis task stopped unexpectedly: {error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn prepare_compatible_playback(
    app: tauri::AppHandle,
    media_path: String,
    ffmpeg_path: Option<String>,
) -> Result<String, String> {
    let cache_directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Could not resolve the application cache directory: {error}"))?;
    let locations = resolve_runtime_locations();
    let resolved_ffmpeg = ffmpeg_path
        .filter(|path| !path.trim().is_empty())
        .or_else(|| locations.ffmpeg.map(|location| location.path))
        .ok_or_else(|| "FFmpeg is not installed on this device.".to_owned())?;
    let playback_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let stream = playback::prepare_compatible_playback(
            &media_path,
            &resolved_ffmpeg,
            &cache_directory,
            playback_app.state::<playback::PlaybackProcesses>().inner(),
        )
        .map_err(|error| error.to_string())?;
        playback_app
            .state::<media_server::MediaServer>()
            .authorize_directory(&stream.directory, stream.manifest_name)
            .map_err(|error| format!("Could not authorize compatibility streaming: {error}"))
    })
    .await
    .map_err(|error| format!("The playback conversion stopped unexpectedly: {error}"))?
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
        .setup(|app| {
            let server = media_server::MediaServer::start()
                .map_err(|error| format!("Could not start local media streaming: {error}"))?;
            app.manage(server);
            app.manage(playback::PlaybackProcesses::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            runtime_info,
            runtime_locations,
            authorize_media_path,
            prepare_compatible_playback,
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
