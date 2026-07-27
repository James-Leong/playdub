use crate::subtitles::{
    Cue, SubtitleSegment, align_translation, parse_whisper_srt, render_srt, render_vtt,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{OsStr, OsString},
    fmt::{self, Write as _},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) const MAX_MEDIA_BYTES: u64 = 100 * 1024 * 1024 * 1024;
const MAX_MODEL_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_AUDIO_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_SUBTITLE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LOG_BYTES: u64 = 1024 * 1024;
const MAX_TRANSLATION_PROMPT_BYTES: usize = 24 * 1024;
const TRANSLATION_BATCH_SIZE: usize = 24;
const FIRST_ANALYSIS_WINDOW_MS: u64 = 60 * 1_000;
const ANALYSIS_WINDOW_MS: u64 = 5 * 60 * 1_000;
const EXTRACT_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
const INFERENCE_TIMEOUT: Duration = Duration::from_secs(6 * 60 * 60);
const TRANSLATION_BATCH_TIMEOUT: Duration = Duration::from_secs(30 * 60);
static JOB_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnalyzeVideoRequest {
    pub media_path: String,
    pub ffmpeg_path: String,
    pub whisper_cli_path: String,
    pub whisper_model_path: String,
    pub llama_cli_path: Option<String>,
    pub llama_model_path: Option<String>,
    pub argos_translate_path: Option<String>,
    pub source_language: String,
    pub target_language: String,
    pub generation_id: u64,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnalysisResult {
    pub generation_id: u64,
    pub duration_ms: u64,
    pub source_language: String,
    pub target_language: String,
    pub segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SubtitleFormat {
    Srt,
    Vtt,
}

impl SubtitleFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportSubtitlesRequest {
    pub media_path: String,
    pub output_path: Option<String>,
    pub format: SubtitleFormat,
    pub target_language: String,
    pub segments: Vec<SubtitleSegment>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug)]
pub(crate) struct InferenceError(pub(crate) String);

impl fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for InferenceError {}

pub(crate) type Result<T> = std::result::Result<T, InferenceError>;

pub(crate) fn analyze_video<F, G>(
    request: AnalyzeVideoRequest,
    cache_directory: &Path,
    progress: F,
    partial_result: G,
) -> Result<AnalysisResult>
where
    F: Fn(&'static str, f64, &'static str),
    G: Fn(&[SubtitleSegment]),
{
    validate_language(&request.source_language, "source")?;
    validate_language(&request.target_language, "target")?;

    let media = validate_input_file(&request.media_path, "video", MAX_MEDIA_BYTES)?;
    let ffmpeg = validate_executable(&request.ffmpeg_path, "FFmpeg")?;
    let whisper = validate_executable(&request.whisper_cli_path, "whisper-cli")?;
    let whisper_model = validate_input_file(
        &request.whisper_model_path,
        "Whisper model",
        MAX_MODEL_BYTES,
    )?;
    let job = JobDirectory::create(cache_directory)?;
    let duration_ms = request
        .duration_ms
        .filter(|duration| *duration > 0)
        .ok_or_else(|| {
            InferenceError(
                "The video duration is unavailable; reopen the video and try again".to_owned(),
            )
        })?;
    let mut offset_ms = 0_u64;
    let mut window_index = 0_u64;
    let mut next_segment_id = 1_u32;
    let mut segments = Vec::new();

    while offset_ms < duration_ms {
        let requested_window_ms = if window_index == 0 {
            FIRST_ANALYSIS_WINDOW_MS
        } else {
            ANALYSIS_WINDOW_MS
        };
        let window_ms = requested_window_ms.min(duration_ms - offset_ms);
        let window_directory = job.path().join(format!("window-{window_index:06}"));
        fs::create_dir(&window_directory).map_err(|error| {
            InferenceError(format!("Could not create an analysis window: {error}"))
        })?;
        let audio = window_directory.join("audio.wav");
        let fraction = analysis_fraction(offset_ms, duration_ms);

        progress("extracting", fraction, "Preparing the next local segment");
        extract_audio_window(
            &ffmpeg,
            &media,
            &audio,
            &window_directory,
            offset_ms,
            window_ms,
        )?;
        validate_generated_file(&audio, "extracted audio", MAX_AUDIO_BYTES)?;
        if wav_duration_ms(&audio)? == 0 {
            offset_ms = offset_ms.saturating_add(window_ms);
            window_index = window_index.saturating_add(1);
            continue;
        }

        progress(
            "recognizing",
            fraction,
            "Recognizing the next local segment",
        );
        let source_output = window_directory.join("source");
        run_whisper(
            &whisper,
            &whisper_model,
            &audio,
            &source_output,
            &request.source_language,
            false,
            &window_directory,
            "whisper-source",
        )?;
        let source_cues = read_srt(&source_output.with_extension("srt"))?;
        if !source_cues.is_empty() {
            progress("translating", fraction, "Translating new captions locally");
            let translated = translate(
                &request,
                &whisper,
                &whisper_model,
                &audio,
                &source_cues,
                &window_directory,
            )?;
            let window_segments =
                offset_segments(&source_cues, translated, offset_ms, &mut next_segment_id);
            partial_result(&window_segments);
            segments.extend(window_segments);
        }

        offset_ms = offset_ms.saturating_add(window_ms);
        window_index = window_index.saturating_add(1);
    }

    if segments.is_empty() {
        return Err(InferenceError(
            "No speech was recognized in this video".to_owned(),
        ));
    }
    progress("complete", 1.0, "Local subtitles are ready");
    Ok(AnalysisResult {
        generation_id: request.generation_id,
        duration_ms,
        source_language: request.source_language,
        target_language: request.target_language,
        segments,
    })
}

fn analysis_fraction(offset_ms: u64, duration_ms: u64) -> f64 {
    let permille = offset_ms
        .min(duration_ms)
        .saturating_mul(1_000)
        .checked_div(duration_ms)
        .unwrap_or_default();
    f64::from(u32::try_from(permille).expect("analysis progress is at most 1000")) / 1_000.0
}

fn offset_segments(
    source_cues: &[Cue],
    translated: Vec<Option<String>>,
    offset_ms: u64,
    next_segment_id: &mut u32,
) -> Vec<SubtitleSegment> {
    source_cues
        .iter()
        .zip(translated)
        .map(|(cue, translated_text)| {
            let segment = SubtitleSegment {
                id: *next_segment_id,
                start_ms: cue.start_ms.saturating_add(offset_ms),
                end_ms: cue.end_ms.saturating_add(offset_ms),
                source_text: cue.text.clone(),
                translated_text,
            };
            *next_segment_id = next_segment_id.saturating_add(1);
            segment
        })
        .collect()
}

fn wav_duration_ms(path: &Path) -> Result<u64> {
    const HEADER_LIMIT: u64 = 1024 * 1024;
    let file = File::open(path)
        .map_err(|error| InferenceError(format!("Could not inspect extracted audio: {error}")))?;
    let mut header = Vec::new();
    file.take(HEADER_LIMIT)
        .read_to_end(&mut header)
        .map_err(|error| {
            InferenceError(format!("Could not read extracted audio header: {error}"))
        })?;
    if header.len() < 12 || !matches!(&header[0..4], b"RIFF" | b"RF64") || &header[8..12] != b"WAVE"
    {
        return Err(InferenceError(
            "FFmpeg output is not a valid PCM WAV file".to_owned(),
        ));
    }

    let mut offset = 12_usize;
    let mut byte_rate = None;
    let mut rf64_data_size = None;
    while offset.checked_add(8).is_some_and(|end| end <= header.len()) {
        let identifier = &header[offset..offset + 4];
        let size = read_u32_le(&header[offset + 4..offset + 8])? as usize;
        let data_offset = offset + 8;
        if identifier == b"ds64" && size >= 16 && data_offset + 16 <= header.len() {
            rf64_data_size = Some(read_u64_le(&header[data_offset + 8..data_offset + 16])?);
        } else if identifier == b"fmt " && size >= 12 && data_offset + 12 <= header.len() {
            byte_rate = Some(u64::from(read_u32_le(
                &header[data_offset + 8..data_offset + 12],
            )?));
        } else if identifier == b"data" {
            let data_bytes = if size == u32::MAX as usize {
                rf64_data_size.ok_or_else(|| {
                    InferenceError("RF64 audio is missing its data size".to_owned())
                })?
            } else {
                size as u64
            };
            let rate = byte_rate
                .filter(|rate| *rate > 0)
                .ok_or_else(|| InferenceError("WAV audio is missing its byte rate".to_owned()))?;
            return data_bytes
                .checked_mul(1_000)
                .map(|bytes| bytes / rate)
                .ok_or_else(|| InferenceError("WAV duration is too large".to_owned()));
        }

        let padded_size = size
            .checked_add(size % 2)
            .ok_or_else(|| InferenceError("Invalid WAV chunk size".to_owned()))?;
        offset = data_offset
            .checked_add(padded_size)
            .ok_or_else(|| InferenceError("Invalid WAV chunk offset".to_owned()))?;
    }
    Err(InferenceError(
        "Could not find WAV audio duration metadata".to_owned(),
    ))
}

fn read_u32_le(bytes: &[u8]) -> Result<u32> {
    let value = <[u8; 4]>::try_from(bytes)
        .map_err(|_| InferenceError("Invalid WAV integer field".to_owned()))?;
    Ok(u32::from_le_bytes(value))
}

fn read_u64_le(bytes: &[u8]) -> Result<u64> {
    let value = <[u8; 8]>::try_from(bytes)
        .map_err(|_| InferenceError("Invalid WAV integer field".to_owned()))?;
    Ok(u64::from_le_bytes(value))
}

pub(crate) fn validate_media_path(value: &str) -> Result<PathBuf> {
    validate_input_file(value, "video", MAX_MEDIA_BYTES)
}

fn translate(
    request: &AnalyzeVideoRequest,
    whisper: &Path,
    whisper_model: &Path,
    audio: &Path,
    source_cues: &[Cue],
    job_directory: &Path,
) -> Result<Vec<Option<String>>> {
    if language_matches(&request.source_language, &request.target_language)
        && !request.source_language.eq_ignore_ascii_case("auto")
    {
        return Ok(source_cues
            .iter()
            .map(|cue| Some(cue.text.clone()))
            .collect());
    }

    if let Some(path) = request
        .argos_translate_path
        .as_deref()
        .filter(|path| !path.is_empty() && !is_english(&request.target_language))
    {
        let argos = validate_executable(path, "argos-translate")?;
        return translate_with_argos(
            &argos,
            source_cues,
            &request.source_language,
            &request.target_language,
            job_directory,
        );
    }

    if is_english(&request.target_language) {
        let translated_output = job_directory.join("english");
        run_whisper(
            whisper,
            whisper_model,
            audio,
            &translated_output,
            &request.source_language,
            true,
            job_directory,
            "whisper-translation",
        )?;
        let translated_cues = read_srt(&translated_output.with_extension("srt"))?;
        return Ok(align_translation(source_cues, &translated_cues));
    }

    let llama_path = request.llama_cli_path.as_deref().ok_or_else(|| {
        InferenceError(format!(
            "Translation to '{}' requires a local llama-cli executable and GGUF model",
            request.target_language
        ))
    })?;
    let model_path = request.llama_model_path.as_deref().ok_or_else(|| {
        InferenceError(format!(
            "Translation to '{}' requires a local llama-cli executable and GGUF model",
            request.target_language
        ))
    })?;
    let llama = validate_executable(llama_path, "llama-cli")?;
    let model = validate_input_file(model_path, "llama translation model", MAX_MODEL_BYTES)?;
    translate_with_llama(
        &llama,
        &model,
        source_cues,
        &request.source_language,
        &request.target_language,
        job_directory,
    )
}

fn translate_with_argos(
    executable: &Path,
    source_cues: &[Cue],
    source_language: &str,
    target_language: &str,
    job_directory: &Path,
) -> Result<Vec<Option<String>>> {
    let source = argos_language_code(source_language)?;
    let target = argos_language_code(target_language)?;
    let input_path = job_directory.join("argos.batch.input");
    let output_path = job_directory.join("argos.batch.output");
    let error_path = job_directory.join("argos.batch.stderr");
    write_argos_batch(&input_path, source_cues)?;
    let arguments = [
        OsString::from("--playdub-batch"),
        OsString::from(source),
        OsString::from(target),
    ];
    run_command_with_input(
        executable,
        &arguments,
        job_directory,
        &input_path,
        &output_path,
        &error_path,
        TRANSLATION_BATCH_TIMEOUT,
    )?;
    let output = read_limited(&output_path, MAX_SUBTITLE_BYTES, "Argos batch response")?;
    parse_argos_batch(&output, source_cues.len())
}

fn write_argos_batch(path: &Path, source_cues: &[Cue]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| InferenceError(format!("Could not create Argos batch input: {error}")))?;
    for (index, cue) in source_cues.iter().enumerate() {
        let text = cue.text.trim();
        if text.len() > MAX_TRANSLATION_PROMPT_BYTES {
            return Err(InferenceError(format!(
                "Subtitle cue {} exceeds the local translation input limit",
                index + 1
            )));
        }
        let length = u32::try_from(text.len())
            .map_err(|_| InferenceError("Argos batch input is too large".to_owned()))?;
        file.write_all(&length.to_be_bytes())
            .and_then(|()| file.write_all(text.as_bytes()))
            .map_err(|error| {
                InferenceError(format!("Could not write Argos batch input: {error}"))
            })?;
    }
    Ok(())
}

