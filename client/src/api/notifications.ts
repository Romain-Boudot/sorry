import { request } from "./client";
import type { NotificationPref } from "./types";

export function getNotificationPrefs(baseUrl: string, token: string) {
  return request<NotificationPref[]>(baseUrl, "/notifications/preferences", token);
}

export function setNotificationPref(baseUrl: string, token: string, data: { scope: string; target_id: number; level: string; mute_until?: string | null }) {
  return request<void>(baseUrl, "/notifications/preferences", token, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

export function deleteNotificationPref(baseUrl: string, token: string, scope: string, targetId: number) {
  return request<void>(baseUrl, "/notifications/preferences", token, {
    method: "DELETE",
    body: JSON.stringify({ scope, target_id: targetId }),
  });
}
