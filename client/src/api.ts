async function hashPassword(password: string): Promise<string> {
  const encoded = new TextEncoder().encode(password);
  const hash = await crypto.subtle.digest("SHA-256", encoded);
  return Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2, "0")).join("");
}

async function resolveBaseUrl(input: string): Promise<string> {
  const stripped = input.replace(/^https?:\/\//, "").replace(/\/+$/, "");
  for (const scheme of ["https", "http"]) {
    try {
      const url = `${scheme}://${stripped}`;
      const res = await fetch(`${url}/info`, { mode: "cors" });
      if (res.ok) return url;
    } catch {}
  }
  throw new Error("Server unreachable");
}

async function request<T>(
  baseUrl: string,
  path: string,
  token?: string,
  options: RequestInit = {}
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };

  const res = await fetch(`${baseUrl}/api${path}`, { ...options, headers });
  if (!res.ok) throw new Error(`${res.status}`);
  if (res.status === 204) return undefined as T;
  return res.json();
}

export { resolveBaseUrl };

export const api = {
  async serverInfo(baseUrl: string) {
    const res = await fetch(`${baseUrl}/info`);
    if (!res.ok) throw new Error(`${res.status}`);
    return res.json() as Promise<{ name: string; description?: string; icon_url?: string }>;
  },

  serverStats(baseUrl: string, token: string) {
    return request<ServerStats>(baseUrl, "/server/stats", token);
  },

  updateServer(baseUrl: string, token: string, data: { name?: string; description?: string }) {
    return request<void>(baseUrl, "/server", token, {
      method: "PATCH",
      body: JSON.stringify(data),
    });
  },

  async uploadServerIcon(baseUrl: string, token: string, file: File) {
    const formData = new FormData();
    formData.append("icon", file);
    const res = await fetch(`${baseUrl}/api/server/icon`, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}` },
      body: formData,
    });
    if (!res.ok) throw new Error(`${res.status}`);
    return res.json() as Promise<{ icon_url: string }>;
  },

  async deleteServerIcon(baseUrl: string, token: string) {
    const res = await fetch(`${baseUrl}/api/server/icon`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok) throw new Error(`${res.status}`);
  },

  async login(baseUrl: string, username: string, password: string, inviteCode?: string, totpCode?: string) {
    const hashed = await hashPassword(password);
    return request<{ token?: string; user?: User; totp_required?: boolean }>(baseUrl, "/auth/login", undefined, {
      method: "POST",
      body: JSON.stringify({ username, password: hashed, invite_code: inviteCode, totp_code: totpCode }),
    });
  },

  totpSetup(baseUrl: string, token: string) {
    return request<{ secret: string; otpauth_url: string }>(baseUrl, "/auth/totp/setup", token, {
      method: "POST",
    });
  },

  totpVerify(baseUrl: string, token: string, code: string) {
    return request<void>(baseUrl, "/auth/totp/verify", token, {
      method: "POST",
      body: JSON.stringify({ code }),
    });
  },

  totpDisable(baseUrl: string, token: string) {
    return request<void>(baseUrl, "/auth/totp/disable", token, {
      method: "POST",
    });
  },

  totpStatus(baseUrl: string, token: string) {
    return request<{ enabled: boolean }>(baseUrl, "/auth/totp/status", token);
  },

  checkInvite(baseUrl: string, code: string) {
    return request<{ valid: boolean; guest: boolean }>(baseUrl, `/invites/check/${code}`);
  },

  quickLogin(baseUrl: string, inviteCode: string, displayName: string) {
    return request<{ token?: string; user?: User }>(baseUrl, "/auth/quick", undefined, {
      method: "POST",
      body: JSON.stringify({ invite_code: inviteCode, display_name: displayName }),
    });
  },

  refreshToken(baseUrl: string, token: string) {
    return request<{ token: string }>(baseUrl, "/auth/refresh", token, {
      method: "POST",
    });
  },

  fetchOg(baseUrl: string, token: string, url: string) {
    return request<{ title?: string; description?: string; image?: string; site_name?: string; url: string }>(baseUrl, "/og", token, {
      method: "POST",
      body: JSON.stringify({ url }),
    });
  },

  me(baseUrl: string, token: string) {
    return request<MeResponse>(baseUrl, "/users/me", token);
  },

  listChannels(baseUrl: string, token: string) {
    return request<Channel[]>(baseUrl, "/channels", token);
  },

  createChannel(baseUrl: string, token: string, name: string, kind: "text" | "voice", groupId?: number) {
    return request<Channel>(baseUrl, "/channels", token, {
      method: "POST",
      body: JSON.stringify({ name, kind, group_id: groupId ?? null }),
    });
  },

  listGroups(baseUrl: string, token: string) {
    return request<ChannelGroup[]>(baseUrl, "/channels/groups", token);
  },

  createGroup(baseUrl: string, token: string, name: string) {
    return request<ChannelGroup>(baseUrl, "/channels/groups", token, {
      method: "POST",
      body: JSON.stringify({ name }),
    });
  },

  updateGroup(baseUrl: string, token: string, id: number, name: string) {
    return request<void>(baseUrl, `/channels/groups/${id}`, token, {
      method: "PATCH",
      body: JSON.stringify({ name }),
    });
  },

  deleteGroup(baseUrl: string, token: string, id: number) {
    return request<void>(baseUrl, `/channels/groups/${id}`, token, { method: "DELETE" });
  },

  reorderChannels(baseUrl: string, token: string, ids: number[]) {
    return request<void>(baseUrl, "/channels/reorder", token, {
      method: "POST",
      body: JSON.stringify({ ids }),
    });
  },

  reorderGroups(baseUrl: string, token: string, ids: number[]) {
    return request<void>(baseUrl, "/channels/groups/reorder", token, {
      method: "POST",
      body: JSON.stringify({ ids }),
    });
  },

  deleteChannel(baseUrl: string, token: string, channelId: number) {
    return request<void>(baseUrl, `/channels/${channelId}`, token, { method: "DELETE" });
  },

  updateChannel(baseUrl: string, token: string, channelId: number, data: { name?: string }) {
    return request<Channel>(baseUrl, `/channels/${channelId}`, token, {
      method: "PATCH",
      body: JSON.stringify(data),
    });
  },

  moveChannel(baseUrl: string, token: string, channelId: number, groupId: number | null) {
    return request<void>(baseUrl, `/channels/${channelId}/group`, token, {
      method: "PATCH",
      body: JSON.stringify({ group_id: groupId }),
    });
  },

  listMessages(baseUrl: string, token: string, channelId: number, limit = 50, before?: number) {
    const params = new URLSearchParams({ limit: String(limit) });
    if (before) params.set("before", String(before));
    return request<Message[]>(baseUrl, `/channels/${channelId}/messages?${params}`, token);
  },

  userMessages(baseUrl: string, token: string, userId: number, limit = 50, before?: number) {
    const params = new URLSearchParams({ limit: String(limit) });
    if (before) params.set("before", String(before));
    return request<Message[]>(baseUrl, `/users/${userId}/messages?${params}`, token);
  },

  channelAttachments(baseUrl: string, token: string, channelId: number, limit = 50, before?: number) {
    const params = new URLSearchParams({ limit: String(limit) });
    if (before) params.set("before", String(before));
    return request<ChannelAttachment[]>(baseUrl, `/channels/${channelId}/attachments?${params}`, token);
  },

  searchMessages(baseUrl: string, token: string, channelId: number, query: string, limit = 25) {
    const params = new URLSearchParams({ q: query, limit: String(limit) });
    return request<Message[]>(baseUrl, `/channels/${channelId}/search?${params}`, token);
  },

  sendMessage(baseUrl: string, token: string, channelId: number, content: string, replyToId?: number) {
    return request<Message>(baseUrl, `/channels/${channelId}/messages`, token, {
      method: "POST",
      body: JSON.stringify({ content, reply_to_id: replyToId }),
    });
  },

  async sendMessageWithFiles(baseUrl: string, token: string, channelId: number, content: string, files: File[], replyToId?: number) {
    const formData = new FormData();
    formData.append("content", content);
    if (replyToId) formData.append("reply_to_id", String(replyToId));
    for (const file of files) {
      formData.append("file", file);
    }
    const res = await fetch(`${baseUrl}/api/channels/${channelId}/upload`, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}` },
      body: formData,
    });
    if (!res.ok) throw new Error(`${res.status}`);
    return res.json() as Promise<Message>;
  },

  getLivekitToken(baseUrl: string, token: string, channelId: number) {
    return request<{ token: string; url: string }>(baseUrl, "/livekit/token", token, {
      method: "POST",
      body: JSON.stringify({ channel_id: channelId }),
    });
  },

  updateDisplayName(baseUrl: string, token: string, displayName: string) {
    return request<User>(baseUrl, "/users/me", token, {
      method: "PATCH",
      body: JSON.stringify({ display_name: displayName }),
    });
  },

  async uploadAvatar(baseUrl: string, token: string, file: File) {
    const formData = new FormData();
    formData.append("avatar", file);
    const res = await fetch(`${baseUrl}/api/users/me/avatar`, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}` },
      body: formData,
    });
    if (!res.ok) throw new Error(`${res.status}`);
    return res.json() as Promise<User>;
  },

  async changePassword(baseUrl: string, token: string, currentPassword: string, newPassword: string) {
    const currentHashed = await hashPassword(currentPassword);
    const newHashed = await hashPassword(newPassword);
    return request<void>(baseUrl, "/users/me/password", token, {
      method: "POST",
      body: JSON.stringify({ current_password: currentHashed, new_password: newHashed }),
    });
  },

  async deleteAvatar(baseUrl: string, token: string) {
    const res = await fetch(`${baseUrl}/api/users/me/avatar`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok) throw new Error(`${res.status}`);
  },

  banUser(baseUrl: string, token: string, userId: number) {
    return request<void>(baseUrl, `/users/${userId}/ban`, token, { method: "POST" });
  },

  unbanUser(baseUrl: string, token: string, userId: number) {
    return request<void>(baseUrl, `/users/${userId}/ban`, token, { method: "DELETE" });
  },

  listBanned(baseUrl: string, token: string) {
    return request<BannedUser[]>(baseUrl, "/users/banned", token);
  },

  listInvites(baseUrl: string, token: string) {
    return request<Invite[]>(baseUrl, "/invites", token);
  },

  createInvite(baseUrl: string, token: string, data: { max_uses?: number | null; expires_at?: number | null; role_id?: number | null; guest?: boolean }) {
    return request<Invite>(baseUrl, "/invites", token, {
      method: "POST",
      body: JSON.stringify(data),
    });
  },

  deleteInvite(baseUrl: string, token: string, code: string) {
    return request<void>(baseUrl, `/invites/${code}`, token, { method: "DELETE" });
  },

  getUserRoles(baseUrl: string, token: string, userId: number) {
    return request<Role[]>(baseUrl, `/users/${userId}/roles`, token);
  },

  listRoles(baseUrl: string, token: string) {
    return request<Role[]>(baseUrl, "/roles", token);
  },

  updateRole(baseUrl: string, token: string, roleId: number, data: { name: string; permissions: number; color?: string | null }) {
    return request<void>(baseUrl, `/roles/${roleId}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  },

  reorderRoles(baseUrl: string, token: string, ids: number[]) {
    return request<void>(baseUrl, "/roles/reorder", token, {
      method: "POST",
      body: JSON.stringify({ ids }),
    });
  },

  deleteRole(baseUrl: string, token: string, roleId: number) {
    return request<void>(baseUrl, `/roles/${roleId}`, token, { method: "DELETE" });
  },

  listOverwrites(baseUrl: string, token: string, channelId: number) {
    return request<ChannelOverwrite[]>(baseUrl, `/channels/${channelId}/overwrites`, token);
  },

  setOverwrite(baseUrl: string, token: string, channelId: number, roleId: number, allow: number, deny: number) {
    return request<void>(baseUrl, `/channels/${channelId}/overwrites`, token, {
      method: "PUT",
      body: JSON.stringify({ role_id: roleId, allow, deny }),
    });
  },

  deleteOverwrite(baseUrl: string, token: string, channelId: number, roleId: number) {
    return request<void>(baseUrl, `/channels/${channelId}/overwrites`, token, {
      method: "DELETE",
      body: JSON.stringify({ role_id: roleId }),
    });
  },

  getNotificationPrefs(baseUrl: string, token: string) {
    return request<NotificationPref[]>(baseUrl, "/notifications/preferences", token);
  },

  setNotificationPref(baseUrl: string, token: string, data: { scope: string; target_id: number; level: string; mute_until?: string | null }) {
    return request<void>(baseUrl, "/notifications/preferences", token, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  },

  deleteNotificationPref(baseUrl: string, token: string, scope: string, targetId: number) {
    return request<void>(baseUrl, "/notifications/preferences", token, {
      method: "DELETE",
      body: JSON.stringify({ scope, target_id: targetId }),
    });
  },

  assignRole(baseUrl: string, token: string, roleId: number, userId: number) {
    return request<void>(baseUrl, `/roles/${roleId}/assign`, token, {
      method: "POST",
      body: JSON.stringify({ user_id: userId }),
    });
  },

  removeRole(baseUrl: string, token: string, roleId: number, userId: number) {
    return request<void>(baseUrl, `/roles/${roleId}/remove`, token, {
      method: "POST",
      body: JSON.stringify({ user_id: userId }),
    });
  },
};

