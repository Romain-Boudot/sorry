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
    return res.json() as Promise<{ name: string; description?: string; icon_url?: string }>;
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

  async login(baseUrl: string, username: string, password: string, inviteCode?: string) {
    const hashed = await hashPassword(password);
    return request<{ token: string; user: User }>(baseUrl, "/auth/login", undefined, {
      method: "POST",
      body: JSON.stringify({ username, password: hashed, invite_code: inviteCode }),
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

  async sendMessageWithFiles(baseUrl: string, token: string, channelId: number, content: string, files: File[]) {
    const formData = new FormData();
    formData.append("content", content);
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

  listInvites(baseUrl: string, token: string) {
    return request<Invite[]>(baseUrl, "/invites", token);
  },

  createInvite(baseUrl: string, token: string, data: { max_uses?: number | null; expires_at?: number | null }) {
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

export interface Message {
  id: number;
  channel_id: number;
  author_id: number;
  content: string;
  created_at: string;
  attachments: Attachment[];
}

export interface VoiceUserState {
  muted: boolean;
  deafened: boolean;
  force_muted: boolean;
  force_deafened: boolean;
}

export interface Invite {
  code: string;
  created_by: number;
  max_uses: number | null;
  uses: number;
  expires_at: number | null;
  created_at: number;
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
