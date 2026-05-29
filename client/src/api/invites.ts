import { request } from "./client";
import type { Invite } from "./types";

export function checkInvite(baseUrl: string, code: string) {
  return request<{ valid: boolean; guest: boolean }>(baseUrl, `/invites/check/${code}`);
}

export function listInvites(baseUrl: string, token: string) {
  return request<Invite[]>(baseUrl, "/invites", token);
}

export function createInvite(baseUrl: string, token: string, data: { max_uses?: number | null; expires_at?: number | null; role_id?: number | null; guest?: boolean }) {
  return request<Invite>(baseUrl, "/invites", token, {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export function deleteInvite(baseUrl: string, token: string, code: string) {
  return request<void>(baseUrl, `/invites/${code}`, token, { method: "DELETE" });
}