// Types
export interface User {
  id: number;
  display_name: string;
  avatar_url: string | null;
  username?: string;
  created_at?: string;
  guest?: boolean;
}

export interface BannedUser {
  id: number;
  display_name: string;
  username: string;
  avatar_url: string | null;
  banned_at: number;
}

export interface ChannelGroup {
  id: number;
  name: string;
  position: number;
}

export interface Channel {
  id: number;
  name: string;
  kind: "text" | "voice";
  position: number;
  group_id: number | null;
}

export interface Role {
  id: number;
  name: string;
  permissions: number;
  color: string | null;
  position: number;
}

export interface Attachment {
  id: number;
  filename: string;
  content_type: string;
  size: number;
  url: string;
}

export interface ChannelAttachment {
  id: number;
  filename: string;
  content_type: string;
  size: number;
  url: string;
  author_id: number;
  created_at: string;
}

export interface ServerStats {
  version: string;
  uptime_secs: number;
  users_total: number;
  users_online: number;
  users_guests: number;
  channels_text: number;
  channels_voice: number;
  messages_total: number;
  messages_today: number;
  files_total: number;
  files_size_bytes: number;
  bans_active: number;
  invites_active: number;
  db_size_bytes: number;
  disk_free_bytes: number;
  disk_total_bytes: number;
}

