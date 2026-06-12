import { describe, expect, it } from "vitest";
import { renderMarkdownPreview } from "./markdown";

describe("markdown", () => {
  it("renders section headings and paragraphs", () => {
    const html = renderMarkdownPreview("## 今日概览\n内容段落");
    expect(html).toContain("<h2>今日概览</h2>");
    expect(html).toContain("<p>内容段落</p>");
  });

  it("wraps bullet lists in ul", () => {
    const html = renderMarkdownPreview("- 第一项\n- 第二项");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>第一项</li>");
    expect(html).toContain("</ul>");
  });

  it("renders bold inline", () => {
    const html = renderMarkdownPreview("**重点**内容");
    expect(html).toContain("<strong>重点</strong>");
  });
});
