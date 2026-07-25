import { describe, expect, it } from "vitest";
import type { SubtitleSegment } from "./playdub";
import { findActiveSegment, formatClock, normalizeSegments } from "./subtitles";

const SEGMENTS: SubtitleSegment[] = [
  {
    id: 2,
    startMs: 1_500,
    endMs: 3_000,
    sourceText: "Second",
    translatedText: "第二句",
  },
  { id: 1, startMs: 0, endMs: 1_500, sourceText: "First" },
];

describe("formatClock", () => {
  it("formats short and long playback times", () => {
    expect(formatClock(65.9)).toBe("01:05");
    expect(formatClock(3_661)).toBe("01:01:01");
  });

  it("guards invalid playback times", () => {
    expect(formatClock(Number.NaN)).toBe("00:00");
    expect(formatClock(-1)).toBe("00:00");
  });
});

describe("normalizeSegments", () => {
  it("sorts segments, trims text, and drops malformed entries", () => {
    expect(
      normalizeSegments([
        ...SEGMENTS,
        { id: 3, startMs: 4_000, endMs: 2_000, sourceText: "ignored" },
        { id: 4, startMs: 5_000, endMs: 6_000, sourceText: "  " },
      ]),
    ).toEqual([
      { id: 1, startMs: 0, endMs: 1_500, sourceText: "First" },
      {
        id: 2,
        startMs: 1_500,
        endMs: 3_000,
        sourceText: "Second",
        translatedText: "第二句",
      },
    ]);
  });
});

describe("findActiveSegment", () => {
  const sorted = normalizeSegments(SEGMENTS);

  it("uses start-inclusive and end-exclusive media timestamps", () => {
    expect(findActiveSegment(sorted, 0)?.id).toBe(1);
    expect(findActiveSegment(sorted, 1.499)?.id).toBe(1);
    expect(findActiveSegment(sorted, 1.5)?.id).toBe(2);
    expect(findActiveSegment(sorted, 3)).toBeNull();
  });

  it("returns null for an invalid playback clock", () => {
    expect(findActiveSegment(sorted, Number.NaN)).toBeNull();
    expect(findActiveSegment(sorted, -1)).toBeNull();
  });
});
