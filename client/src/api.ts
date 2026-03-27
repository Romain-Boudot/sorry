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

  createChannel(baseUrl: string, token: string, name: string, kind: "text" | "voice") {
    return request<Channel>(baseUrl, "/channels", token, {
      method: "POST",
      body: JSON.stringify({ name, kind }),
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
};

// Types
export interface User {
  id: number;
  username: string;
  display_name: string;
}

export interface Channel {
  id: number;
  name: string;
  kind: "text" | "voice";
  position: number;
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