export interface ReplyPreview {
  id: number;
  author_id: number;
  content: string;
}

export interface Mention {
  kind: "user" | "role";
  id: number;
}

export interface Reaction {
  emoji: string;
  count: number;
  user_ids: number[];
}

export interface Message {
  id: number;
  channel_id: number;
  author_id: number;
  content: string;
  created_at: string;
  attachments: Attachment[];
  reply_to?: ReplyPreview;
  mentions: Mention[];
  reactions: Reaction[];
}

export interface NotificationPref {
  scope: "channel" | "server";
  target_id: number;
  level: "all" | "mentions" | "nothing";
  mute_until: string | null;
}

export interface VoiceUserState {
  muted: boolean;
  deafened: boolean;
  force_muted: boolean;
  force_deafened: boolean;
  screen_sharing: boolean;
  camera_on: boolean;
}

export interface Invite {
  code: string;
  created_by: number;
  max_uses: number | null;
  uses: number;
  expires_at: number | null;
  created_at: number;
  role_id: number | null;
  guest: boolean;
}

export interface ChannelOverwrite {
  channel_id: number;
  role_id: number;
  allow: number;
  deny: number;
}

export interface MeResponse {
  user: User;
  permissions: number;
  users: User[];
  online_users: number[];
  voice_state: Record<number, Record<number, VoiceUserState>>;
  roles: Role[];
  user_roles: Record<number, number[]>;
  max_file_size: number;
}

