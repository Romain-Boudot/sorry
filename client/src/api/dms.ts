import { request } from "./client";
import type { DmConversation, DmMessage } from "./types";

export function listDmConversations(baseUrl: string, token: string) {
  return request<DmConversation[]>(baseUrl, "/dms", token);
}

export function listDmMessages(baseUrl: string, token: string, userId: number, limit = 50, before?: number) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", String(before));
  return request<DmMessage[]>(baseUrl, `/dms/${userId}?${params}`, token);
}
