import { request } from "./client";
import type { User, BannedUser, Message, AuditLog, MeResponse } from "./types";

export function me(baseUrl: string, token: string) {
  return request<MeResponse>(baseUrl, "/users/me", token);
}

export function updateDisplayName(baseUrl: string, token: string, displayName: string) {
  return request<User>(baseUrl, "/users/me", token, {
    method: "PATCH",
    body: JSON.stringify({ display_name: displayName }),
  });
}

export async function uploadAvatar(baseUrl: string, token: string, file: File) {
  const formData = new FormData();
  formData.append("avatar", file);
  const res = await fetch(`${baseUrl}/api/users/me/avatar`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
    body: formData,
  });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json() as Promise<User>;
}

export async function deleteAvatar(baseUrl: string, token: string) {
  const res = await fetch(`${baseUrl}/api/users/me/avatar`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!res.ok) throw new Error(`${res.status}`);
}

export function banUser(baseUrl: string, token: string, userId: number) {
  return request<void>(baseUrl, `/users/${userId}/ban`, token, { method: "POST" });
}

export function unbanUser(baseUrl: string, token: string, userId: number) {
  return request<void>(baseUrl, `/users/${userId}/ban`, token, { method: "DELETE" });
}

export function listBanned(baseUrl: string, token: string) {
  return request<BannedUser[]>(baseUrl, "/users/banned", token);
}

export function userMessages(baseUrl: string, token: string, userId: number, limit = 50, before?: number) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", String(before));
  return request<Message[]>(baseUrl, `/users/${userId}/messages?${params}`, token);
}

export function userAudit(baseUrl: string, token: string, userId: number, limit = 50, before?: number) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", String(before));
  return request<AuditLog[]>(baseUrl, `/users/${userId}/audit?${params}`, token);
}

export function uploadPublicKey(baseUrl: string, token: string, publicKey: string) {
  return request<void>(baseUrl, "/users/me/key", token, {
    method: "POST",
    body: JSON.stringify({ public_key: publicKey }),
  });
}
