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

  login(baseUrl: string, username: string, password: string, serverPassword?: string) {
    return request<{ token: string; user: User }>(baseUrl, "/auth/login", undefined, {
      method: "POST",
      body: JSON.stringify({ username, password, server_password: serverPassword }),
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
};

// Types
export interface User {
  id: number;
  username: string;
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

export interface Message {
  id: number;
  channel_id: number;
  author_id: number;
  content: string;
  created_at: string;
}

export interface MeResponse {
  user: User;
  permissions: number;
  users: User[];
  online_users: number[];
  voice_state: Record<number, number[]>;
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
