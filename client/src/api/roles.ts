import { request } from "./client";
import type { Role, ChannelOverwrite } from "./types";

export function getUserRoles(baseUrl: string, token: string, userId: number) {
  return request<Role[]>(baseUrl, `/users/${userId}/roles`, token);
}

export function listRoles(baseUrl: string, token: string) {
  return request<Role[]>(baseUrl, "/roles", token);
}

export function updateRole(baseUrl: string, token: string, roleId: number, data: { name: string; permissions: number; color?: string | null }) {
  return request<void>(baseUrl, `/roles/${roleId}`, token, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

export function reorderRoles(baseUrl: string, token: string, ids: number[]) {
  return request<void>(baseUrl, "/roles/reorder", token, {
    method: "POST",
    body: JSON.stringify({ ids }),
  });
}

export function deleteRole(baseUrl: string, token: string, roleId: number) {
  return request<void>(baseUrl, `/roles/${roleId}`, token, { method: "DELETE" });
}

export function assignRole(baseUrl: string, token: string, roleId: number, userId: number) {
  return request<void>(baseUrl, `/roles/${roleId}/assign`, token, {
    method: "POST",
    body: JSON.stringify({ user_id: userId }),
  });
}

export function removeRole(baseUrl: string, token: string, roleId: number, userId: number) {
  return request<void>(baseUrl, `/roles/${roleId}/remove`, token, {
    method: "POST",
    body: JSON.stringify({ user_id: userId }),
  });
}

export function listOverwrites(baseUrl: string, token: string, channelId: number) {
  return request<ChannelOverwrite[]>(baseUrl, `/channels/${channelId}/overwrites`, token);
}

export function setOverwrite(baseUrl: string, token: string, channelId: number, roleId: number, allow: number, deny: number) {
  return request<void>(baseUrl, `/channels/${channelId}/overwrites`, token, {
    method: "PUT",
    body: JSON.stringify({ role_id: roleId, allow, deny }),
  });
}

export function deleteOverwrite(baseUrl: string, token: string, channelId: number, roleId: number) {
  return request<void>(baseUrl, `/channels/${channelId}/overwrites`, token, {
    method: "DELETE",
    body: JSON.stringify({ role_id: roleId }),
  });
}