fn parse_argos_batch(bytes: &[u8], expected: usize) -> Result<Vec<Option<String>>> {
    let mut offset = 0_usize;
    let mut translated = Vec::with_capacity(expected);
    while offset < bytes.len() {
        let header_end = offset
            .checked_add(4)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| InferenceError("Argos returned a truncated batch header".to_owned()))?;
        let length = u32::from_be_bytes(
            bytes[offset..header_end]
                .try_into()
                .expect("batch header is exactly four bytes"),
        ) as usize;
        let text_end = header_end
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| InferenceError("Argos returned a truncated batch item".to_owned()))?;
        let text = std::str::from_utf8(&bytes[header_end..text_end])
            .map_err(|_| InferenceError("Argos response is not valid UTF-8".to_owned()))?
            .trim();
        translated.push((!text.is_empty()).then(|| text.to_owned()));
        offset = text_end;
    }
    if translated.len() != expected {
        return Err(InferenceError(format!(
            "Argos returned {} translations for {expected} subtitle cues",
            translated.len()
        )));
    }
    Ok(translated)
}

fn argos_language_code(language: &str) -> Result<&'static str> {
    match normalize_language(language).as_str() {
        "zh" | "zh-cn" | "zho" | "chinese" => Ok("zh"),
        "ja" | "jpn" | "japanese" => Ok("ja"),
        "ko" | "kor" | "korean" => Ok("ko"),
        "en" | "eng" | "english" => Ok("en"),
        _ => Err(InferenceError(format!(
            "The default Argos translation path supports only Chinese, Japanese, Korean, and English; got '{language}'"
        ))),
    }
}

