use crate::inference::{
    InferenceError, MAX_MEDIA_BYTES, Result, validate_executable, validate_input_file,
};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FIRST_SEGMENT_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(100);
static PLAYBACK_JOB_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct PlaybackProcesses {
    stopping: Arc<AtomicBool>,
}

impl PlaybackProcesses {
    pub(crate) fn new() -> Self {
        Self {
            stopping: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for PlaybackProcesses {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
    }
}

pub(crate) struct HlsPlayback {
    pub directory: PathBuf,
    pub manifest_name: &'static str,
}

pub(crate) fn prepare_compatible_playback(
    media_path: &str,
    ffmpeg_path: &str,
    cache_directory: &Path,
    processes: &PlaybackProcesses,
) -> Result<HlsPlayback> {
    let media = validate_input_file(media_path, "video", MAX_MEDIA_BYTES)?;
    let ffmpeg = validate_executable(ffmpeg_path, "FFmpeg")?;
    let directory = create_playback_directory(cache_directory)?;
    let manifest_name = "stream.m3u8";
    let manifest = directory.join(manifest_name);
    let first_segment = directory.join("segment-000000.ts");
    let stdout = create_process_file(&directory.join("transcode.stdout"))?;
    let stderr = create_process_file(&directory.join("transcode.stderr"))?;
    let arguments = hls_arguments(&media, &manifest);
    let mut child = Command::new(ffmpeg)
        .args(arguments)
        .current_dir(&directory)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            InferenceError(format!(
                "Could not start local compatibility streaming: {error}"
            ))
        })?;

    let started = Instant::now();
    loop {
        if manifest.is_file() && first_segment.is_file() {
            break;
        }
        if let Some(status) = child.try_wait().map_err(|error| {
            InferenceError(format!(
                "Could not monitor compatibility streaming: {error}"
            ))
        })? {
            return Err(InferenceError(format!(
                "Compatibility streaming stopped before its first segment with {status}"
            )));
        }
        if started.elapsed() >= FIRST_SEGMENT_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(InferenceError(
                "Preparing the first compatible playback segment timed out".to_owned(),
            ));
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    }

    let stopping = Arc::clone(&processes.stopping);
    thread::Builder::new()
        .name("playdub-playback-transcode".to_owned())
        .spawn(move || monitor_transcoder(child, &stopping))
        .map_err(|error| {
            InferenceError(format!(
                "Could not monitor compatibility streaming: {error}"
            ))
        })?;
    Ok(HlsPlayback {
        directory,
        manifest_name,
    })
}

fn monitor_transcoder(mut child: std::process::Child, stopping: &AtomicBool) {
    loop {
        if stopping.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => thread::sleep(PROCESS_POLL_INTERVAL),
        }
    }
}

fn create_playback_directory(cache_directory: &Path) -> Result<PathBuf> {
    let root = cache_directory.join("playback-streams");
    fs::create_dir_all(&root)
        .map_err(|error| InferenceError(format!("Could not create playback cache: {error}")))?;
    for _ in 0..100 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let counter = PLAYBACK_JOB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!("stream-{}-{now}-{counter}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(InferenceError(format!(
                    "Could not create playback stream: {error}"
                )));
            }
        }
    }
    Err(InferenceError(
        "Could not allocate a playback stream".to_owned(),
    ))
}

fn create_process_file(path: &Path) -> Result<std::fs::File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| InferenceError(format!("Could not create playback process log: {error}")))
}

fn hls_arguments(media: &Path, manifest: &Path) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("-nostdin"),
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-y"),
        OsString::from("-i"),
        media.as_os_str().to_owned(),
        OsString::from("-map"),
        OsString::from("0:v:0"),
        OsString::from("-map"),
        OsString::from("0:a:0?"),
        OsString::from("-sn"),
        OsString::from("-dn"),
        OsString::from("-c:v"),
        OsString::from(playback_video_encoder()),
    ];

    #[cfg(target_os = "macos")]
    arguments.extend([
        OsString::from("-allow_sw"),
        OsString::from("1"),
        OsString::from("-b:v"),
        OsString::from("5M"),
    ]);
    #[cfg(target_os = "windows")]
    arguments.extend([
        OsString::from("-rate_control"),
        OsString::from("quality"),
        OsString::from("-quality"),
        OsString::from("75"),
    ]);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    arguments.extend([
        OsString::from("-preset"),
        OsString::from("veryfast"),
        OsString::from("-crf"),
        OsString::from("23"),
    ]);

    arguments.extend([
        OsString::from("-pix_fmt"),
        OsString::from("yuv420p"),
        OsString::from("-force_key_frames"),
        OsString::from("expr:gte(t,n_forced*6)"),
        OsString::from("-c:a"),
        OsString::from("aac"),
        OsString::from("-b:a"),
        OsString::from("192k"),
        OsString::from("-f"),
        OsString::from("hls"),
        OsString::from("-hls_time"),
        OsString::from("6"),
        OsString::from("-hls_playlist_type"),
        OsString::from("event"),
        OsString::from("-hls_flags"),
        OsString::from("independent_segments+temp_file"),
        OsString::from("-hls_segment_filename"),
        OsString::from("segment-%06d.ts"),
        manifest.as_os_str().to_owned(),
    ]);
    arguments
}

const fn playback_video_encoder() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "h264_videotoolbox"
    }
    #[cfg(target_os = "windows")]
    {
        "h264_mf"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "libx264"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_output_is_segmented_h264_aac_hls() {
        let arguments = hls_arguments(Path::new("/input.mkv"), Path::new("/stream/stream.m3u8"));
        let values = arguments
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>();
        assert!(
            values
                .windows(2)
                .any(|pair| pair == ["-c:v", playback_video_encoder()])
        );
        assert!(values.windows(2).any(|pair| pair == ["-c:a", "aac"]));
        assert!(
            values
                .windows(2)
                .any(|pair| pair == ["-hls_playlist_type", "event"])
        );
        assert_eq!(
            values.last().map(AsRef::as_ref),
            Some("/stream/stream.m3u8")
        );
    }
}
