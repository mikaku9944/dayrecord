function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function inlineFormat(s: string): string {
  return escapeHtml(s).replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
}

/** Lightweight Markdown preview for daily summaries (## / ### / lists / paragraphs). */
export function renderMarkdownPreview(md: string): string {
  const lines = md.replace(/\r\n/g, "\n").split("\n");
  const out: string[] = [];
  let inList = false;

  const closeList = (): void => {
    if (inList) {
      out.push("</ul>");
      inList = false;
    }
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      closeList();
      continue;
    }

    if (trimmed.startsWith("## ")) {
      closeList();
      out.push(`<h2>${inlineFormat(trimmed.slice(3))}</h2>`);
    } else if (trimmed.startsWith("### ")) {
      closeList();
      out.push(`<h3>${inlineFormat(trimmed.slice(4))}</h3>`);
    } else if (/^[-*] /.test(trimmed)) {
      if (!inList) {
        out.push("<ul>");
        inList = true;
      }
      out.push(`<li>${inlineFormat(trimmed.slice(2))}</li>`);
    } else if (/^\d+\.\s/.test(trimmed)) {
      closeList();
      out.push(`<p class="md-ordered">${inlineFormat(trimmed)}</p>`);
    } else {
      closeList();
      out.push(`<p>${inlineFormat(trimmed)}</p>`);
    }
  }

  closeList();
  return out.join("");
}