fn language_matches(left: &str, right: &str) -> bool {
    normalize_language(left) == normalize_language(right)
}

fn is_english(language: &str) -> bool {
    matches!(
        normalize_language(language).as_str(),
        "en" | "eng" | "english"
    )
}

fn normalize_language(language: &str) -> String {
    language.trim().to_ascii_lowercase().replace('_', "-")
}

fn validate_language(language: &str, kind: &str) -> Result<()> {
    let value = language.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(InferenceError(format!(
            "The {kind} language must be a language name or code using letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

fn extract_audio_window(
    ffmpeg: &Path,
    media: &Path,
    audio: &Path,
    job_directory: &Path,
    offset_ms: u64,
    duration_ms: u64,
) -> Result<()> {
    let arguments = [
        OsString::from("-nostdin"),
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-y"),
        OsString::from("-ss"),
        OsString::from(ffmpeg_timestamp(offset_ms)),
        OsString::from("-i"),
        media.as_os_str().to_owned(),
        OsString::from("-t"),
        OsString::from(ffmpeg_timestamp(duration_ms)),
        OsString::from("-map"),
        OsString::from("0:a:0"),
        OsString::from("-vn"),
        OsString::from("-ac"),
        OsString::from("1"),
        OsString::from("-ar"),
        OsString::from("16000"),
        OsString::from("-c:a"),
        OsString::from("pcm_s16le"),
        audio.as_os_str().to_owned(),
    ];
    run_command(ffmpeg, &arguments, job_directory, "ffmpeg", EXTRACT_TIMEOUT)
}

fn ffmpeg_timestamp(milliseconds: u64) -> String {
    format!("{}.{:03}", milliseconds / 1_000, milliseconds % 1_000)
}

#[allow(clippy::too_many_arguments)]
fn run_whisper(
    executable: &Path,
    model: &Path,
    audio: &Path,
    output_prefix: &Path,
    source_language: &str,
    translate_to_english: bool,
    job_directory: &Path,
    stage: &str,
) -> Result<()> {
    let mut arguments = vec![
        OsString::from("-m"),
        model.as_os_str().to_owned(),
        OsString::from("-f"),
        audio.as_os_str().to_owned(),
        OsString::from("-osrt"),
        OsString::from("-of"),
        output_prefix.as_os_str().to_owned(),
        OsString::from("-l"),
        OsString::from(source_language),
        OsString::from("-np"),
    ];
    if translate_to_english {
        arguments.push(OsString::from("-tr"));
    }
    run_command(
        executable,
        &arguments,
        job_directory,
        stage,
        INFERENCE_TIMEOUT,
    )?;
    validate_generated_file(
        &output_prefix.with_extension("srt"),
        "Whisper subtitle output",
        MAX_SUBTITLE_BYTES,
    )?;
    Ok(())
}

fn read_srt(path: &Path) -> Result<Vec<Cue>> {
    let contents = read_limited(path, MAX_SUBTITLE_BYTES, "Whisper subtitle output")?;
    let text = String::from_utf8(contents)
        .map_err(|_| InferenceError("Whisper subtitle output is not valid UTF-8".to_owned()))?;
    parse_whisper_srt(&text).map_err(InferenceError)
}

fn translate_with_llama(
    executable: &Path,
    model: &Path,
    source_cues: &[Cue],
    source_language: &str,
    target_language: &str,
    job_directory: &Path,
) -> Result<Vec<Option<String>>> {
    let mut translated = vec![None; source_cues.len()];

    for (batch_index, batch) in source_cues.chunks(TRANSLATION_BATCH_SIZE).enumerate() {
        let offset = batch_index * TRANSLATION_BATCH_SIZE;
        let prompt = translation_prompt(batch, offset, source_language, target_language)?;
        let output_path = job_directory.join(format!("llama-{batch_index}.response"));
        let error_path = job_directory.join(format!("llama-{batch_index}.stderr"));
        let arguments = [
            OsString::from("-m"),
            model.as_os_str().to_owned(),
            OsString::from("-p"),
            OsString::from(prompt),
            OsString::from("-n"),
            OsString::from((batch.len() * 96).clamp(128, 4_096).to_string()),
            OsString::from("--temp"),
            OsString::from("0"),
            OsString::from("--offline"),
            OsString::from("--no-display-prompt"),
            OsString::from("--no-show-timings"),
            OsString::from("--single-turn"),
            OsString::from("--reasoning-format"),
            OsString::from("none"),
        ];
        run_command_with_logs(
            executable,
            &arguments,
            job_directory,
            &output_path,
            &error_path,
            TRANSLATION_BATCH_TIMEOUT,
        )?;
        let bytes = read_limited(&output_path, MAX_SUBTITLE_BYTES, "llama-cli response")?;
        let response = String::from_utf8(bytes)
            .map_err(|_| InferenceError("llama-cli response is not valid UTF-8".to_owned()))?;
        let parsed = parse_llama_response(&response, offset, batch.len())?;
        for (relative_index, text) in parsed.into_iter().enumerate() {
            translated[offset + relative_index] = Some(text);
        }
    }

    Ok(translated)
}

fn translation_prompt(
    batch: &[Cue],
    offset: usize,
    source_language: &str,
    target_language: &str,
) -> Result<String> {
    let mut prompt = format!(
        "Translate subtitle text from {source_language} to {target_language}. \
         Preserve meaning, names, tone, and line breaks. Do not explain. \
         Return exactly one line per input, in the same order, formatted as \
         PLAYDUB_<six-digit-number><TAB><translation>. Inputs:\n"
    );
    for (index, cue) in batch.iter().enumerate() {
        let flattened = cue.text.replace(['\r', '\n', '\t'], " ");
        let id = offset + index + 1;
        writeln!(prompt, "PLAYDUB_{id:06}\t{flattened}").expect("writing to a String cannot fail");
    }
    if prompt.len() > MAX_TRANSLATION_PROMPT_BYTES {
        return Err(InferenceError(format!(
            "A translation batch exceeds the {MAX_TRANSLATION_PROMPT_BYTES}-byte prompt limit"
        )));
    }
    Ok(prompt)
}

fn parse_llama_response(response: &str, offset: usize, expected: usize) -> Result<Vec<String>> {
    let mut values = vec![None; expected];
    for line in response.lines() {
        let trimmed = line.trim();
        let Some(remainder) = trimmed.strip_prefix("PLAYDUB_") else {
            continue;
        };
        let Some((raw_id, text)) = remainder.split_once('\t') else {
            continue;
        };
        if raw_id.len() != 6 || !raw_id.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let id = raw_id.parse::<usize>().map_err(|_| {
            InferenceError("llama-cli returned an invalid cue identifier".to_owned())
        })?;
        let Some(relative) = id.checked_sub(offset + 1) else {
            continue;
        };
        if relative >= expected {
            continue;
        }
        let cleaned = text.trim();
        if cleaned.is_empty() || cleaned.len() > 64 * 1024 {
            return Err(InferenceError(format!(
                "llama-cli returned invalid text for cue {id}"
            )));
        }
        if values[relative].replace(cleaned.to_owned()).is_some() {
            return Err(InferenceError(format!(
                "llama-cli returned cue {id} more than once"
            )));
        }
    }

    if values.iter().any(Option::is_none) {
        return Err(InferenceError(
            "llama-cli did not return exactly one tagged translation for every subtitle cue"
                .to_owned(),
        ));
    }
    Ok(values
        .into_iter()
        .map(|value| value.expect("all values were checked"))
        .collect())
}

pub(crate) fn export_subtitles(request: &ExportSubtitlesRequest) -> Result<PathBuf> {
    let media = validate_input_file(&request.media_path, "video", MAX_MEDIA_BYTES)?;
    validate_language(&request.target_language, "target")?;
    if request.segments.len() > 100_000 {
        return Err(InferenceError(
            "Subtitle export exceeds the 100000 cue limit".to_owned(),
        ));
    }

    let output = match request.output_path.as_deref() {
        Some(path) => validate_output_path(path, request.format)?,
        None => default_export_path(&media, &request.target_language, request.format)?,
    };
    let rendered = match request.format {
        SubtitleFormat::Srt => render_srt(&request.segments),
        SubtitleFormat::Vtt => render_vtt(&request.segments),
    }
    .map_err(InferenceError)?;
    if rendered.len() as u64 > MAX_SUBTITLE_BYTES {
        return Err(InferenceError(
            "Rendered subtitles exceed the 64 MiB output limit".to_owned(),
        ));
    }

    write_export(&output, rendered.as_bytes(), request.overwrite)?;
    output.canonicalize().map_err(|error| {
        InferenceError(format!("Could not resolve exported subtitle path: {error}"))
    })
}

fn default_export_path(
    media: &Path,
    target_language: &str,
    format: SubtitleFormat,
) -> Result<PathBuf> {
    let stem = media
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| InferenceError("The video filename is not valid UTF-8".to_owned()))?;
    let language = normalize_language(target_language);
    Ok(media.with_file_name(format!("{stem}.{language}.playdub.{}", format.extension())))
}

fn validate_output_path(value: &str, format: SubtitleFormat) -> Result<PathBuf> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(InferenceError(
            "Subtitle output path must be absolute".to_owned(),
        ));
    }
    if path.extension().and_then(OsStr::to_str) != Some(format.extension()) {
        return Err(InferenceError(format!(
            "Subtitle output must use the .{} extension",
            format.extension()
        )));
    }
    let parent = path
        .parent()
        .ok_or_else(|| InferenceError("Subtitle output needs a parent directory".to_owned()))?
        .canonicalize()
        .map_err(|error| {
            InferenceError(format!(
                "Could not resolve subtitle output directory: {error}"
            ))
        })?;
    if !parent.is_dir() {
        return Err(InferenceError(
            "Subtitle output parent is not a directory".to_owned(),
        ));
    }
    let name = path
        .file_name()
        .ok_or_else(|| InferenceError("Subtitle output needs a filename".to_owned()))?;
    Ok(parent.join(name))
}

fn write_export(path: &Path, contents: &[u8], overwrite: bool) -> Result<()> {
    if let Ok(metadata) = path.symlink_metadata()
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(InferenceError(
            "Existing subtitle output is not a regular file".to_owned(),
        ));
    }
    let mut options = OpenOptions::new();
    options.write(true);
    if overwrite {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }
    let mut file = options.open(path).map_err(|error| {
        let hint = if path.exists() && !overwrite {
            " (set overwrite to true to replace it)"
        } else {
            ""
        };
        InferenceError(format!("Could not create subtitle output{hint}: {error}"))
    })?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| InferenceError(format!("Could not write subtitle output: {error}")))
}

