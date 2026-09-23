import { Marked, Renderer, type Tokens } from "marked";

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
  const parser = new Marked<string, string>({
    async: false,
    gfm: true,
    breaks: false,
  });
  return parser.parse(markdown, {
    async: false,
    renderer: new IssueMarkdownRenderer(baseUrl),
  });
}

/**
 * Renders Tracker Markdown as inert HTML.
 *
 * Issue bodies are untrusted: raw HTML is escaped instead of passed through,
 * images never load remote content, and links only survive as the existing
 * safe http/https product action.
 */
class IssueMarkdownRenderer extends Renderer {
  constructor(private readonly baseUrl: string) {
    super();
  }

  override html({ text }: Tokens.HTML | Tokens.Tag): string {
    return escapeHtml(text);
  }

  override image({ text, tokens }: Tokens.Image): string {
    const label = this.parser.parseInline(tokens, this.parser.textRenderer);
    return `<span class="unsafe-image">${escapeHtml(label || text)}</span>`;
  }

  override listitem(item: Tokens.ListItem): string {
    const rendered = super.listitem(item);
    return item.task ? rendered.replace("<li>", '<li class="task-list-item">') : rendered;
  }

  override table(token: Tokens.Table): string {
    return `<div class="issue-table-scroll">${super.table(token)}</div>`;
  }

  override link({ href, title, tokens }: Tokens.Link): string {
    const label = this.parser.parseInline(tokens);
    const safe = safeHttpUrl(href, this.baseUrl);
    if (!safe) {
      return `<span class="unsafe-link">${label}</span>`;
    }
    const titleAttribute = title ? ` title="${escapeHtml(title)}"` : "";
    return `<a href="${escapeHtml(safe)}" data-act="open-external" data-url="${escapeHtml(safe)}"${titleAttribute}>${label}</a>`;
  }
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
