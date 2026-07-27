import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export interface SubtitleSegment {
  id: number;
  startMs: number;
  endMs: number;
  sourceText: string;
  translatedText?: string;
}

export interface AuthorizedMedia {
  path: string;
  playbackUrl: string;
}

export interface AnalysisRequest {
  mediaPath: string;
  ffmpegPath: string;
  whisperCliPath: string;
  whisperModelPath: string;
  llamaCliPath?: string;
  llamaModelPath?: string;
  argosTranslatePath?: string;
  sourceLanguage: string;
  targetLanguage: string;
  generationId: number;
  durationMs?: number;
}

export interface AnalysisResult {
  generationId: number;
  durationMs: number;
  segments: SubtitleSegment[];
}

export interface ExportSubtitlesRequest {
  mediaPath: string;
  format: "srt" | "vtt";
  targetLanguage: string;
  segments: SubtitleSegment[];
}

export interface AnalysisSegments {
  generationId: number;
  segments: SubtitleSegment[];
}

const VIDEO_FILTERS = [
  {
    name: "Video",
    extensions: ["mp4", "mkv", "mov", "webm", "avi", "m4v"],
  },
];

export function isDesktopRuntime(): boolean {
  return isTauri();
}

export async function chooseVideo(): Promise<string | null> {
  const selection = await open({
    title: "Open a video",
    multiple: false,
    directory: false,
    filters: VIDEO_FILTERS,
  });
  return typeof selection === "string" ? selection : null;
}

export async function authorizeMediaPath(mediaPath: string): Promise<AuthorizedMedia> {
  return invoke<AuthorizedMedia>("authorize_media_path", { mediaPath });
}

export async function analyzeVideo(request: AnalysisRequest): Promise<AnalysisResult> {
  return invoke<AnalysisResult>("analyze_video", { request });
}

export async function prepareCompatiblePlayback(
  mediaPath: string,
  ffmpegPath?: string,
): Promise<string> {
  return invoke<string>("prepare_compatible_playback", { mediaPath, ffmpegPath });
}

export async function exportSubtitles(request: ExportSubtitlesRequest): Promise<string> {
  return invoke<string>("export_subtitles", { request });
}

export async function subscribeAnalysisSegments(
  callback: (result: AnalysisSegments) => void,
): Promise<UnlistenFn> {
  return listen<AnalysisSegments>("analysis-segments", (event) => callback(event.payload));
}
