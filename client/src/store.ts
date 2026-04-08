/**
 * Central reactive store — state, types, persistence, and thin re-exports.
 *
 * All business logic lives in composables/:
 *   useConnection.ts  — connect/disconnect/mute/unmute servers
 *   useMessaging.ts   — send/edit/delete messages
 *   useVoice.ts       — voice channel operations
 *   useNotifications.ts — notification prefs & sounds
 *   useEvents.ts      — WS event dispatching
 *
 * Components that already import from "./store" keep working —
 * the public API is re-exported here as thin wrappers.
 */
import { reactive } from "vue";
import { api, resolveBaseUrl, type User, type Channel, type ChannelGroup, type ChannelOverwrite, type Message, type Role, type VoiceUserState, type NotificationPref, type WsConnection, type WsConnectionState } from "./api";

// ── Types ──

export interface SavedServer {
  id: string;
  name: string;
  url: string;
  username: string;
  token: string;
  autoConnect?: boolean;
  iconUrl?: string | null;
  description?: string | null;
}

export interface ServerState {
  connected: boolean;
  muted: boolean;
  wsState: WsConnectionState;
  user: User | null;
  users: Map<number, User>;
  groups: ChannelGroup[];
  channels: Channel[];
  messages: Map<number, Message[]>;
  activeChannelId: number | null;
  onlineUsers: Set<number>;
  voiceState: Map<number, Map<number, VoiceUserState>>;
  wsConnection: WsConnection | null;
  unreadCount: number;
  permissions: number;
  roles: Role[];
  userRoles: Map<number, number[]>;
  maxFileSize: number;
  notificationPrefs: NotificationPref[];
  channelUnread: Map<number, number>;
  channelMentions: Map<number, number>;
  voiceChannelId: number | null;
  voiceConnectingChannelId: number | null;
  speakingUsers: Set<string>;
  voiceStatus: "idle" | "connecting" | "connected" | "error";
  isMuted: boolean;
  isDeafened: boolean;
  wasMutedBeforeDeafen: boolean;
  isScreenSharing: boolean;
  isCameraOn: boolean;
  videoTrackVersion: number;
  /** Map<channelId, Map<userId, timeout>> — users currently typing */
  typingUsers: Map<number, Map<number, ReturnType<typeof setTimeout>>>;
  /** Timestamp of last typing event sent by us */
  lastTypingSent: number;
  channelOverwrites: ChannelOverwrite[];
}

// ── Factory ──

export function createServerState(): ServerState {
  return {
    connected: false,
    muted: false,
    wsState: "disconnected",
    user: null,
    users: new Map(),
    groups: [],
    channels: [],
    messages: new Map(),
    activeChannelId: null,
    onlineUsers: new Set(),
    voiceState: new Map(),
    wsConnection: null,
    unreadCount: 0,
    notificationPrefs: [],
    channelUnread: new Map(),
    channelMentions: new Map(),
    voiceChannelId: null,
    voiceConnectingChannelId: null,
    permissions: 0,
    roles: [],
    userRoles: new Map(),
    maxFileSize: 25 * 1024 * 1024,
    speakingUsers: new Set(),
    voiceStatus: "idle",
    isMuted: false,
    isDeafened: false,
    wasMutedBeforeDeafen: false,
    isScreenSharing: false,
    isCameraOn: false,
    videoTrackVersion: 0,
    typingUsers: new Map(),
    lastTypingSent: 0,
    channelOverwrites: [],
  };
}

// ── Persistence ──

function loadSavedServers(): SavedServer[] {
  try {
    return JSON.parse(localStorage.getItem("servers") || "[]");
  } catch {
    return [];
  }
}

export function persistServers() {
  localStorage.setItem("servers", JSON.stringify(store.savedServers));
}

// ── Navigation persistence (survives page refresh) ──

function loadNavState(): { serverId: string | null; channels: Record<string, number> } {
  try {
    return JSON.parse(sessionStorage.getItem("nav") || "{}");
  } catch {
    return { serverId: null, channels: {} };
  }
}

export function persistNav() {
  const channels: Record<string, number> = {};
  for (const [id, state] of store.serverStates) {
    if (state.activeChannelId) channels[id] = state.activeChannelId;
  }
  sessionStorage.setItem("nav", JSON.stringify({
    serverId: store.activeServerId,
    channels,
  }));
}

