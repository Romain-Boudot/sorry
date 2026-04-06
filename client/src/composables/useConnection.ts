/**
 * Server connection lifecycle: connect, disconnect, mute/unmute, snapshot handling.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { api, resolveBaseUrl, createWsConnection, type Snapshot, type VoiceUserState } from "../api";
import { store, persistServers, createServerState, type SavedServer, type ServerState } from "../store";
import { handleEvent } from "./useEvents";

// ── Token refresh ──

const refreshTimers = new Map<string, ReturnType<typeof setTimeout>>();

function getTokenExp(token: string): number | null {
  try {
    const payload = JSON.parse(atob(token.split(".")[1]));
    return payload.exp ?? null;
  } catch { return null; }
}

function scheduleTokenRefresh(serverId: string) {
  const existing = refreshTimers.get(serverId);
  if (existing) clearTimeout(existing);

  const server = store.savedServers.find((s) => s.id === serverId);
  if (!server) return;

  const exp = getTokenExp(server.token);
  if (!exp) return;

  const nowSecs = Math.floor(Date.now() / 1000);
  const remaining = exp - nowSecs;
  const refreshIn = Math.max(remaining * 0.8, 60) * 1000;

  const timer = setTimeout(async () => {
    try {
      const res = await api.refreshToken(server.url, server.token);
      server.token = res.token;
      persistServers();
      scheduleTokenRefresh(serverId);
    } catch {
      // Token expired or server unreachable
    }
  }, refreshIn);

  refreshTimers.set(serverId, timer);
}

// ── Snapshot ──

function applySnapshot(serverId: string, snapshot: Snapshot) {
  const state = store.serverStates.get(serverId);
  if (!state) return;
  const server = store.savedServers.find((s) => s.id === serverId);

  state.user = snapshot.user;
  state.permissions = snapshot.permissions;
  state.groups = snapshot.groups;
  state.channels = snapshot.channels;
  state.connected = true;
  state.onlineUsers = new Set(snapshot.online_users);
  state.onlineUsers.add(snapshot.user.id);
  state.roles = snapshot.roles;
  state.maxFileSize = snapshot.max_file_size;

  state.users.clear();
  for (const u of snapshot.users) {
    state.users.set(u.id, u);
  }

  state.userRoles.clear();
  for (const [uid, rids] of Object.entries(snapshot.user_roles)) {
    state.userRoles.set(Number(uid), rids as number[]);
  }

  state.voiceState.clear();
  for (const [chId, usersObj] of Object.entries(snapshot.voice_state)) {
    const map = new Map<number, VoiceUserState>();
    for (const [uid, vs] of Object.entries(usersObj)) {
      map.set(Number(uid), vs as VoiceUserState);
    }
    state.voiceState.set(Number(chId), map);
  }

  if (server) {
    server.name = snapshot.server_name;
    server.iconUrl = snapshot.server_icon_url ?? null;
    server.description = snapshot.server_description ?? null;
    persistServers();
  }

  // Reload notification prefs (not in snapshot)
  if (server) {
    api.getNotificationPrefs(server.url, server.token)
      .then((prefs) => {
        const s = store.serverStates.get(serverId);
        if (s) s.notificationPrefs = prefs;
      })
      .catch(() => {});
  }

  // Select first text channel if none selected
  if (!state.activeChannelId) {
    const firstText = snapshot.channels.find((c) => c.kind === "text");
    if (firstText) {
      state.activeChannelId = firstText.id;
      if (server) {
        api.listMessages(server.url, server.token, firstText.id)
          .then((msgs) => {
            const s = store.serverStates.get(serverId);
            if (s) s.messages.set(firstText.id, msgs.reverse());
          })
          .catch(() => {});
      }
    }
  }
}

// ── Connect ──

export async function connectToServer(serverId: string) {
  const server = store.savedServers.find((s) => s.id === serverId);
  if (!server) return;

  const existing = store.serverStates.get(serverId);
  if (existing?.connected) {
    store.activeServerId = serverId;
    return;
  }

  const state = createServerState();
  store.serverStates.set(serverId, state);

  try {
    const resolvedUrl = await resolveBaseUrl(server.url);
    if (resolvedUrl !== server.url) {
      server.url = resolvedUrl;
      persistServers();
    }

    scheduleTokenRefresh(serverId);

    if (Notification.permission === "default") {
      Notification.requestPermission();
    }

    let firstSnapshotReceived = false;
    let resolveReady!: () => void;
    const ready = new Promise<void>((r) => { resolveReady = r; });

    state.wsConnection = createWsConnection(server.url, () => server.token, {
      onSnapshot: (snapshot) => {
        applySnapshot(serverId, snapshot);
        if (!firstSnapshotReceived) {
          firstSnapshotReceived = true;
          resolveReady();
        }
      },
      onEvent: (event) => {
        handleEvent(serverId, event);
      },
      onStateChange: (wsState) => {
        state.wsState = wsState;
        if (wsState === "connected") {
          state.connected = true;
        } else if (wsState === "disconnected") {
          state.connected = false;
        }
      },
    });

    const timeout = new Promise<void>((_, reject) =>
      setTimeout(() => reject(new Error("snapshot timeout")), 15000)
    );
    await Promise.race([ready, timeout]);

    store.activeServerId = serverId;
  } catch {
    state.connected = false;
    state.wsConnection?.destroy();
    state.wsConnection = null;
  }
}

export async function connectAll() {
  const promises = store.savedServers.map((server) => {
    if (server.autoConnect === false) return Promise.resolve();
    const state = store.serverStates.get(server.id);
    if (state?.muted) return Promise.resolve();
    return connectToServer(server.id).catch(() => {});
  });
  await Promise.all(promises);
}

export function muteServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  if (state) {
    state.wsConnection?.destroy();
    state.wsConnection = null;
    state.connected = false;
    state.muted = true;
  }

  if (store.activeServerId === serverId) {
    const next = store.savedServers.find(
      (s) => s.id !== serverId && store.serverStates.get(s.id)?.connected
    );
    store.activeServerId = next?.id ?? null;
  }
}

export function unmuteServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  if (state) {
    state.muted = false;
  }
  connectToServer(serverId);
}

export function removeServer(serverId: string) {
  const state = store.serverStates.get(serverId);
  state?.wsConnection?.destroy();
  store.serverStates.delete(serverId);
  store.savedServers = store.savedServers.filter((s) => s.id !== serverId);
  persistServers();

  if (store.activeServerId === serverId) {
    store.activeServerId = store.savedServers[0]?.id ?? null;
  }
}