export interface ServerEvent {
  type: string;
  data: unknown;
}

/** Event séquencé reçu du serveur (contient seq + type + data) */
export interface SequencedEvent extends ServerEvent {
  seq: number;
}

/** Snapshot complet reçu à la connexion/reconnexion */
export interface Snapshot {
  seq: number;
  user: User;
  permissions: number;
  users: User[];
  online_users: number[];
  channels: Channel[];
  groups: ChannelGroup[];
  roles: Role[];
  user_roles: Record<number, number[]>;
  voice_state: Record<number, Record<number, VoiceUserState>>;
  server_name: string;
  server_description: string | null;
  server_icon_url: string | null;
  max_file_size: number;
}

/** États FSM de la connexion WebSocket */
export type WsConnectionState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting";

export interface WsConnection {
  ws: WebSocket | null;
  state: WsConnectionState;
  lastSeq: number;
  reconnectAttempt: number;
  /** Annule la reconnexion en cours et ferme le WS */
  destroy: () => void;
}

/**
 * Crée une connexion WS avec :
 * - Backoff exponentiel (1s → 2s → 4s → 8s → max 30s)
 * - Tracking du seq number pour détecter les gaps
 * - Snapshot automatique à la connexion (envoyé par le serveur)
 * - RequestSnapshot si gap détecté
 */
