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

export function renderMarkdown(content: string): string {
  return md.render(content);
}

/** Extract all http/https URLs from text */
export function extractUrls(content: string): string[] {
  const urlRegex = /https?:\/\/[^\s<>)"'\]]+/gi;
  return [...new Set(content.match(urlRegex) || [])];
}
