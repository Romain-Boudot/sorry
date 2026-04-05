import MarkdownIt from "markdown-it";

const md = new MarkdownIt({
  html: false,        // No raw HTML — XSS prevention
  breaks: true,       // Convert \n to <br>
  linkify: true,      // Auto-detect URLs and make them clickable
  typographer: false,  // No fancy quotes etc
});

// Only allow http/https links (block javascript:, data:, etc.)
const defaultLinkOpen = md.renderer.rules.link_open ||
  function (tokens, idx, options, _env, self) { return self.renderToken(tokens, idx, options); };

md.renderer.rules.link_open = function (tokens, idx, options, env, self) {
  const href = tokens[idx].attrGet("href");
  if (href && !/^https?:\/\//i.test(href)) {
    // Replace dangerous link with harmless text
    tokens[idx].attrSet("href", "#");
  }
  // Open links in new tab
  tokens[idx].attrSet("target", "_blank");
  tokens[idx].attrSet("rel", "noopener noreferrer");
  return defaultLinkOpen(tokens, idx, options, env, self);
};

// Disable image rendering (use attachments instead)
md.disable("image");

// Limit heading levels: render h1/h2 as h3
const defaultHeadingOpen = md.renderer.rules.heading_open ||
  function (tokens, idx, options, _env, self) { return self.renderToken(tokens, idx, options); };

md.renderer.rules.heading_open = function (tokens, idx, options, env, self) {
  const level = parseInt(tokens[idx].tag.slice(1));
  if (level < 3) {
    tokens[idx].tag = "h3";
  }
  return defaultHeadingOpen(tokens, idx, options, env, self);
};

md.renderer.rules.heading_close = function (tokens, idx, options, env, self) {
  const level = parseInt(tokens[idx].tag.slice(1));
  if (level < 3) {
    tokens[idx].tag = "h3";
  }
  return self.renderToken(tokens, idx, options);
};

type MentionResolver = (kind: "user" | "role", id: number) => { name: string; color?: string | null } | null;

let mentionResolver: MentionResolver | null = null;

export function setMentionResolver(resolver: MentionResolver) {
  mentionResolver = resolver;
}

function sanitizeColor(color: string | null | undefined): string {
  if (!color) return "";
  if (/^#[0-9a-fA-F]{3}([0-9a-fA-F]{3})?$/.test(color)) return color;
  if (/^rgb\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*\)$/.test(color)) return color;
  return "";
}

function escapeHtml(str: string): string {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function renderMentions(html: string): string {
  if (!mentionResolver) return html;
  return html.replace(/&lt;@(&amp;)?(\d+)&gt;/g, (_match, isRole, idStr) => {
    const id = parseInt(idStr);
    const kind = isRole ? "role" : "user";
    const resolved = mentionResolver!(kind as "user" | "role", id);
    if (!resolved) return kind === "role" ? `@unknown-role` : `@unknown`;
    const safeColor = sanitizeColor(resolved.color);
    const style = safeColor ? ` style="color:${safeColor}"` : "";
    return `<span class="mention"${style}>@${escapeHtml(resolved.name)}</span>`;
  });
}

export function renderMarkdown(content: string): string {
  return renderMentions(md.render(content));
}

/** Extract all http/https URLs from text */
export function extractUrls(content: string): string[] {
  const urlRegex = /https?:\/\/[^\s<>)"'\]]+/gi;
  return [...new Set(content.match(urlRegex) || [])];
}
