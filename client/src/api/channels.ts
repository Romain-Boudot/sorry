import { request } from "./client";
import type { Channel, ChannelGroup, ChannelAttachment, Message } from "./types";

export function listChannels(baseUrl: string, token: string) {
  return request<Channel[]>(baseUrl, "/channels", token);
}

export function createChannel(baseUrl: string, token: string, name: string, kind: "text" | "voice", groupId?: number) {
  return request<Channel>(baseUrl, "/channels", token, {
    method: "POST",
    body: JSON.stringify({ name, kind, group_id: groupId ?? null }),
  });
}

export function listGroups(baseUrl: string, token: string) {
  return request<ChannelGroup[]>(baseUrl, "/channels/groups", token);
}

export function createGroup(baseUrl: string, token: string, name: string) {
  return request<ChannelGroup>(baseUrl, "/channels/groups", token, {
    method: "POST",
    body: JSON.stringify({ name }),
  });
}

export function updateGroup(baseUrl: string, token: string, id: number, name: string) {
  return request<void>(baseUrl, `/channels/groups/${id}`, token, {
    method: "PATCH",
    body: JSON.stringify({ name }),
  });
}

export function deleteGroup(baseUrl: string, token: string, id: number) {
  return request<void>(baseUrl, `/channels/groups/${id}`, token, { method: "DELETE" });
}

export function reorderChannels(baseUrl: string, token: string, ids: number[]) {
  return request<void>(baseUrl, "/channels/reorder", token, {
    method: "POST",
    body: JSON.stringify({ ids }),
  });
}

export function reorderGroups(baseUrl: string, token: string, ids: number[]) {
  return request<void>(baseUrl, "/channels/groups/reorder", token, {
    method: "POST",
    body: JSON.stringify({ ids }),
  });
}

export function deleteChannel(baseUrl: string, token: string, channelId: number) {
  return request<void>(baseUrl, `/channels/${channelId}`, token, { method: "DELETE" });
}

export function updateChannel(baseUrl: string, token: string, channelId: number, data: { name?: string; description?: string; user_limit?: number | null }) {
  return request<Channel>(baseUrl, `/channels/${channelId}`, token, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
}

export function moveChannel(baseUrl: string, token: string, channelId: number, groupId: number | null) {
  return request<void>(baseUrl, `/channels/${channelId}/group`, token, {
    method: "PATCH",
    body: JSON.stringify({ group_id: groupId }),
  });
}

export function togglePin(baseUrl: string, token: string, channelId: number, messageId: number) {
  return request<void>(baseUrl, `/channels/${channelId}/messages/${messageId}/pin`, token, {
    method: "POST",
  });
}

export function listPinned(baseUrl: string, token: string, channelId: number) {
  return request<Message[]>(baseUrl, `/channels/${channelId}/pins`, token);
}

export function listMessages(baseUrl: string, token: string, channelId: number, limit = 50, before?: number) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", String(before));
  return request<Message[]>(baseUrl, `/channels/${channelId}/messages?${params}`, token);
}

export function searchMessages(baseUrl: string, token: string, channelId: number, query: string, limit = 25) {
  const params = new URLSearchParams({ q: query, limit: String(limit) });
  return request<Message[]>(baseUrl, `/channels/${channelId}/search?${params}`, token);
}

export function sendMessage(baseUrl: string, token: string, channelId: number, content: string, replyToId?: number, nonce?: string) {
  return request<Message>(baseUrl, `/channels/${channelId}/messages`, token, {
    method: "POST",
    body: JSON.stringify({ content, reply_to_id: replyToId, nonce }),
  });
}

export async function sendMessageWithFiles(baseUrl: string, token: string, channelId: number, content: string, files: File[], replyToId?: number, nonce?: string) {
  const formData = new FormData();
  formData.append("content", content);
  if (replyToId) formData.append("reply_to_id", String(replyToId));
  if (nonce) formData.append("nonce", nonce);
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
}

export function channelAttachments(baseUrl: string, token: string, channelId: number, limit = 50, before?: number) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", String(before));
  return request<ChannelAttachment[]>(baseUrl, `/channels/${channelId}/attachments?${params}`, token);
}