/**
 * Pending channel selections from nav restore — applied when serverState is created.
 * Keyed by serverId → channelId.
 */
export const pendingChannels = new Map<string, number>();

/** Restore navigation state. Call BEFORE connectAll() so the UI shows immediately. */
export function restoreNav() {
  const nav = loadNavState();
  if (nav.serverId && store.savedServers.some((s) => s.id === nav.serverId)) {
    store.activeServerId = nav.serverId;
  }
  for (const [id, channelId] of Object.entries(nav.channels ?? {})) {
    if (typeof channelId === "number") {
      // If state already exists, apply directly; otherwise store for later
      const state = store.serverStates.get(id);
      if (state) {
        state.activeChannelId = channelId;
      } else {
        pendingChannels.set(id, channelId);
      }
    }
  }
}

// ── Reactive store ──

export const store = reactive({
  savedServers: loadSavedServers(),
  activeServerId: null as string | null,
  serverStates: new Map<string, ServerState>(),
  showAddServerModal: false,
  prefillServerUrl: "",
  prefillInviteCode: "",
  showSettingsModal: false,
  audioInputDevice: localStorage.getItem("audioInputDevice") || "",
  audioOutputDevice: localStorage.getItem("audioOutputDevice") || "",
  showServerSettingsModal: false,
  serverSettingsTab: "profile" as string,
  serverSettingsChannelId: null as number | null,
  channelSettingsId: null as number | null,
  groupSettingsId: null as number | null,
});

// ── Getters ──

export function activeState(): ServerState | undefined {
  if (!store.activeServerId) return undefined;
  return store.serverStates.get(store.activeServerId);
}

export function activeServer(): SavedServer | undefined {
  return store.savedServers.find((s) => s.id === store.activeServerId);
}

export function resolveUser(userId: number): string {
  const state = activeState();
  if (!state) return `User #${userId}`;
  return state.users.get(userId)?.display_name ?? `User #${userId}`;
}

export function resolveAvatarUrl(userId: number): string | null {
  const state = activeState();
  const server = activeServer();
  if (!state || !server) return null;
  const avatarPath = state.users.get(userId)?.avatar_url;
  if (!avatarPath) return null;
  return `${server.url}${avatarPath}`;
}

export function resolveUserColor(userId: number): string | null {
  const state = activeState();
  if (!state) return null;
  const roleIds = state.userRoles.get(userId);
  if (!roleIds) return null;
  const userRoles = state.roles
    .filter((r) => roleIds.includes(r.id) && r.color)
    .sort((a, b) => a.position - b.position);
  return userRoles[0]?.color ?? null;
}

export function isGuest(userId: number): boolean {
  const state = activeState();
  if (!state) return false;
  return state.users.get(userId)?.guest === true;
}

export function isUserSpeaking(userId: number): boolean {
  const state = activeState();
  if (!state) return false;
  return state.speakingUsers.has(`user-${userId}`);
}

export function getUserVoiceState(channelId: number, userId: number): VoiceUserState | undefined {
  const state = activeState();
  if (!state) return undefined;
  return state.voiceState.get(channelId)?.get(userId);
}

export function isActiveChannelVoice(): boolean {
  const state = activeState();
  if (!state?.activeChannelId) return false;
  const ch = state.channels.find((c) => c.id === state.activeChannelId);
  return ch?.kind === "voice";
}

// ── Re-exports from composables (backwards-compatible public API) ──

import { connectToServer, connectAll, muteServer, unmuteServer, removeServer } from "./composables/useConnection";
import * as _messaging from "./composables/useMessaging";
import * as _voice from "./composables/useVoice";
import * as _notifs from "./composables/useNotifications";

export { connectToServer, connectAll, muteServer, unmuteServer, removeServer };