export function createWsConnection(
  baseUrl: string,
  getToken: () => string,
  callbacks: {
    onSnapshot: (snapshot: Snapshot) => void;
    onEvent: (event: SequencedEvent) => void;
    onStateChange: (state: WsConnectionState) => void;
  }
): WsConnection {
  let destroyed = false;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  const conn: WsConnection = {
    ws: null,
    state: "disconnected",
    lastSeq: 0,
    reconnectAttempt: 0,
    destroy: () => {
      destroyed = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      conn.ws?.close();
      conn.ws = null;
      setState("disconnected");
    },
  };

  function setState(s: WsConnectionState) {
    conn.state = s;
    callbacks.onStateChange(s);
  }

  function connect() {
    if (destroyed) return;

    // Fermer proprement l'ancien WS avant d'en créer un nouveau
    if (conn.ws) {
      const old = conn.ws;
      old.onclose = null; // éviter que le onclose relance un reconnect
      old.onmessage = null;
      old.onerror = null;
      old.close();
      conn.ws = null;
    }

    const isReconnect = conn.reconnectAttempt > 0;
    setState(isReconnect ? "reconnecting" : "connecting");

    const url = new URL(baseUrl);
    const proto = url.protocol === "https:" ? "wss:" : "ws:";
    const ws = new WebSocket(`${proto}//${url.host}/ws?token=${getToken()}`);
    conn.ws = ws;

    ws.onopen = () => {
      conn.reconnectAttempt = 0;
      setState("connected");
    };

    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data);

        // Snapshot (envoyé automatiquement à la connexion ou sur RequestSnapshot)
        if (msg.type === "Snapshot") {
          const snapshot = msg.data as Snapshot;
          conn.lastSeq = snapshot.seq;
          callbacks.onSnapshot(snapshot);
          return;
        }

        // Event séquencé normal
        const event = msg as SequencedEvent;
        if (event.seq !== undefined) {
          // Ignorer les events déjà couverts par le snapshot
          if (event.seq <= conn.lastSeq) return;
          // Détecter un gap dans la séquence
          if (conn.lastSeq > 0 && event.seq > conn.lastSeq + 1) {
            console.warn(`[WS] Gap détecté: attendu ${conn.lastSeq + 1}, reçu ${event.seq}. Demande de snapshot.`);
            ws.send(JSON.stringify({ type: "RequestSnapshot" }));
            return;
          }
          conn.lastSeq = event.seq;
        }

        callbacks.onEvent(event);
      } catch {
        // ignore malformed
      }
    };

    ws.onclose = () => {
      if (destroyed) return;
      conn.ws = null;
      scheduleReconnect();
    };

    ws.onerror = () => {
      // onclose sera appelé après
    };
  }

  function scheduleReconnect() {
    if (destroyed) return;
    setState("reconnecting");
    conn.reconnectAttempt++;
    // Backoff exponentiel: 1s, 2s, 4s, 8s, 16s, 30s max
    const delay = Math.min(1000 * Math.pow(2, conn.reconnectAttempt - 1), 30000);
    reconnectTimer = setTimeout(connect, delay);
  }

  // Connexion initiale
  connect();
  return conn;
}
