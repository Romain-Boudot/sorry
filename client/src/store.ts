import { reactive } from "vue";
import { api, resolveBaseUrl, connectWS, type User, type Channel, type ChannelGroup, type Message, type Role, type ServerEvent, type VoiceUserState, type NotificationPref } from "./api";
import { joinVoice, leaveVoice, toggleMute as voiceToggleMute, toggleDeafen as voiceToggleDeafen, setMuted as voiceSetMuted, setDeafened as voiceSetDeafened, startScreenShare as voiceStartScreenShare, stopScreenShare as voiceStopScreenShare, setCameraEnabled as voiceSetCamera } from "./voice";

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
  muted: boolean; // déconnecté manuellement
  user: User | null;
  users: Map<number, User>;
  groups: ChannelGroup[];
  channels: Channel[];
  messages: Map<number, Message[]>;
  activeChannelId: number | null;
  onlineUsers: Set<number>;
  voiceState: Map<number, Map<number, VoiceUserState>>;
  ws: WebSocket | null;
  unreadCount: number;
  permissions: number;
  roles: Role[];
  userRoles: Map<number, number[]>;
  maxFileSize: number;
  // Notifications
  notificationPrefs: NotificationPref[];
  channelUnread: Map<number, number>;
  channelMentions: Map<number, number>;
  // Vocal
  voiceChannelId: number | null;
  voiceConnectingChannelId: number | null;
  speakingUsers: Set<string>;
  voiceStatus: "idle" | "connecting" | "connected" | "error";
  isMuted: boolean;
  isDeafened: boolean;
  wasMutedBeforeDeafen: boolean;
  isScreenSharing: boolean;
  isCameraOn: boolean;
  videoTrackVersion: number; // incremented on track changes to trigger reactivity
}

function defaultVoiceUserState(): VoiceUserState {
  return { muted: false, deafened: false, force_muted: false, force_deafened: false };
}

function createServerState(): ServerState {
  return {
    connected: false,
    muted: false,
    user: null,
    users: new Map(),
    groups: [],
    channels: [],
    messages: new Map(),
    activeChannelId: null,
    onlineUsers: new Set(),
    voiceState: new Map(),
    ws: null,
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
  };
}

const refreshTimers = new Map<string, ReturnType<typeof setTimeout>>();

function getTokenExp(token: string): number | null {
  try {
    const payload = JSON.parse(atob(token.split(".")[1]));
    return payload.exp ?? null;
  } catch { return null; }
}

function scheduleTokenRefresh(serverId: string) {
  // Clear any existing timer
  const existing = refreshTimers.get(serverId);
  if (existing) clearTimeout(existing);

  const server = store.savedServers.find((s) => s.id === serverId);
  if (!server) return;

  const exp = getTokenExp(server.token);
  if (!exp) return;

  // Refresh when 80% of the TTL has elapsed (e.g. 4 days into a 5-day token)
  const nowSecs = Math.floor(Date.now() / 1000);
  const remaining = exp - nowSecs;
  const refreshIn = Math.max(remaining * 0.8, 60) * 1000; // at least 1 min

  const timer = setTimeout(async () => {
    try {
      const res = await api.refreshToken(server.url, server.token);
      server.token = res.token;
      persistServers();
      // Schedule next refresh
      scheduleTokenRefresh(serverId);
    } catch {
      // Token expired or server unreachable — user will need to re-login
    }
  }, refreshIn);

  refreshTimers.set(serverId, timer);
}

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

/// State du serveur actif (pour les composants)
export function activeState(): ServerState | undefined {
  if (!store.activeServerId) return undefined;
  return store.serverStates.get(store.activeServerId);
}

export function activeServer(): SavedServer | undefined {
  return store.savedServers.find((s) => s.id === store.activeServerId);
}

/// Résoudre un user id → display name (serveur actif)
export function resolveUser(userId: number): string {
  const state = activeState();
  if (!state) return `User #${userId}`;
  return state.users.get(userId)?.display_name ?? `User #${userId}`;
}

