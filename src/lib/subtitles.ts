import type { SubtitleSegment } from "./playdub";

export function formatClock(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return "00:00";
  }

  const wholeSeconds = Math.floor(seconds);
  const hours = Math.floor(wholeSeconds / 3600);
  const minutes = Math.floor((wholeSeconds % 3600) / 60);
  const remainder = wholeSeconds % 60;

  if (hours > 0) {
    return `${hours.toString().padStart(2, "0")}:${minutes
      .toString()
      .padStart(2, "0")}:${remainder.toString().padStart(2, "0")}`;
  }

  return `${minutes.toString().padStart(2, "0")}:${remainder.toString().padStart(2, "0")}`;
}

export function findActiveSegment(
  segments: readonly SubtitleSegment[],
  playbackSeconds: number,
): SubtitleSegment | null {
  if (!Number.isFinite(playbackSeconds) || playbackSeconds < 0 || segments.length === 0) {
    return null;
  }

  const playbackMs = playbackSeconds * 1000;
  let active: SubtitleSegment | null = null;

  for (const segment of segments) {
    if (segment.startMs > playbackMs) {
      break;
    }
    if (segment.endMs > playbackMs) {
      active = segment;
    }
  }

  return active;
}

export function normalizeSegments(segments: readonly SubtitleSegment[]): SubtitleSegment[] {
  return segments
    .filter(
      (segment) =>
        Number.isFinite(segment.startMs) &&
        Number.isFinite(segment.endMs) &&
        segment.startMs >= 0 &&
        segment.endMs > segment.startMs &&
        segment.sourceText.trim().length > 0,
    )
    .map((segment) => ({
      ...segment,
      sourceText: segment.sourceText.trim(),
      translatedText: segment.translatedText?.trim() || undefined,
    }))
    .sort((left, right) => left.startMs - right.startMs || left.endMs - right.endMs);
}
