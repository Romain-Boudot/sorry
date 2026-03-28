import { reactive } from "vue";
import { api, connectWS, type User, type Channel, type ChannelGroup, type Message, type ServerEvent, type VoiceUserState } from "./api";
import { joinVoice, leaveVoice, toggleMute as voiceToggleMute, toggleDeafen as voiceToggleDeafen } from "./voice";

export interface SavedServer {
  id: string;
  name: string;
  url: string;
  username: string;
  token: string;
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
  // Vocal
  voiceChannelId: number | null;
  voiceConnectingChannelId: number | null;
  speakingUsers: Set<string>;
  voiceStatus: "idle" | "connecting" | "connected" | "error";
  isMuted: boolean;
  isDeafened: boolean;
  wasMutedBeforeDeafen: boolean;
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
    voiceChannelId: null,
    voiceConnectingChannelId: null,
    permissions: 0,
    speakingUsers: new Set(),
    voiceStatus: "idle",
    isMuted: false,
    isDeafened: false,
    wasMutedBeforeDeafen: false,
  };
}

function loadSavedServers(): SavedServer[] {
  try {
    return JSON.parse(localStorage.getItem("servers") || "[]");
  } catch {
    return [];
  }
}

function persistServers() {
  localStorage.setItem("servers", JSON.stringify(store.savedServers));
}

export const store = reactive({
  savedServers: loadSavedServers(),
  activeServerId: null as string | null,
  serverStates: new Map<string, ServerState>(),
  showAddServerModal: false,
  showSettingsModal: false,
  showServerSettingsModal: false,
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
  serverPassword?: string,
  displayName?: string
) {
  const baseUrl = url.replace(/\/+$/, "");
  const res = await api.login(baseUrl, username, password, serverPassword);

  if (displayName) {
    await api.updateDisplayName(baseUrl, res.token, displayName);
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
    const [me, channels, groups] = await Promise.all([
      api.me(server.url, server.token),
      api.listChannels(server.url, server.token),
      api.listGroups(server.url, server.token),
    ]);

    state.user = me.user;
    state.permissions = me.permissions;
    state.groups = groups;
    state.channels = channels;
    state.connected = true;
    state.onlineUsers = new Set(me.online_users);
    state.onlineUsers.add(me.user.id);

    for (const u of me.users) {
      state.users.set(u.id, u);
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
  } else {
    connectToServer(serverId);
  }
}

export async function selectChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  state.activeChannelId = channelId;
  if (!state.messages.has(channelId)) {
    const msgs = await api.listMessages(server.url, server.token, channelId);
    state.messages.set(channelId, msgs.reverse());
  }
}

export async function sendMessage(content: string) {
  const state = activeState();
  if (!state?.activeChannelId || !content.trim()) return;

  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(
      JSON.stringify({
        type: "SendMessage",
        data: { channel_id: state.activeChannelId, content },
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
      if (store.activeServerId !== serverId) {
        state.unreadCount++;
      }
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
      if (!state.voiceState.has(channel_id)) {
        state.voiceState.set(channel_id, new Map());
      }
      state.voiceState.get(channel_id)!.set(user_id, vs);

      // Si c'est moi qui suis force muted/deafened, appliquer localement
      if (user_id === state.user?.id) {
        if (vs.force_muted && !state.isMuted) {
          state.isMuted = true;
          voiceToggleMute();
        }
        if (vs.force_deafened && !state.isDeafened) {
          state.isDeafened = true;
          voiceToggleDeafen();
        }
      }
      break;
    }
  }
}