/// Résoudre un user id → avatar URL complète (serveur actif)
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
  // Find the highest role (lowest position) that has a color
  const userRoles = state.roles
    .filter((r) => roleIds.includes(r.id) && r.color)
    .sort((a, b) => a.position - b.position);
  return userRoles[0]?.color ?? null;
}

/// Est-ce qu'un user est un guest ?
export function isGuest(userId: number): boolean {
  const state = activeState();
  if (!state) return false;
  return state.users.get(userId)?.guest === true;
}

/// Est-ce qu'un user (par son id) est en train de parler ?
export function isUserSpeaking(userId: number): boolean {
  const state = activeState();
  if (!state) return false;
  return state.speakingUsers.has(`user-${userId}`);
}

/// Récupère le voice state d'un user dans un channel
export function getUserVoiceState(channelId: number, userId: number): VoiceUserState | undefined {
  const state = activeState();
  if (!state) return undefined;
  return state.voiceState.get(channelId)?.get(userId);
}

/// Le channel actif est-il un channel vocal ?
export function isActiveChannelVoice(): boolean {
  const state = activeState();
  if (!state?.activeChannelId) return false;
  const ch = state.channels.find((c) => c.id === state.activeChannelId);
  return ch?.kind === "voice";
}

/// Envoyer l'état vocal self au serveur (seulement si en vocal)
function sendVoiceStateUpdate(state: ServerState) {
  if (!state.voiceChannelId) return;
  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(JSON.stringify({
      type: "UpdateVoiceState",
      data: { muted: state.isMuted, deafened: state.isDeafened },
    }));
  }
}

/// Ajouter un nouveau serveur et s'y connecter
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
    try {
      await api.uploadAvatar(baseUrl, res.token, defaultAvatar);
    } catch {}
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

/// Connexion rapide (guest) — invite code + display name only
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

/// Connecter à un serveur (sans déconnecter les autres)
export async function connectToServer(serverId: string) {
  const server = store.savedServers.find((s) => s.id === serverId);
  if (!server) return;

  const existing = store.serverStates.get(serverId);
  if (existing?.connected) {
    store.activeServerId = serverId;
    return;
  }

  const state = createServerState();
  store.serverStates.set(serverId, state);

  try {
    // Resolve protocol if needed (fixes saved http:// URLs when server uses https)
    const resolvedUrl = await resolveBaseUrl(server.url);
    if (resolvedUrl !== server.url) {
      server.url = resolvedUrl;
      persistServers();
    }

    const [me, channels, groups, info] = await Promise.all([
      api.me(server.url, server.token),
      api.listChannels(server.url, server.token),
      api.listGroups(server.url, server.token),
      api.serverInfo(server.url).catch(() => null),
    ]);

    // Update saved server info from /info
    if (info) {
      server.name = info.name;
      server.iconUrl = info.icon_url ?? null;
      server.description = info.description ?? null;
      persistServers();
    }

    state.user = me.user;
    state.permissions = me.permissions;
    state.groups = groups;
    state.channels = channels;
    state.connected = true;
    state.onlineUsers = new Set(me.online_users);
    state.onlineUsers.add(me.user.id);
    scheduleTokenRefresh(serverId);

    for (const u of me.users) {
      state.users.set(u.id, u);
    }

    state.roles = me.roles;
    state.maxFileSize = me.max_file_size;
    for (const [uid, rids] of Object.entries(me.user_roles)) {
      state.userRoles.set(Number(uid), rids as number[]);
    }

    for (const [chId, usersObj] of Object.entries(me.voice_state)) {
      const map = new Map<number, VoiceUserState>();
      for (const [uid, vs] of Object.entries(usersObj)) {
        map.set(Number(uid), vs as VoiceUserState);
      }
      state.voiceState.set(Number(chId), map);
    }

    const firstText = channels.find((c) => c.kind === "text");
    if (firstText) {
      state.activeChannelId = firstText.id;
      const msgs = await api.listMessages(server.url, server.token, firstText.id);
      state.messages.set(firstText.id, msgs.reverse());
    }

    // Load notification preferences
    try {
      state.notificationPrefs = await api.getNotificationPrefs(server.url, server.token);
    } catch {}

    // Request browser notification permission
    if (Notification.permission === "default") {
      Notification.requestPermission();
    }

    state.ws = connectWS(server.url, server.token, (event) =>
      handleEvent(serverId, event)
    );

    store.activeServerId = serverId;
  } catch {
    state.connected = false;
  }
}

