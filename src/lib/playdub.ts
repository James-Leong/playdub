import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export interface SubtitleSegment {
  id: number;
  startMs: number;
  endMs: number;
  sourceText: string;
  translatedText?: string;
}

export interface AnalysisRequest {
  mediaPath: string;
  ffmpegPath: string;
  whisperCliPath: string;
  whisperModelPath: string;
  llamaCliPath?: string;
  llamaModelPath?: string;
  sourceLanguage: string;
  targetLanguage: string;
  generationId: number;
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

export interface AnalysisProgress {
  generationId: number;
  progress: number;
  stage: "extracting" | "recognizing" | "translating" | "complete";
  message?: string;
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

export async function authorizeMediaPath(mediaPath: string): Promise<string> {
  return invoke<string>("authorize_media_path", { mediaPath });
}

export async function chooseExecutable(title: string): Promise<string | null> {
  const selection = await open({
    title,
    multiple: false,
    directory: false,
  });
  return typeof selection === "string" ? selection : null;
}

export async function chooseModel(title: string): Promise<string | null> {
  const selection = await open({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: "Model", extensions: ["bin", "gguf", "onnx"] }],
  });
  return typeof selection === "string" ? selection : null;
}

export function mediaSourceFromPath(path: string): string {
  return isDesktopRuntime() ? convertFileSrc(path) : path;
}

export async function analyzeVideo(request: AnalysisRequest): Promise<AnalysisResult> {
  return invoke<AnalysisResult>("analyze_video", { request });
}

export async function exportSubtitles(request: ExportSubtitlesRequest): Promise<string> {
  return invoke<string>("export_subtitles", { request });
}

export async function subscribeAnalysisProgress(
  callback: (progress: AnalysisProgress) => void,
): Promise<UnlistenFn> {
  return listen<AnalysisProgress>("analysis-progress", (event) => callback(event.payload));
}
