import { describe, expect, it } from "vitest";
import { formatCount, formatDuration, recordingLabel } from "./format";

describe("format", () => {
  it("formats duration with hours", () => {
    expect(formatDuration(3661)).toBe("1小时1分");
  });

  it("formats duration minutes only", () => {
    expect(formatDuration(120)).toBe("2分钟");
  });

  it("formats count", () => {
    expect(formatCount(1234)).toContain("1");
  });

  it("recording label", () => {
    expect(recordingLabel(true)).toBe("录制中");
    expect(recordingLabel(false)).toBe("已暂停");
  });
});