pub(crate) fn validate_input_file(value: &str, kind: &str, maximum_size: u64) -> Result<PathBuf> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(InferenceError(format!("{kind} path must be absolute")));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| InferenceError(format!("Could not resolve {kind} path: {error}")))?;
    let metadata = canonical
        .metadata()
        .map_err(|error| InferenceError(format!("Could not inspect {kind}: {error}")))?;
    if !metadata.is_file() {
        return Err(InferenceError(format!("{kind} path is not a regular file")));
    }
    if metadata.len() == 0 || metadata.len() > maximum_size {
        return Err(InferenceError(format!(
            "{kind} size must be between 1 byte and {maximum_size} bytes"
        )));
    }
    Ok(canonical)
}

pub(crate) fn validate_executable(value: &str, kind: &str) -> Result<PathBuf> {
    let executable = validate_input_file(value, kind, 1024 * 1024 * 1024)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if executable
            .metadata()
            .map_err(|error| InferenceError(format!("Could not inspect {kind}: {error}")))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(InferenceError(format!("{kind} is not executable")));
        }
    }
    Ok(executable)
}

pub(crate) fn validate_generated_file(path: &Path, kind: &str, maximum_size: u64) -> Result<()> {
    let metadata = path
        .symlink_metadata()
        .map_err(|error| InferenceError(format!("{kind} was not created: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(InferenceError(format!(
            "{kind} is not a regular generated file"
        )));
    }
    if metadata.len() == 0 || metadata.len() > maximum_size {
        return Err(InferenceError(format!(
            "{kind} size must be between 1 byte and {maximum_size} bytes"
        )));
    }
    Ok(())
}

