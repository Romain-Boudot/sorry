import { reactive } from "vue";
import type {
  User,
  Channel,
  ChannelGroup,
  ChannelOverwrite,
  Message,
  Role,
  VoiceUserState,
  NotificationPref,
  WsConnection,
  WsConnectionState,
  DmMessage,
  WebhookInfo,
} from "../api";
import type { Keypair } from "../crypto";

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
  /** Webhooks publics (sans token) — utilisés pour rendre les messages d'un webhook. */
  webhooks: Map<number, WebhookInfo>;
  // ── DMs (E2EE) ──
  /** Keypair locale pour ce serveur. Null tant qu'on ne l'a pas générée. */
  dmKeypair: Keypair | null;
  /** Map<userId, DmMessage[]> — historique chiffré déchiffré côté client. */
  dms: Map<number, DmMessage[]>;
  /** Liste des conversations actives (peers avec qui on a échangé). */
  dmConversations: number[];
  /** ID du peer actuellement ouvert dans le panneau DM. null si aucun. */
  activeDmUserId: number | null;
  /** Onglet actif dans la sidebar : channels ou DMs. Non persisté. */
  activeTab: "channels" | "dms";
  /** Nombre de DMs non lus par peer. */
  dmUnread: Map<number, number>;
  /** Empreintes connues (TOFU). On compare à la clé courante du peer pour détecter une rotation. */
  knownFingerprints: Map<number, string>;
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
    webhooks: new Map(),
    dmKeypair: null,
    dms: new Map(),
    dmConversations: [],
    activeDmUserId: null,
    activeTab: "channels",
    dmUnread: new Map(),
    knownFingerprints: new Map(),
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
  /** ID du serveur dont la session a expire — declenche l'ouverture de ReauthModal. */
  reauthServerId: null as string | null,
});

// ── Core getters (needed by every domain module) ──

export function activeState(): ServerState | undefined {
  if (!store.activeServerId) return undefined;
  return store.serverStates.get(store.activeServerId);
}

export function activeServer(): SavedServer | undefined {
  return store.savedServers.find((s) => s.id === store.activeServerId);
}
