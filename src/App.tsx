import {
  type ChangeEvent,
  type DragEvent,
  type ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  analyzeVideo,
  authorizeMediaPath,
  chooseExecutable,
  chooseModel,
  chooseVideo,
  exportSubtitles,
  isDesktopRuntime,
  mediaSourceFromPath,
  type SubtitleSegment,
  subscribeAnalysisProgress,
} from "./lib/playdub";
import { getRuntimeInfo, type RuntimeInfo } from "./lib/runtime";
import { DEFAULT_SETTINGS, loadSettings, type PlaydubSettings, saveSettings } from "./lib/settings";
import { findActiveSegment, formatClock, normalizeSegments } from "./lib/subtitles";

type AnalysisState = "idle" | "processing" | "complete" | "error";

interface MediaSelection {
  name: string;
  path: string;
  source: string;
  canAnalyze: boolean;
  objectUrl?: string;
}

interface IconProps {
  children: ReactNode;
  className?: string;
  viewBox?: string;
}

const LANGUAGES = [
  { value: "auto", label: "Auto detect" },
  { value: "en", label: "English" },
  { value: "zh", label: "简体中文" },
  { value: "ja", label: "日本語" },
  { value: "ko", label: "한국어" },
  { value: "es", label: "Español" },
  { value: "fr", label: "Français" },
  { value: "de", label: "Deutsch" },
];