/// Connecter à TOUS les serveurs non-mutés
export async function connectAll() {
  const promises = store.savedServers.map((server) => {
    if (server.autoConnect === false) return Promise.resolve();
    const state = store.serverStates.get(server.id);
    if (state?.muted) return Promise.resolve();
    return connectToServer(server.id).catch(() => {});
  });
  await Promise.all(promises);
}

/// Switch l'affichage vers un serveur (sans reconnecter)
export function switchToServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  if (state?.connected) {
    store.activeServerId = serverId;
    state.unreadCount = 0;
    // Clear unread/mentions for the currently viewed channel
    if (state.activeChannelId) {
      state.channelUnread.delete(state.activeChannelId);
      state.channelMentions.delete(state.activeChannelId);
    }
  } else {
    connectToServer(serverId);
  }
}

export async function selectChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  state.activeChannelId = channelId;
  // Clear unread/mentions for this channel
  state.channelUnread.delete(channelId);
  state.channelMentions.delete(channelId);
  if (!state.messages.has(channelId)) {
    const msgs = await api.listMessages(server.url, server.token, channelId);
    state.messages.set(channelId, msgs.reverse());
  }
}

export async function sendMessage(content: string, files?: File[], replyToId?: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state?.activeChannelId) return;
  if (!content.trim() && (!files || files.length === 0)) return;

  if (files && files.length > 0) {
    // Use REST upload endpoint for messages with files
    await api.sendMessageWithFiles(server.url, server.token, state.activeChannelId, content, files, replyToId);
    // The server broadcasts MessageCreate via WS, so it will appear automatically
  } else if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(
      JSON.stringify({
        type: "SendMessage",
        data: { channel_id: state.activeChannelId, content, reply_to_id: replyToId ?? null },
      })
    );
  }
}

export function editMessage(messageId: number, content: string) {
  const state = activeState();
  if (!state?.ws || state.ws.readyState !== WebSocket.OPEN) return;
  state.ws.send(JSON.stringify({
    type: "EditMessage",
    data: { message_id: messageId, content },
  }));
}

export function deleteMessage(messageId: number) {
  const state = activeState();
  if (!state?.ws || state.ws.readyState !== WebSocket.OPEN) return;
  // Optimistic delete
  for (const [, msgs] of state.messages) {
    const idx = msgs.findIndex((m) => m.id === messageId);
    if (idx >= 0) { msgs.splice(idx, 1); break; }
  }
  state.ws.send(JSON.stringify({
    type: "DeleteMessage",
    data: { message_id: messageId },
  }));
}

/// Rejoindre un channel vocal
export async function joinVoiceChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  state.voiceStatus = "connecting";
  state.voiceConnectingChannelId = channelId;

  try {
    const { token, url } = await api.getLivekitToken(server.url, server.token, channelId);

    await joinVoice(url, token, {
      onConnected: () => {
        state.voiceChannelId = channelId;
        state.voiceStatus = "connected";
        // Apply pre-existing mute/deaf state
        if (state.isMuted) {
          voiceToggleMute(); // mic was enabled by default on join, disable it
        }
        if (state.isDeafened) {
          voiceToggleDeafen();
        }
        if (state.ws && state.ws.readyState === WebSocket.OPEN) {
          state.ws.send(JSON.stringify({ type: "JoinVoice", data: { channel_id: channelId } }));
          sendVoiceStateUpdate(state);
        }
      },
      onDisconnected: () => {
        const prevChannel = state.voiceChannelId;
        if (!prevChannel) return; // jamais vraiment connecté, onError s'en occupe
        state.voiceChannelId = null;
        state.voiceConnectingChannelId = null;
        state.voiceStatus = "idle";
        if (state.ws && state.ws.readyState === WebSocket.OPEN) {
          state.ws.send(JSON.stringify({ type: "LeaveVoice", data: { channel_id: prevChannel } }));
        }
      },
      onParticipantJoined: () => {},
      onParticipantLeft: () => {},
      onActiveSpeakersChanged: (identities) => {
        state.speakingUsers = new Set(identities);
      },
      onTrackChanged: () => {
        state.videoTrackVersion++;
      },
      onError: (err) => {
        state.voiceStatus = "error";
        console.error("Voice error:", err);
      },
    });
  } catch {
    state.voiceStatus = "error";
  }
}