export async function addServer(
  name: string,
  url: string,
  username: string,
  password: string,
  inviteCode?: string,
  displayName?: string,
  defaultAvatar?: File,
  totpCode?: string
) {
  const baseUrl = await resolveBaseUrl(url);

  const duplicate = store.savedServers.find(
    (s) => s.url === baseUrl && s.username === username
  );
  if (duplicate) {
    await connectToServer(duplicate.id);
    store.activeServerId = duplicate.id;
    return;
  }

  const res = await api.login(baseUrl, username, password, inviteCode, totpCode);

  if (res.totp_required) {
    throw new Error("totp_required");
  }

  if (!res.token || !res.user) {
    throw new Error("401");
  }

  if (displayName) {
    await api.updateDisplayName(baseUrl, res.token, displayName);
  }

  if (defaultAvatar) {
    try { await api.uploadAvatar(baseUrl, res.token, defaultAvatar); } catch {}
  }

  const server: SavedServer = {
    id: crypto.randomUUID(),
    name,
    url: baseUrl,
    username,
    token: res.token,
  };

  store.savedServers.push(server);
  persistServers();

  await connectToServer(server.id);
  store.activeServerId = server.id;
}

export async function addServerGuest(
  name: string,
  url: string,
  inviteCode: string,
  displayName: string,
) {
  const baseUrl = await resolveBaseUrl(url);

  const res = await api.quickLogin(baseUrl, inviteCode, displayName);

  if (!res.token || !res.user) {
    throw new Error("403");
  }

  const server: SavedServer = {
    id: crypto.randomUUID(),
    name,
    url: baseUrl,
    username: `guest-${res.user.id}`,
    token: res.token,
  };

  store.savedServers.push(server);
  persistServers();

  await connectToServer(server.id);
  store.activeServerId = server.id;
}

export function switchToServer(serverId: string) {
  store.activeServerId = serverId;
  const state = store.serverStates.get(serverId);
  if (state?.connected) {
    state.unreadCount = 0;
    if (state.activeChannelId) {
      state.channelUnread.delete(state.activeChannelId);
      state.channelMentions.delete(state.activeChannelId);
    }
  } else {
    connectToServer(serverId);
  }
  persistNav();
}

export async function selectChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  state.activeChannelId = channelId;
  state.channelUnread.delete(channelId);
  state.channelMentions.delete(channelId);
  persistNav();
  if (!state.messages.has(channelId)) {
    const msgs = await api.listMessages(server.url, server.token, channelId);
    state.messages.set(channelId, msgs.reverse());
  }
}

// Thin wrappers that resolve activeState/activeServer automatically
export async function sendMessage(content: string, files?: File[], replyToId?: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _messaging.sendMessage(server, state, content, files, replyToId);
}

export function editMessage(messageId: number, content: string) {
  const state = activeState();
  if (!state) return;
  _messaging.editMessage(state, messageId, content);
}

export function deleteMessage(messageId: number) {
  const state = activeState();
  if (!state) return;
  _messaging.deleteMessage(state, messageId);
}

export function toggleReaction(messageId: number, emoji: string) {
  const state = activeState();
  if (!state) return;
  _messaging.toggleReaction(state, messageId, emoji);
}

export function sendTyping() {
  const state = activeState();
  if (!state) return;
  _messaging.sendTyping(state);
}

export async function joinVoiceChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _voice.joinVoiceChannel(server, state, channelId);
}

export async function leaveVoiceChannel() {
  const state = activeState();
  if (!state) return;
  await _voice.leaveVoiceChannel(state);
}

export function toggleMute() {
  const state = activeState();
  if (!state) return;
  _voice.toggleMute(state);
}

export function toggleDeafen() {
  const state = activeState();
  if (!state) return;
  _voice.toggleDeafen(state);
}

export async function toggleScreenShare() {
  const state = activeState();
  if (!state) return;
  await _voice.toggleScreenShare(state);
}

export async function toggleCamera() {
  const state = activeState();
  if (!state) return;
  await _voice.toggleCamera(state);
}

export function forceMute(userId: number, muted: boolean) {
  const state = activeState();
  if (!state) return;
  _voice.forceMute(state, userId, muted);
}

export function forceDeafen(userId: number, deafened: boolean) {
  const state = activeState();
  if (!state) return;
  _voice.forceDeafen(state, userId, deafened);
}

export async function setNotificationPref(
  scope: "channel" | "server",
  targetId: number,
  level: "all" | "mentions" | "nothing",
  muteUntil?: string | null,
) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _notifs.setNotificationPref(server, state, scope, targetId, level, muteUntil);
}

export async function removeNotificationPref(scope: "channel" | "server", targetId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _notifs.removeNotificationPref(server, state, scope, targetId);
}