function Icon({ children, className, viewBox = "0 0 24 24" }: IconProps) {
  return (
    <svg
      className={className}
      viewBox={viewBox}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

function PlayIcon() {
  return (
    <Icon>
      <path d="m8 5 11 7-11 7V5Z" fill="currentColor" stroke="none" />
    </Icon>
  );
}

function PauseIcon() {
  return (
    <Icon>
      <path d="M8 5v14M16 5v14" />
    </Icon>
  );
}

function VolumeIcon() {
  return (
    <Icon>
      <path d="M11 5 6.5 9H3v6h3.5L11 19V5ZM15 9a4 4 0 0 1 0 6M18 6a8 8 0 0 1 0 12" />
    </Icon>
  );
}

function FullscreenIcon() {
  return (
    <Icon>
      <path d="M8 3H3v5M16 3h5v5M8 21H3v-5M16 21h5v-5" />
    </Icon>
  );
}

function SettingsIcon() {
  return (
    <Icon>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z" />
    </Icon>
  );
}

function CloseIcon() {
  return (
    <Icon>
      <path d="m6 6 12 12M18 6 6 18" />
    </Icon>
  );
}

function FolderIcon() {
  return (
    <Icon>
      <path d="M3 6.5h6l2 2h10v9.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6.5Z" />
    </Icon>
  );
}

function getFileName(path: string): string {
  return path.split(/[\\/]/).pop() || "Untitled video";
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return typeof error === "string" ? error : "An unexpected local processing error occurred.";
}

export function App() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const playerRef = useRef<HTMLDivElement>(null);
  const browserInputRef = useRef<HTMLInputElement>(null);
  const settingsCloseRef = useRef<HTMLButtonElement>(null);
  const generationRef = useRef(0);

  const [runtime, setRuntime] = useState<RuntimeInfo | null>(null);
  const [media, setMedia] = useState<MediaSelection | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(0.85);
  const [settings, setSettings] = useState<PlaydubSettings>(loadSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [segments, setSegments] = useState<SubtitleSegment[]>([]);
  const [analysisState, setAnalysisState] = useState<AnalysisState>("idle");
  const [analysisStage, setAnalysisStage] = useState("Ready to translate");
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);

  const activeSegment = useMemo(
    () => findActiveSegment(segments, currentTime),
    [currentTime, segments],
  );

  useEffect(() => {
    getRuntimeInfo()
      .then(setRuntime)
      .catch(() => setRuntime(null));
  }, []);

  useEffect(() => {
    saveSettings(settings);
  }, [settings]);

  useEffect(() => {
    if (!settingsOpen) {
      return;
    }

    settingsCloseRef.current?.focus();

    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setSettingsOpen(false);
      }
    }

    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [settingsOpen]);

  useEffect(() => {
    let disposed = false;
    let stopListening: (() => void) | undefined;

    if (isDesktopRuntime()) {
      subscribeAnalysisProgress((event) => {
        if (event.generationId !== generationRef.current) {
          return;
        }
        setProgress(Math.max(0, Math.min(1, event.progress)));
        setAnalysisStage(event.message || event.stage);
      })
        .then((unlisten) => {
          if (disposed) {
            unlisten();
          } else {
            stopListening = unlisten;
          }
        })
        .catch(() => undefined);
    }

    return () => {
      disposed = true;
      stopListening?.();
    };
  }, []);

  useEffect(
    () => () => {
      if (media?.objectUrl) {
        URL.revokeObjectURL(media.objectUrl);
      }
    },
    [media],
  );

  function loadMedia(nextMedia: MediaSelection) {
    generationRef.current += 1;
    setMedia(nextMedia);
    setSegments([]);
    setAnalysisState("idle");
    setAnalysisStage("Ready to translate");
    setProgress(0);
    setCurrentTime(0);
    setDuration(0);
    setError(null);
    setExportMessage(null);
  }

  async function openVideo() {
    setError(null);
    if (!isDesktopRuntime()) {
      browserInputRef.current?.click();
      return;
    }

    try {
      const path = await chooseVideo();
      if (path) {
        const authorizedPath = await authorizeMediaPath(path);
        loadMedia({
          name: getFileName(authorizedPath),
          path: authorizedPath,
          source: mediaSourceFromPath(authorizedPath),
          canAnalyze: true,
        });
      }
    } catch (openError) {
      setError(`Could not open the video picker. ${errorMessage(openError)}`);
    }
  }

  function loadBrowserFile(file: File) {
    if (!file.type.startsWith("video/")) {
      setError("Choose a supported video file.");
      return;
    }
    const objectUrl = URL.createObjectURL(file);
    loadMedia({
      name: file.name,
      path: "",
      source: objectUrl,
      canAnalyze: false,
      objectUrl,
    });
  }

  function handleBrowserFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) {
      const objectUrl = URL.createObjectURL(file);
      loadMedia({
        name: file.name,
        path: "",
        source: objectUrl,
        canAnalyze: false,
        objectUrl,
      });
    }
    event.target.value = "";
  }

  function handleDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    const file = event.dataTransfer.files[0];
    if (file) {
      loadBrowserFile(file);
    }
  }

  async function togglePlayback() {
    const video = videoRef.current;
    if (!video) {
      return;
    }
    if (video.paused) {
      await video.play().catch((playError) => setError(errorMessage(playError)));
    } else {
      video.pause();
    }
  }

  function seek(nextTime: number) {
    const video = videoRef.current;
    if (!video) {
      return;
    }
    video.currentTime = nextTime;
    setCurrentTime(nextTime);
  }

  function updateVolume(nextVolume: number) {
    const video = videoRef.current;
    if (video) {
      video.volume = nextVolume;
    }
    setVolume(nextVolume);
  }

  async function toggleFullscreen() {
    if (!document.fullscreenElement) {
      await playerRef.current?.requestFullscreen();
    } else {
      await document.exitFullscreen();
    }
  }

  async function runAnalysis() {
    if (!media?.canAnalyze || !media.path) {
      setError("Open the video from the desktop file picker before starting local analysis.");
      return;
    }
    if (
      !settings.ffmpegPath.trim() ||
      !settings.whisperCliPath.trim() ||
      !settings.whisperModelPath.trim()
    ) {
      setError(
        "Choose FFmpeg, Whisper CLI, and a Whisper model in Settings before starting transcription.",
      );
      setSettingsOpen(true);
      return;
    }
    if (
      settings.targetLanguage !== "en" &&
      settings.targetLanguage !== settings.sourceLanguage &&
      (!settings.llamaCliPath.trim() || !settings.llamaModelPath.trim())
    ) {
      setError(
        "Translation to this language requires llama.cpp CLI and a local GGUF model in Settings.",
      );
      setSettingsOpen(true);
      return;
    }

    const generationId = generationRef.current + 1;
    generationRef.current = generationId;
    setError(null);
    setExportMessage(null);
    setSegments([]);
    setAnalysisState("processing");
    setAnalysisStage("Preparing local analysis");
    setProgress(0);

    try {
      const result = await analyzeVideo({
        mediaPath: media.path,
        ffmpegPath: settings.ffmpegPath.trim(),
        whisperCliPath: settings.whisperCliPath.trim(),
        whisperModelPath: settings.whisperModelPath.trim(),
        llamaCliPath: settings.llamaCliPath.trim() || undefined,
        llamaModelPath: settings.llamaModelPath.trim() || undefined,
        sourceLanguage: settings.sourceLanguage,
        targetLanguage: settings.targetLanguage,
        generationId,
      });

      if (result.generationId !== generationRef.current) {
        return;
      }
      setSegments(normalizeSegments(result.segments));
      setProgress(1);
      setAnalysisState("complete");
      setAnalysisStage(`${result.segments.length} captions ready`);
    } catch (analysisError) {
      if (generationId !== generationRef.current) {
        return;
      }
      setAnalysisState("error");
      setAnalysisStage("Analysis stopped");
      setError(errorMessage(analysisError));
    }
  }

  async function exportCaptionFile(format: "srt" | "vtt") {
    if (!media?.path || segments.length === 0) {
      return;
    }
    setError(null);
    setExportMessage(null);
    try {
      const path = await exportSubtitles({
        mediaPath: media.path,
        format,
        targetLanguage: settings.targetLanguage,
        segments,
      });
      setExportMessage(`Saved ${format.toUpperCase()} to ${path}`);
    } catch (exportError) {
      setError(`Could not export subtitles. ${errorMessage(exportError)}`);
    }
  }

  const statusLabel =
    analysisState === "processing"
      ? `${analysisStage}${progress > 0 ? ` · ${Math.round(progress * 100)}%` : ""}`
      : analysisStage;

  return (
    <main className="app-shell">
      <header className="topbar">
        <button className="brand" type="button" onClick={openVideo} aria-label="Open a video">
          <span className="brand-mark" aria-hidden="true">
            <PlayIcon />
          </span>
          Playdub
        </button>
        <div className="topbar-actions">
          <span className="privacy-badge">On-device · Private</span>
          <button
            className="icon-button"
            type="button"
            onClick={() => setSettingsOpen(true)}
            aria-label="Open settings"
          >
            <SettingsIcon />
          </button>
        </div>
      </header>

      <input
        ref={browserInputRef}
        className="visually-hidden"
        type="file"
        accept="video/*,.mkv"
        onChange={handleBrowserFile}
        tabIndex={-1}
      />

      {media ? (
        <section className="workspace" aria-label="Video player">
          <section
            className="player-frame"
            ref={playerRef}
            aria-label="Video playback surface"
            onDragOver={(event) => event.preventDefault()}
            onDrop={handleDrop}
          >
            {/* biome-ignore lint/a11y/useMediaCaption: Playdub renders generated synchronized captions. */}
            <video
              ref={videoRef}
              src={media.source}
              onLoadedMetadata={(event) => setDuration(event.currentTarget.duration || 0)}
              onDurationChange={(event) => setDuration(event.currentTarget.duration || 0)}
              onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
              onPlay={() => setIsPlaying(true)}
              onPause={() => setIsPlaying(false)}
              onEnded={() => setIsPlaying(false)}
              onError={() => setError("This video could not be decoded by the system player.")}
              onDoubleClick={toggleFullscreen}
              aria-label={`Playing ${media.name}`}
            />

            <div className="video-scrim" aria-hidden="true" />
            {activeSegment && (
              <div className="captions" aria-live="polite" aria-atomic="true">
                {activeSegment.translatedText && (
                  <p className="caption-translation">{activeSegment.translatedText}</p>
                )}
                {settings.showSourceCaptions && (
                  <p className="caption-source">{activeSegment.sourceText}</p>
                )}
              </div>
            )}

            <div className="player-chrome">
              <div className="now-playing">
                <span className="media-name">{media.name}</span>
                <span className={`analysis-status status-${analysisState}`}>
                  {analysisState === "processing" && <span className="status-spinner" />}
                  {statusLabel}
                </span>
              </div>
              <label className="timeline">
                <span className="visually-hidden">Playback position</span>
                <input
                  type="range"
                  min="0"
                  max={Math.max(duration, 0)}
                  step="0.05"
                  value={Math.min(currentTime, duration || 0)}
                  onChange={(event) => seek(Number(event.target.value))}
                  style={
                    {
                      "--range-progress": `${duration > 0 ? (currentTime / duration) * 100 : 0}%`,
                    } as React.CSSProperties
                  }
                />
              </label>
              <div className="controls">
                <button
                  className="play-button"
                  type="button"
                  onClick={togglePlayback}
                  aria-label={isPlaying ? "Pause" : "Play"}
                >
                  {isPlaying ? <PauseIcon /> : <PlayIcon />}
                </button>
                <span className="timecode">
                  {formatClock(currentTime)}
                  <span>/</span>
                  {formatClock(duration)}
                </span>
                <div className="control-spacer" />
                <label className="volume-control">
                  <VolumeIcon />
                  <span className="visually-hidden">Volume</span>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={volume}
                    onChange={(event) => updateVolume(Number(event.target.value))}
                  />
                </label>
                <button
                  className="icon-button player-icon"
                  type="button"
                  onClick={toggleFullscreen}
                  aria-label="Toggle fullscreen"
                >
                  <FullscreenIcon />
                </button>
              </div>
            </div>
          </section>

          <aside className="task-panel" aria-label="Translation tools">
            <div>
              <span className="panel-eyebrow">Local captions</span>
              <h1>Understand every line.</h1>
              <p>
                Speech recognition and translation run through tools installed on this computer.
              </p>
            </div>

            <div className="language-flow">
              <label>
                <span>Spoken language</span>
                <select
                  value={settings.sourceLanguage}
                  onChange={(event) =>
                    setSettings((current) => ({
                      ...current,
                      sourceLanguage: event.target.value,
                    }))
                  }
                >
                  {LANGUAGES.map((language) => (
                    <option key={language.value} value={language.value}>
                      {language.label}
                    </option>
                  ))}
                </select>
              </label>
              <span className="language-arrow" aria-hidden="true">
                →
              </span>
              <label>
                <span>Translate to</span>
                <select
                  value={settings.targetLanguage}
                  onChange={(event) =>
                    setSettings((current) => ({
                      ...current,
                      targetLanguage: event.target.value,
                    }))
                  }
                >
                  {LANGUAGES.filter((language) => language.value !== "auto").map((language) => (
                    <option key={language.value} value={language.value}>
                      {language.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <button
              className="primary-button"
              type="button"
              onClick={runAnalysis}
              disabled={analysisState === "processing"}
            >
              {analysisState === "processing" ? "Processing locally…" : "Generate subtitles"}
            </button>

            {analysisState === "processing" && (
              <div className="progress-block" role="status" aria-live="polite">
                <div className="progress-track">
                  <span
                    className={progress === 0 ? "indeterminate" : ""}
                    style={progress > 0 ? { width: `${progress * 100}%` } : undefined}
                  />
                </div>
                <span>{statusLabel}</span>
              </div>
            )}

            {segments.length > 0 && (
              <div className="result-card">
                <div>
                  <strong>{segments.length} captions</strong>
                  <span>Synced to playback</span>
                </div>
                <div className="export-actions">
                  <button type="button" onClick={() => exportCaptionFile("srt")}>
                    Export SRT
                  </button>
                  <button type="button" onClick={() => exportCaptionFile("vtt")}>
                    Export VTT
                  </button>
                </div>
              </div>
            )}

            {error && (
              <div className="notice error-notice" role="alert">
                <strong>Something needs attention</strong>
                <span>{error}</span>
                <button type="button" onClick={() => setError(null)}>
                  Dismiss
                </button>
              </div>
            )}
            {exportMessage && (
              <div className="notice success-notice" role="status">
                {exportMessage}
              </div>
            )}

            <div className="panel-footer">
              <button className="text-button" type="button" onClick={openVideo}>
                Open another video
              </button>
              <span>{runtime ? `${runtime.appName} ${runtime.version}` : "Browser preview"}</span>
            </div>
          </aside>
        </section>
      ) : (
        <section
          className="empty-player"
          aria-labelledby="welcome-title"
          onDragOver={(event) => event.preventDefault()}
          onDrop={handleDrop}
        >
          <div className="ambient-glow" aria-hidden="true" />
          <div className="welcome">
            <span className="eyebrow">Watch across languages</span>
            <h1 id="welcome-title">Your videos, understood locally.</h1>
            <p>
              Playdub recognizes speech and translates subtitles on your device. Your media never
              needs to leave your computer.
            </p>
            <button className="open-button" type="button" onClick={openVideo}>
              <FolderIcon />
              Open a video
            </button>
            <small>or drop a video anywhere</small>
          </div>
          {error && (
            <div className="floating-error" role="alert">
              {error}
            </div>
          )}
        </section>
      )}

      {settingsOpen && (
        <div className="modal-backdrop">
          <button
            className="modal-dismiss-layer"
            type="button"
            onClick={() => setSettingsOpen(false)}
            aria-label="Close settings"
          />
          <section
            className="settings-panel"
            role="dialog"
            aria-modal="true"
            aria-labelledby="settings-title"
          >
            <div className="settings-header">
              <div>
                <span className="panel-eyebrow">Local runtime</span>
                <h2 id="settings-title">Settings</h2>
              </div>
              <button
                ref={settingsCloseRef}
                className="icon-button"
                type="button"
                onClick={() => setSettingsOpen(false)}
                aria-label="Close settings"
              >
                <CloseIcon />
              </button>
            </div>
            <p className="settings-intro">
              Playdub never downloads or uploads media. Choose executables and models you have
              reviewed and installed locally.
            </p>

            <SettingsPath
              label="FFmpeg executable"
              value={settings.ffmpegPath}
              placeholder="/path/to/ffmpeg"
              onChange={(value) => setSettings((current) => ({ ...current, ffmpegPath: value }))}
              onBrowse={() => chooseExecutable("Choose FFmpeg")}
            />
            <SettingsPath
              label="Whisper CLI executable"
              value={settings.whisperCliPath}
              placeholder="/path/to/whisper-cli"
              onChange={(value) =>
                setSettings((current) => ({ ...current, whisperCliPath: value }))
              }
              onBrowse={() => chooseExecutable("Choose Whisper CLI")}
            />
            <SettingsPath
              label="Whisper model"
              value={settings.whisperModelPath}
              placeholder="/path/to/model.bin"
              onChange={(value) =>
                setSettings((current) => ({ ...current, whisperModelPath: value }))
              }
              onBrowse={() => chooseModel("Choose Whisper model")}
            />

            <div className="settings-divider">
              <span>Non-English translation</span>
            </div>

            <SettingsPath
              label="llama.cpp CLI executable"
              value={settings.llamaCliPath}
              placeholder="/path/to/llama-cli"
              onChange={(value) => setSettings((current) => ({ ...current, llamaCliPath: value }))}
              onBrowse={() => chooseExecutable("Choose llama.cpp CLI")}
            />
            <SettingsPath
              label="Translation model"
              value={settings.llamaModelPath}
              placeholder="/path/to/model.gguf"
              onChange={(value) =>
                setSettings((current) => ({ ...current, llamaModelPath: value }))
              }
              onBrowse={() => chooseModel("Choose translation model")}
            />

            <label className="toggle-row">
              <span>
                <strong>Show original caption</strong>
                <small>Display source text below the translation.</small>
              </span>
              <input
                type="checkbox"
                checked={settings.showSourceCaptions}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    showSourceCaptions: event.target.checked,
                  }))
                }
              />
            </label>

            <div className="settings-footer">
              <button
                className="text-button"
                type="button"
                onClick={() => setSettings(DEFAULT_SETTINGS)}
              >
                Restore defaults
              </button>
              <button
                className="primary-button compact"
                type="button"
                onClick={() => setSettingsOpen(false)}
              >
                Done
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}

interface SettingsPathProps {
  label: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onBrowse: () => Promise<string | null>;
}

function SettingsPath({ label, value, placeholder, onChange, onBrowse }: SettingsPathProps) {
  async function browse() {
    try {
      const selection = await onBrowse();
      if (selection) {
        onChange(selection);
      }
    } catch {
      // The editable field remains available if the native picker is unavailable.
    }
  }

  return (
    <label className="path-field">
      <span>{label}</span>
      <span className="path-input">
        <input
          type="text"
          value={value}
          placeholder={placeholder}
          spellCheck={false}
          onChange={(event) => onChange(event.target.value)}
        />
        <button type="button" onClick={browse} disabled={!isDesktopRuntime()}>
          Browse
        </button>
      </span>
    </label>
  );
}
