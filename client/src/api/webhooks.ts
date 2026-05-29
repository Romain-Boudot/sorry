import { request } from "./client";
import type { Webhook } from "./types";

export function listWebhooks(baseUrl: string, token: string, channelId: number) {
  return request<Webhook[]>(baseUrl, `/channels/${channelId}/webhooks`, token);
}

export function createWebhook(baseUrl: string, token: string, channelId: number, data: { name: string }) {
  return request<Webhook>(baseUrl, `/channels/${channelId}/webhooks`, token, {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export function updateWebhook(baseUrl: string, token: string, channelId: number, webhookId: number, data: { name?: string }) {
  return request<Webhook>(baseUrl, `/channels/${channelId}/webhooks/${webhookId}`, token, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
}

export function deleteWebhook(baseUrl: string, token: string, channelId: number, webhookId: number) {
  return request<void>(baseUrl, `/channels/${channelId}/webhooks/${webhookId}`, token, { method: "DELETE" });
}

export async function uploadWebhookAvatar(baseUrl: string, token: string, channelId: number, webhookId: number, file: File) {
  const formData = new FormData();
  formData.append("avatar", file);
  const res = await fetch(`${baseUrl}/api/channels/${channelId}/webhooks/${webhookId}/avatar`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
    body: formData,
  });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json() as Promise<Webhook>;
}

export function deleteWebhookAvatar(baseUrl: string, token: string, channelId: number, webhookId: number) {
  return request<void>(baseUrl, `/channels/${channelId}/webhooks/${webhookId}/avatar`, token, { method: "DELETE" });
}
