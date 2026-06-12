import { describe, expect, it } from "vitest";
import { renderMarkdownPreview } from "./markdown";

describe("markdown", () => {
  it("renders headings", () => {
    const html = renderMarkdownPreview("## 今日概览\n内容");
    expect(html).toContain("<h2>今日概览</h2>");
  });
});
