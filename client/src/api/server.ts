import { request } from "./client";
import type { ServerStats, AuditLog, ServerLogEntry } from "./types";

export async function serverInfo(baseUrl: string) {
  const res = await fetch(`${baseUrl}/info`);
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json() as Promise<{ name: string; description?: string; icon_url?: string }>;
}

export function serverStats(baseUrl: string, token: string) {
  return request<ServerStats>(baseUrl, "/server/stats", token);
}

export function updateServer(baseUrl: string, token: string, data: { name?: string; description?: string }) {
  return request<void>(baseUrl, "/server", token, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
}

export async function uploadServerIcon(baseUrl: string, token: string, file: File) {
  const formData = new FormData();
  formData.append("icon", file);
  const res = await fetch(`${baseUrl}/api/server/icon`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
    body: formData,
  });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json() as Promise<{ icon_url: string }>;
}

export async function deleteServerIcon(baseUrl: string, token: string) {
  const res = await fetch(`${baseUrl}/api/server/icon`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!res.ok) throw new Error(`${res.status}`);
}

export function fetchOg(baseUrl: string, token: string, url: string) {
  return request<{ title?: string; description?: string; image?: string; site_name?: string; url: string }>(baseUrl, "/og", token, {
    method: "POST",
    body: JSON.stringify({ url }),
  });
}

export function listAudit(baseUrl: string, token: string, limit = 50, before?: number) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", String(before));
  return request<AuditLog[]>(baseUrl, `/audit?${params}`, token);
}

export function listServerLogs(baseUrl: string, token: string, limit = 200) {
  const params = new URLSearchParams({ limit: String(limit) });
  return request<ServerLogEntry[]>(baseUrl, `/audit/server?${params}`, token);
}