fn read_limited(path: &Path, limit: u64, kind: &str) -> Result<Vec<u8>> {
    validate_generated_file(path, kind, limit)?;
    let file = File::open(path)
        .map_err(|error| InferenceError(format!("Could not read {kind}: {error}")))?;
    let capacity = usize::try_from(
        path.metadata()
            .map_err(|error| InferenceError(format!("Could not inspect {kind}: {error}")))?
            .len(),
    )
    .map_err(|_| InferenceError(format!("{kind} is too large for this platform")))?;
    let mut output = Vec::with_capacity(capacity);
    file.take(limit + 1)
        .read_to_end(&mut output)
        .map_err(|error| InferenceError(format!("Could not read {kind}: {error}")))?;
    if output.len() as u64 > limit {
        return Err(InferenceError(format!("{kind} exceeds its size limit")));
    }
    Ok(output)
}

pub(crate) fn run_command(
    executable: &Path,
    arguments: &[OsString],
    working_directory: &Path,
    stage: &str,
    timeout: Duration,
) -> Result<()> {
    let output = working_directory.join(format!("{stage}.stdout"));
    let error = working_directory.join(format!("{stage}.stderr"));
    run_command_with_logs(
        executable,
        arguments,
        working_directory,
        &output,
        &error,
        timeout,
    )
}