/// Quitter le channel vocal
export async function leaveVoiceChannel() {
  const state = activeState();
  if (!state) return;

  const prevChannel = state.voiceChannelId;
  await leaveVoice();
  state.voiceChannelId = null;
  state.voiceConnectingChannelId = null;
  state.voiceStatus = "idle";
  state.isScreenSharing = false;
  state.isCameraOn = false;
  // Keep mute/deaf state — user may want to rejoin muted

  if (prevChannel && state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(JSON.stringify({ type: "LeaveVoice", data: { channel_id: prevChannel } }));
  }
}

/// Toggle mute micro
export function toggleMute() {
  const state = activeState();
  if (!state) return;
  if (state.voiceChannelId) {
    const micEnabled = voiceToggleMute();
    state.isMuted = !micEnabled;
  } else {
    state.isMuted = !state.isMuted;
  }
  // If unmuting while deafened, undeafen too
  if (!state.isMuted && state.isDeafened) {
    state.isDeafened = false;
    if (state.voiceChannelId) voiceToggleDeafen();
  }
  sendVoiceStateUpdate(state);
}

/// Toggle deafen (sourd)
export function toggleDeafen() {
  const state = activeState();
  if (!state) return;
  const wasDeafened = state.isDeafened;

  if (!wasDeafened) {
    // Becoming deafened — remember current mute state
    state.wasMutedBeforeDeafen = state.isMuted;
    state.isDeafened = true;
    if (!state.isMuted) {
      state.isMuted = true;
      if (state.voiceChannelId) voiceToggleMute();
    }
    if (state.voiceChannelId) voiceToggleDeafen();
  } else {
    // Undeafening — restore previous mute state
    state.isDeafened = false;
    if (state.voiceChannelId) voiceToggleDeafen();
    if (!state.wasMutedBeforeDeafen) {
      state.isMuted = false;
      if (state.voiceChannelId) voiceToggleMute();
    }
  }
  sendVoiceStateUpdate(state);
}

/// Toggle screen share
export async function toggleScreenShare() {
  const state = activeState();
  if (!state?.voiceChannelId) return;
  if (state.isScreenSharing) {
    await voiceStopScreenShare();
    state.isScreenSharing = false;
  } else {
    const ok = await voiceStartScreenShare();
    state.isScreenSharing = ok;
  }
  state.videoTrackVersion++;
}

/// Toggle webcam
export async function toggleCamera() {
  const state = activeState();
  if (!state?.voiceChannelId) return;
  const next = !state.isCameraOn;
  const ok = await voiceSetCamera(next);
  if (ok) state.isCameraOn = next;
  state.videoTrackVersion++;
}

/// Force mute un autre user (nécessite MUTE_MEMBERS)
export function forceMute(userId: number, muted: boolean) {
  const state = activeState();
  if (!state?.ws || state.ws.readyState !== WebSocket.OPEN) return;
  state.ws.send(JSON.stringify({
    type: "ForceMute",
    data: { user_id: userId, muted },
  }));
}

/// Force deafen un autre user (nécessite DEAFEN_MEMBERS)
export function forceDeafen(userId: number, deafened: boolean) {
  const state = activeState();
  if (!state?.ws || state.ws.readyState !== WebSocket.OPEN) return;
  state.ws.send(JSON.stringify({
    type: "ForceDeafen",
    data: { user_id: userId, deafened },
  }));
}

/// Set notification preference for a channel or server
export async function setNotificationPref(
  scope: "channel" | "server",
  targetId: number,
  level: "all" | "mentions" | "nothing",
  muteUntil?: string | null,
) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  await api.setNotificationPref(server.url, server.token, {
    scope,
    target_id: targetId,
    level,
    mute_until: muteUntil ?? null,
  });

  // Update local state
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

