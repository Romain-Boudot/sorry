import { activeState, activeServer } from "../core";
import * as _notifs from "../../composables/useNotifications";

export async function setNotificationPref(
  scope: "channel" | "server",
  targetId: number,
  level: "all" | "mentions" | "nothing",
  muteUntil?: string | null,
) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _notifs.setNotificationPref(server, state, scope, targetId, level, muteUntil);
}

export async function removeNotificationPref(scope: "channel" | "server", targetId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _notifs.removeNotificationPref(server, state, scope, targetId);
}
