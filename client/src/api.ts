async function hashPassword(password: string): Promise<string> {
  const encoded = new TextEncoder().encode(password);
  const hash = await crypto.subtle.digest("SHA-256", encoded);
  return Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2, "0")).join("");
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

export const api = {
  async serverInfo(baseUrl: string) {
    const res = await fetch(`${baseUrl}/info`);
    if (!res.ok) throw new Error(`${res.status}`);
    return res.json() as Promise<{ name: string }>;
  },

  async login(baseUrl: string, username: string, password: string, serverPassword?: string) {
    const hashed = await hashPassword(password);
    return request<{ token: string; user: User }>(baseUrl, "/auth/login", undefined, {
      method: "POST",
      body: JSON.stringify({ username, password: hashed, server_password: serverPassword }),
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

  sendMessage(baseUrl: string, token: string, channelId: number, content: string) {
    return request<Message>(baseUrl, `/channels/${channelId}/messages`, token, {
      method: "POST",
      body: JSON.stringify({ content }),
    });
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

  getUserRoles(baseUrl: string, token: string, userId: number) {
    return request<Role[]>(baseUrl, `/users/${userId}/roles`, token);
  },

  listRoles(baseUrl: string, token: string) {
    return request<Role[]>(baseUrl, "/roles", token);
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

export interface Message {
  id: number;
  channel_id: number;
  author_id: number;
  content: string;
  created_at: string;
}

export interface VoiceUserState {
  muted: boolean;
  deafened: boolean;
  force_muted: boolean;
  force_deafened: boolean;
}

export interface MeResponse {
  user: User;
  permissions: number;
  users: User[];
  online_users: number[];
  voice_state: Record<number, Record<number, VoiceUserState>>;
}

export interface ServerEvent {
  type: string;
  data: unknown;
}

export function connectWS(
  baseUrl: string,
  token: string,
  onEvent: (event: ServerEvent) => void
): WebSocket {
  const url = new URL(baseUrl);
  const proto = url.protocol === "https:" ? "wss:" : "ws:";
  const ws = new WebSocket(`${proto}//${url.host}/ws?token=${token}`);

  ws.onmessage = (e) => {
    try {
      const event = JSON.parse(e.data) as ServerEvent;
      onEvent(event);
    } catch {
      // ignore
    }
  };

  ws.onclose = () => {
    setTimeout(() => connectWS(baseUrl, token, onEvent), 2000);
  };

  return ws;
}