/// Remove notification preference (reset to default)
export async function removeNotificationPref(scope: "channel" | "server", targetId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  await api.deleteNotificationPref(server.url, server.token, scope, targetId);
  state.notificationPrefs = state.notificationPrefs.filter(
    (p) => !(p.scope === scope && p.target_id === targetId)
  );
}

/// Déconnecter d'un serveur (mute — plus de WS, plus de notifs)
export function muteServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  if (state) {
    if (state.ws) state.ws.close();
    state.ws = null;
    state.connected = false;
    state.muted = true;
  }

  if (store.activeServerId === serverId) {
    const next = store.savedServers.find(
      (s) => s.id !== serverId && store.serverStates.get(s.id)?.connected
    );
    store.activeServerId = next?.id ?? null;
  }
}

/// Reconnecter un serveur muté
export function unmuteServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  if (state) {
    state.muted = false;
  }
  connectToServer(serverId);
}

export function removeServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  if (state?.ws) state.ws.close();
  store.serverStates.delete(serverId);
  store.savedServers = store.savedServers.filter((s) => s.id !== serverId);
  persistServers();

  if (store.activeServerId === serverId) {
    store.activeServerId = store.savedServers[0]?.id ?? null;
  }
}

// ── Notification helpers ──

let notifAudio: HTMLAudioElement | null = null;
function getNotifAudio(): HTMLAudioElement {
  if (!notifAudio) {
    notifAudio = new Audio("/notif.wav");
    notifAudio.volume = 0.5;
  }
  return notifAudio;
}

function getEffectiveNotifLevel(state: ServerState, channelId: number): "all" | "mentions" | "nothing" {
  const now = new Date().toISOString();
  // Channel-level pref takes priority
  const channelPref = state.notificationPrefs.find(
    (p) => p.scope === "channel" && p.target_id === channelId
  );
  if (channelPref) {
    if (channelPref.mute_until && channelPref.mute_until < now) {
      // Mute expired — treat as default (fall through to server)
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
      return "all"; // expired
    }
    return serverPref.level as "all" | "mentions" | "nothing";
  }
  return "all";
}

function isMentioned(state: ServerState, msg: Message): boolean {
  if (!state.user) return false;
  // Direct user mention
  if (msg.mentions?.some((m) => m.kind === "user" && m.id === state.user!.id)) return true;
  // Role mention
  const userRoleIds = state.userRoles.get(state.user.id) ?? [];
  if (msg.mentions?.some((m) => m.kind === "role" && userRoleIds.includes(m.id))) return true;
  return false;
}

