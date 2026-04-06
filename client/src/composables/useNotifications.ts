/**
 * Notification logic: prefs, unread tracking, sound, browser notifications.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { api, type Message, type ServerEvent, type User, type Role } from "../api";
import type { ServerState, SavedServer } from "../store";

// ── Audio ──

let notifAudio: HTMLAudioElement | null = null;
function getNotifAudio(): HTMLAudioElement {
  if (!notifAudio) {
    notifAudio = new Audio("/notif.wav");
    notifAudio.volume = 0.5;
  }
  return notifAudio;
}

// ── Effective level ──

function getEffectiveNotifLevel(state: ServerState, channelId: number): "all" | "mentions" | "nothing" {
  const now = new Date().toISOString();
  // Channel-level pref takes priority
  const channelPref = state.notificationPrefs.find(
    (p) => p.scope === "channel" && p.target_id === channelId
  );
  if (channelPref) {
    if (channelPref.mute_until && channelPref.mute_until < now) {
      // Mute expired — fall through to server
    } else {
      return channelPref.level as "all" | "mentions" | "nothing";
    }
  }
  // Server-level pref
  const serverPref = state.notificationPrefs.find(
    (p) => p.scope === "server" && p.target_id === 0
  );
  if (serverPref) {
    if (serverPref.mute_until && serverPref.mute_until < now) {
      return "all";
    }
    return serverPref.level as "all" | "mentions" | "nothing";
  }
  return "all";
}

// ── Mention detection ──

function isMentioned(state: ServerState, msg: Message): boolean {
  if (!state.user) return false;
  if (msg.mentions?.some((m) => m.kind === "user" && m.id === state.user!.id)) return true;
  const userRoleIds = state.userRoles.get(state.user.id) ?? [];
  if (msg.mentions?.some((m) => m.kind === "role" && userRoleIds.includes(m.id))) return true;
  return false;
}

// ── Public API ──

/**
 * Fire a notification (sound + browser) for a new message.
 * Called from the event handler, not from components directly.
 */
export function fireNotification(
  store: { activeServerId: string | null; savedServers: SavedServer[] },
  state: ServerState,
  serverId: string,
  msg: Message,
) {
  if (msg.author_id === state.user?.id) return;

  const level = getEffectiveNotifLevel(state, msg.channel_id);
  const mentioned = isMentioned(state, msg);

  if (level === "nothing") return;
  if (level === "mentions" && !mentioned) return;

  // Track mention count
  if (mentioned) {
    state.channelMentions.set(msg.channel_id, (state.channelMentions.get(msg.channel_id) ?? 0) + 1);
  }

  // Don't fire if user is viewing this channel
  const isViewing = store.activeServerId === serverId && state.activeChannelId === msg.channel_id && document.hasFocus();
  if (isViewing) return;

  // Play sound
  try { getNotifAudio().play(); } catch {}

  // Browser notification
  if (Notification.permission === "granted") {
    const server = store.savedServers.find((s) => s.id === serverId);
    const authorName = state.users.get(msg.author_id)?.display_name ?? "Someone";
    const channelName = state.channels.find((c) => c.id === msg.channel_id)?.name ?? "channel";
    const title = mentioned ? `${authorName} vous a mentionné` : `${authorName} dans #${channelName}`;
    const body = msg.content.length > 100 ? msg.content.slice(0, 100) + "..." : msg.content;
    new Notification(title, { body, tag: `sorry-${serverId}-${msg.id}`, icon: server?.iconUrl ? `${server.url}${server.iconUrl}` : undefined });
  }
}

/**
 * Set a notification preference for a channel or server.
 */
export async function setNotificationPref(
  server: SavedServer,
  state: ServerState,
  scope: "channel" | "server",
  targetId: number,
  level: "all" | "mentions" | "nothing",
  muteUntil?: string | null,
) {
  await api.setNotificationPref(server.url, server.token, {
    scope,
    target_id: targetId,
    level,
    mute_until: muteUntil ?? null,
  });

  const existing = state.notificationPrefs.findIndex(
    (p) => p.scope === scope && p.target_id === targetId
  );
  const pref = { scope, target_id: targetId, level, mute_until: muteUntil ?? null };
  if (existing >= 0) {
    state.notificationPrefs[existing] = pref;
  } else {
    state.notificationPrefs.push(pref);
  }
}

/**
 * Remove (reset) a notification preference.
 */
export async function removeNotificationPref(
  server: SavedServer,
  state: ServerState,
  scope: "channel" | "server",
  targetId: number,
) {
  await api.deleteNotificationPref(server.url, server.token, scope, targetId);
  state.notificationPrefs = state.notificationPrefs.filter(
    (p) => !(p.scope === scope && p.target_id === targetId)
  );
}
