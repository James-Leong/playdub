import { describe, expect, it } from "vitest";
import { formatRuntimeInfo } from "./runtime";

describe("formatRuntimeInfo", () => {
  it("formats the application name and version", () => {
    expect(formatRuntimeInfo({ appName: "Playdub", version: "0.1.0" })).toBe("Playdub 0.1.0");
  });
});