function fireNotification(state: ServerState, serverId: string, msg: Message) {
  if (msg.author_id === state.user?.id) return;

  const level = getEffectiveNotifLevel(state, msg.channel_id);
  const mentioned = isMentioned(state, msg);

  if (level === "nothing") return;
  if (level === "mentions" && !mentioned) return;

  // Track mention count
  if (mentioned) {
    state.channelMentions.set(msg.channel_id, (state.channelMentions.get(msg.channel_id) ?? 0) + 1);
  }

  // Don't fire sound/browser notif if user is viewing this exact channel
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

function handleEvent(serverId: string, event: ServerEvent) {
  const state = store.serverStates.get(serverId);
  if (!state) return;

  switch (event.type) {
    case "MessageCreate": {
      const msg = event.data as Message;
      const msgs = state.messages.get(msg.channel_id);
      if (msgs) {
        msgs.push(msg);
      } else {
        state.messages.set(msg.channel_id, [msg]);
      }
      // Per-channel unread tracking
      const isViewingChannel = store.activeServerId === serverId && state.activeChannelId === msg.channel_id;
      if (!isViewingChannel && msg.author_id !== state.user?.id) {
        state.channelUnread.set(msg.channel_id, (state.channelUnread.get(msg.channel_id) ?? 0) + 1);
      }
      if (store.activeServerId !== serverId) {
        state.unreadCount++;
      }
      // Fire notification (sound + browser)
      fireNotification(state, serverId, msg);
      break;
    }
    case "MessageDelete": {
      const { id } = event.data as { id: number };
      for (const [, msgs] of state.messages) {
        const idx = msgs.findIndex((m) => m.id === id);
        if (idx >= 0) { msgs.splice(idx, 1); break; }
      }
      break;
    }
    case "MessageUpdate": {
      const msg = event.data as Message;
      const msgs = state.messages.get(msg.channel_id);
      if (msgs) {
        const idx = msgs.findIndex((m) => m.id === msg.id);
        if (idx >= 0) msgs[idx] = msg;
      }
      break;
    }
    case "UserOnline": {
      const { user } = event.data as { user: User };
      state.onlineUsers.add(user.id);
      state.users.set(user.id, user);
      break;
    }
    case "UserOffline": {
      const { user_id } = event.data as { user_id: number };
      state.onlineUsers.delete(user_id);
      break;
    }
    case "UserJoinedVoice": {
      const { user, channel_id, voice_state: vs } = event.data as { user: User; channel_id: number; voice_state: VoiceUserState };
      if (!state.voiceState.has(channel_id)) {
        state.voiceState.set(channel_id, new Map());
      }
      state.voiceState.get(channel_id)!.set(user.id, vs ?? defaultVoiceUserState());
      break;
    }
    case "UserLeftVoice": {
      const { user_id, channel_id } = event.data as { user_id: number; channel_id: number };
      state.voiceState.get(channel_id)?.delete(user_id);
      break;
    }
    case "VoiceStateUpdate": {
      const { user_id, channel_id, voice_state: vs } = event.data as { user_id: number; channel_id: number; voice_state: VoiceUserState };

      // Get previous state to detect force changes
      const prevVs = state.voiceState.get(channel_id)?.get(user_id);
      const wasForced = prevVs?.force_muted ?? false;
      const wasForcedDeaf = prevVs?.force_deafened ?? false;

      if (!state.voiceState.has(channel_id)) {
        state.voiceState.set(channel_id, new Map());
      }
      state.voiceState.get(channel_id)!.set(user_id, vs);

      // Only react to force changes on myself
      if (user_id === state.user?.id && state.voiceChannelId) {
        // Force mute/deafen: don't touch isMuted/isDeafened (those reflect self state only)
        // LiveKit server-side handles the actual mute via API

        // Force deafen changed: ON — deafen audio locally (can't hear others)
        if (vs.force_deafened && !wasForcedDeaf) {
          voiceSetDeafened(true);
        }
        // Force deafen changed: OFF — restore audio
        if (!vs.force_deafened && wasForcedDeaf) {
          voiceSetDeafened(false);
        }
      }
      break;
    }
    case "RoleCreate": {
      const role = event.data as Role;
      state.roles.push(role);
      break;
    }
    case "RoleUpdate": {
      const role = event.data as Role;
      const idx = state.roles.findIndex((r) => r.id === role.id);
      if (idx >= 0) state.roles[idx] = role;
      break;
    }
    case "RoleDelete": {
      const { id } = event.data as { id: number };
      state.roles = state.roles.filter((r) => r.id !== id);
      break;
    }
    case "UserRoleUpdate": {
      const { user_id, role_ids, permissions } = event.data as { user_id: number; role_ids: number[]; permissions: number };
      state.userRoles.set(user_id, role_ids);
      // Update permissions in real-time if this is the current user
      if (state.user && user_id === state.user.id) {
        state.permissions = permissions;
      }
      break;
    }
    case "UserUpdate": {
      const user = event.data as User;
      state.users.set(user.id, user);
      if (state.user?.id === user.id) {
        state.user = user;
      }
      break;
    }
    case "ServerUpdate": {
      const { name, icon_url, description } = event.data as { name: string; description: string | null; icon_url: string | null };
      const saved = store.savedServers.find((s) => s.id === serverId);
      if (saved) {
        saved.name = name;
        saved.iconUrl = icon_url;
        saved.description = description;
        persistServers();
      }
      break;
    }
  }
}
