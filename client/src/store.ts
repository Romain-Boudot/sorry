import { reactive } from "vue";
import { api, connectWS, type User, type Channel, type Message, type ServerEvent } from "./api";
import { joinVoice, leaveVoice, toggleMute as voiceToggleMute } from "./voice";

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
  users: Map<number, User>; // cache id → user
  channels: Channel[];
  messages: Map<number, Message[]>;
  activeChannelId: number | null;
  onlineUsers: Set<number>;
  voiceState: Map<number, Set<number>>;
  ws: WebSocket | null;
  unreadCount: number;
  // Vocal
  voiceChannelId: number | null; // channel vocal actif (localement)
  isMuted: boolean;
}

function createServerState(): ServerState {
  return {
    connected: false,
    muted: false,
    user: null,
    users: new Map(),
    channels: [],
    messages: new Map(),
    activeChannelId: null,
    onlineUsers: new Set(),
    voiceState: new Map(),
    ws: null,
    unreadCount: 0,
    voiceChannelId: null,
    isMuted: false,
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

/// Ajouter un nouveau serveur et s'y connecter
export async function addServer(
  name: string,
  url: string,
  username: string,
  password: string,
  serverPassword?: string
) {
  const baseUrl = url.replace(/\/+$/, "");
  const res = await api.login(baseUrl, username, password, serverPassword);

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

  // Déjà connecté ?
  const existing = store.serverStates.get(serverId);
  if (existing?.connected) {
    store.activeServerId = serverId;
    return;
  }

  const state = createServerState();
  store.serverStates.set(serverId, state);

  try {
    const [me, channels] = await Promise.all([
      api.me(server.url, server.token),
      api.listChannels(server.url, server.token),
    ]);

    state.user = me.user;
    state.channels = channels;
    state.connected = true;
    state.onlineUsers = new Set(me.online_users);

    // Cache des users
    for (const u of me.users) {
      state.users.set(u.id, u);
    }

    for (const [chId, userIds] of Object.entries(me.voice_state)) {
      state.voiceState.set(Number(chId), new Set(userIds));
    }

    // Premier channel texte par défaut
    const firstText = channels.find((c) => c.kind === "text");
    if (firstText) {
      state.activeChannelId = firstText.id;
      const msgs = await api.listMessages(server.url, server.token, firstText.id);
      state.messages.set(firstText.id, msgs.reverse());
    }

    // WebSocket
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
    // Pas encore connecté → on connecte
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

/// Rejoindre un channel vocal
export async function joinVoiceChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  // Demander un token LiveKit au backend
  const { token, url } = await api.getLivekitToken(server.url, server.token, channelId);

  await joinVoice(url, token, {
    onConnected: () => {
      state.voiceChannelId = channelId;
      state.isMuted = false;
      // Notifier les autres via WS
      if (state.ws && state.ws.readyState === WebSocket.OPEN) {
        state.ws.send(JSON.stringify({ type: "JoinVoice", data: { channel_id: channelId } }));
      }
    },
    onDisconnected: () => {
      const prevChannel = state.voiceChannelId;
      state.voiceChannelId = null;
      state.isMuted = false;
      if (prevChannel && state.ws && state.ws.readyState === WebSocket.OPEN) {
        state.ws.send(JSON.stringify({ type: "LeaveVoice", data: { channel_id: prevChannel } }));
      }
    },
    onParticipantJoined: () => {},
    onParticipantLeft: () => {},
    onError: (err) => {
      console.error("Voice error:", err);
    },
  });
}

/// Quitter le channel vocal
export async function leaveVoiceChannel() {
  const state = activeState();
  if (!state) return;

  const prevChannel = state.voiceChannelId;
  await leaveVoice();
  state.voiceChannelId = null;
  state.isMuted = false;

  if (prevChannel && state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(JSON.stringify({ type: "LeaveVoice", data: { channel_id: prevChannel } }));
  }
}

/// Toggle mute micro
export function toggleMute() {
  const state = activeState();
  if (!state) return;
  const newState = voiceToggleMute();
  state.isMuted = !newState; // toggleMute retourne le nouvel état du micro (true = enabled)
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

  // Si c'était le serveur actif, switch au prochain connecté
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
      // Incrémenter unread si pas le serveur actif
      if (store.activeServerId !== serverId) {
        state.unreadCount++;
      }
      break;
    }
    case "UserOnline": {
      const { user_id } = event.data as { user_id: number };
      state.onlineUsers.add(user_id);
      break;
    }
    case "UserOffline": {
      const { user_id } = event.data as { user_id: number };
      state.onlineUsers.delete(user_id);
      break;
    }
    case "UserJoinedVoice": {
      const { user, channel_id } = event.data as { user: User; channel_id: number };
      if (!state.voiceState.has(channel_id)) {
        state.voiceState.set(channel_id, new Set());
      }
      state.voiceState.get(channel_id)!.add(user.id);
      break;
    }
    case "UserLeftVoice": {
      const { user_id, channel_id } = event.data as { user_id: number; channel_id: number };
      state.voiceState.get(channel_id)?.delete(user_id);
      break;
    }
  }
}
