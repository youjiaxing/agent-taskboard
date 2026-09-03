export function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

export function safeHttpUrl(raw: string, baseUrl?: string): string | null {
  try {
    const url = baseUrl ? new URL(raw.trim(), baseUrl) : new URL(raw.trim());
    return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : null;
  } catch {
    return null;
  }
}

export function renderMarkdown(markdown: string, baseUrl: string): string {
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n");
  const blocks: string[] = [];
  let paragraph: string[] = [];
  let list: { ordered: boolean; items: string[] } | null = null;
  const flushParagraph = () => {
    if (!paragraph.length) return;
    blocks.push(`<p>${renderInlineMarkdown(paragraph.join(" "), baseUrl)}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (!list) return;
    const tag = list.ordered ? "ol" : "ul";
    blocks.push(`<${tag}>${list.items.map((item) => `<li>${renderInlineMarkdown(item, baseUrl)}</li>`).join("")}</${tag}>`);
    list = null;
  };
  for (const line of lines) {
    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    const item = /^\s*([-*+] |\d+\. )(.+)$/.exec(line);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length;
      blocks.push(`<h${level}>${renderInlineMarkdown(heading[2], baseUrl)}</h${level}>`);
    } else if (item) {
      flushParagraph();
      const ordered = /^\d/.test(item[1]);
      if (list && list.ordered !== ordered) flushList();
      list ??= { ordered, items: [] };
      list.items.push(item[2]);
    } else if (!line.trim()) {
      flushParagraph();
      flushList();
    } else {
      flushList();
      paragraph.push(line.trim());
    }
  }
  flushParagraph();
  flushList();
  return blocks.join("");
}

function renderInlineMarkdown(source: string, baseUrl: string): string {
  const token = /(`[^`\n]+`|\*\*[^*\n]+\*\*|\[[^\]\n]+\]\([^\n)]+\))/g;
  let html = "";
  let offset = 0;
  for (const match of source.matchAll(token)) {
    const index = match.index ?? 0;
    html += escapeHtml(source.slice(offset, index));
    const value = match[0];
    if (value.startsWith("`")) {
      html += `<code>${escapeHtml(value.slice(1, -1))}</code>`;
    } else if (value.startsWith("**")) {
      html += `<strong>${escapeHtml(value.slice(2, -2))}</strong>`;
    } else {
      const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(value);
      const href = link ? safeHttpUrl(link[2], baseUrl) : null;
      html += href
        ? `<a href="${escapeHtml(href)}" data-act="open-external" data-url="${escapeHtml(href)}">${escapeHtml(link?.[1] ?? "")}</a>`
        : `<span class="unsafe-link">${escapeHtml(link?.[1] ?? value)}</span>`;
    }
    offset = index + value.length;
  }
  return html + escapeHtml(source.slice(offset));
}

export function formatTime(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(ms);
  }
}

export function formatCountdown(ms: number): string {
  const seconds = Math.max(0, Math.ceil(ms / 1000));
  if (seconds >= 60) {
    const minutes = Math.floor(seconds / 60);
    const rest = seconds % 60;
    return rest ? `${minutes}m ${rest}s` : `${minutes}m`;
  }
  return `${seconds}s`;
}

export function toLocalInput(ms: number): string {
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function addOpt(left?: number | null, right?: number | null): number | null {
  if (left == null || right == null) return null;
  return left + right;
}

export function parsePairingPayload(raw: string): { address: string; code: string } | null {
  const parts = raw
    .trim()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (parts.length < 2) return null;
  const address = parts[0];
  const code = parts[parts.length - 1];
  if (!address.includes("://") || !code) return null;
  return { address, code };
}