fn run_command_with_logs(
    executable: &Path,
    arguments: &[OsString],
    working_directory: &Path,
    output_path: &Path,
    error_path: &Path,
    timeout: Duration,
) -> Result<()> {
    let stdout = create_log(output_path)?;
    let stderr = create_log(error_path)?;
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let mut child = command
        .spawn()
        .map_err(|error| InferenceError(format!("Could not start local process: {error}")))?;
    let status = wait_with_timeout(&mut child, timeout)?;
    if !status.success() {
        let diagnostics = read_log_excerpt(error_path)
            .or_else(|| read_log_excerpt(output_path))
            .unwrap_or_else(|| "no diagnostic output".to_owned());
        return Err(InferenceError(format!(
            "Local process failed with {status}: {diagnostics}"
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_command_with_input(
    executable: &Path,
    arguments: &[OsString],
    working_directory: &Path,
    input_path: &Path,
    output_path: &Path,
    error_path: &Path,
    timeout: Duration,
) -> Result<()> {
    let stdin = File::open(input_path)
        .map_err(|error| InferenceError(format!("Could not open local process input: {error}")))?;
    let stdout = create_log(output_path)?;
    let stderr = create_log(error_path)?;
    let mut child = Command::new(executable)
        .args(arguments)
        .current_dir(working_directory)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| InferenceError(format!("Could not start local process: {error}")))?;
    let status = wait_with_timeout(&mut child, timeout)?;
    if !status.success() {
        let diagnostics = read_log_excerpt(error_path)
            .or_else(|| read_log_excerpt(output_path))
            .unwrap_or_else(|| "no diagnostic output".to_owned());
        return Err(InferenceError(format!(
            "Local process failed with {status}: {diagnostics}"
        )));
    }
    Ok(())
}

fn create_log(path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| InferenceError(format!("Could not create local process log: {error}")))
}

fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> Result<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(InferenceError(
                    "Local process exceeded its execution time limit".to_owned(),
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(InferenceError(format!(
                    "Could not monitor local process: {error}"
                )));
            }
        }
    }
}

fn read_log_excerpt(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_LOG_BYTES).read_to_end(&mut bytes).ok()?;
    let value = String::from_utf8_lossy(&bytes);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

struct JobDirectory(PathBuf);

impl JobDirectory {
    fn create(cache_directory: &Path) -> Result<Self> {
        fs::create_dir_all(cache_directory).map_err(|error| {
            InferenceError(format!("Could not create application cache: {error}"))
        })?;
        let root = cache_directory.join("inference-jobs");
        fs::create_dir_all(&root).map_err(|error| {
            InferenceError(format!(
                "Could not create inference cache directory: {error}"
            ))
        })?;
        for _ in 0..100 {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let counter = JOB_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!("job-{}-{now}-{counter}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(InferenceError(format!(
                        "Could not create inference job directory: {error}"
                    )));
                }
            }
        }
        Err(InferenceError(
            "Could not allocate a unique inference job directory".to_owned(),
        ))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for JobDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_exact_llama_translation_tags() {
        let response = "noise\nPLAYDUB_000011\tBonjour\nPLAYDUB_000012\tMonde\n";
        assert_eq!(
            parse_llama_response(response, 10, 2).unwrap(),
            vec!["Bonjour", "Monde"]
        );
    }

    #[test]
    fn rejects_missing_and_duplicate_llama_translations() {
        assert!(parse_llama_response("PLAYDUB_000001\tOne\n", 0, 2).is_err());
        assert!(
            parse_llama_response("PLAYDUB_000001\tOne\nPLAYDUB_000001\tAgain\n", 0, 1).is_err()
        );
    }

    #[test]
    fn prompt_flattens_control_characters_and_uses_stable_ids() {
        let cues = vec![Cue {
            start_ms: 0,
            end_ms: 1_000,
            text: "hello\nworld".to_owned(),
        }];
        let prompt = translation_prompt(&cues, 7, "en", "fr").unwrap();
        assert!(prompt.contains("PLAYDUB_000008\thello world"));
    }

    #[test]
    fn validates_language_values() {
        assert!(validate_language("zh-CN", "target").is_ok());
        assert!(validate_language("English", "target").is_ok());
        assert!(validate_language("../bad", "target").is_err());
    }

    #[test]
    fn limits_argos_translation_to_foundation_languages() {
        assert_eq!(argos_language_code("Japanese").unwrap(), "ja");
        assert_eq!(argos_language_code("zh-CN").unwrap(), "zh");
        assert_eq!(argos_language_code("Korean").unwrap(), "ko");
        assert_eq!(argos_language_code("English").unwrap(), "en");
        assert!(argos_language_code("French").is_err());
    }

    #[test]
    fn parses_length_prefixed_argos_batch_in_order() {
        let mut response = Vec::new();
        for text in ["你好", "世界"] {
            response.extend_from_slice(
                &u32::try_from(text.len())
                    .expect("test text length fits u32")
                    .to_be_bytes(),
            );
            response.extend_from_slice(text.as_bytes());
        }
        assert_eq!(
            parse_argos_batch(&response, 2).unwrap(),
            vec![Some("你好".to_owned()), Some("世界".to_owned())]
        );
        assert!(parse_argos_batch(&response, 3).is_err());
        assert!(parse_argos_batch(&response[..response.len() - 1], 2).is_err());
    }

    #[test]
    fn offsets_incremental_segments_on_the_playback_clock() {
        let cues = vec![Cue {
            start_ms: 250,
            end_ms: 1_250,
            text: "hello".to_owned(),
        }];
        let mut next_id = 7;
        assert_eq!(
            offset_segments(&cues, vec![Some("你好".to_owned())], 60_000, &mut next_id),
            vec![SubtitleSegment {
                id: 7,
                start_ms: 60_250,
                end_ms: 61_250,
                source_text: "hello".to_owned(),
                translated_text: Some("你好".to_owned()),
            }]
        );
        assert_eq!(next_id, 8);
    }

    #[test]
    fn reads_pcm_wav_duration_from_its_byte_rate() {
        let directory = test_directory();
        let wav = directory.join("audio.wav");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&64_036_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&16_000_u32.to_le_bytes());
        bytes.extend_from_slice(&32_000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&64_000_u32.to_le_bytes());
        bytes.resize(64_044, 0);
        fs::write(&wav, bytes).unwrap();
        assert_eq!(wav_duration_ms(&wav).unwrap(), 2_000);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn exports_without_overwriting_by_default() {
        let directory = test_directory();
        let media = directory.join("clip.mp4");
        fs::write(&media, b"synthetic").unwrap();
        let output = directory.join("clip.en.srt");
        let request = ExportSubtitlesRequest {
            media_path: media.to_string_lossy().into_owned(),
            output_path: Some(output.to_string_lossy().into_owned()),
            format: SubtitleFormat::Srt,
            target_language: "en".to_owned(),
            segments: vec![SubtitleSegment {
                id: 1,
                start_ms: 0,
                end_ms: 1_000,
                source_text: "hello".to_owned(),
                translated_text: Some("Hello".to_owned()),
            }],
            overwrite: false,
        };
        assert_eq!(
            export_subtitles(&request).unwrap(),
            output.canonicalize().unwrap()
        );
        assert!(export_subtitles(&request).is_err());
        assert_eq!(
            fs::read_to_string(&output).unwrap(),
            "1\n00:00:00,000 --> 00:00:01,000\nHello\n\n"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    fn test_directory() -> PathBuf {
        let counter = JOB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("playdub-test-{}-{counter}", std::process::id()));
        fs::create_dir(&path).unwrap();
        path
    }
}
