import type { Message } from "../../api";
import { activeState, activeServer } from "../core";

/** True if a message originated from a webhook (live or since-deleted). */
export function isWebhookMessage(msg: Message): boolean {
  return msg.webhook_id != null || msg.webhook_username != null;
}

/** Display name for a webhook-authored message (override -> webhook -> fallback). */
export function resolveWebhookName(msg: Message): string {
  if (msg.webhook_username) return msg.webhook_username;
  const state = activeState();
  if (state && msg.webhook_id != null) {
    const wh = state.webhooks.get(msg.webhook_id);
    if (wh) return wh.name;
  }
  return "Webhook";
}

/** Resolved avatar URL for a webhook-authored message (or null for default).
 *  Relative paths (uploads stored on the server) are prefixed with the server's base URL;
 *  absolute http(s) URLs (legacy/per-message override) are returned as-is. */
export function resolveWebhookAvatar(msg: Message): string | null {
  const raw = msg.webhook_avatar_url
    ?? (msg.webhook_id != null ? activeState()?.webhooks.get(msg.webhook_id)?.avatar_url ?? null : null);
  if (!raw) return null;
  if (raw.startsWith("http://") || raw.startsWith("https://")) return raw;
  const server = activeServer();
  return server ? `${server.url}${raw}` : raw;
}
