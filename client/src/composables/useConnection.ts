/**
 * Server connection lifecycle: connect, disconnect, mute/unmute, snapshot handling.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { api, resolveBaseUrl, createWsConnection, type Snapshot, type VoiceUserState } from "../api";
import { store, persistServers, persistNav, pendingChannels, createServerState, type SavedServer, type ServerState } from "../store";
import { handleEvent } from "./useEvents";
import { revokeAllOptimisticBlobs } from "./useMessaging";
import { ensureKeypair, loadConversations } from "./useDms";
import { loadFingerprints, saveFingerprints, deleteKeypair, deleteFingerprints } from "../crypto";
import { showToast } from "./useToast";

// ── Token refresh ──

const refreshTimers = new Map<string, ReturnType<typeof setTimeout>>();

function getTokenExp(token: string): number | null {
  try {
    const payload = JSON.parse(atob(token.split(".")[1]));
    return payload.exp ?? null;
  } catch { return null; }
}

/** True si le token JWT est expire (ou illisible). Garde de quelques secondes. */
export function isTokenExpired(token: string): boolean {
  const exp = getTokenExp(token);
  if (!exp) return true;
  return exp <= Math.floor(Date.now() / 1000) + 5;
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
  // Refresh 60s avant expiration ; si moins de 60s (ou deja expire), tenter
  // immediatement — le serveur 401era si le token est mort, ce qu'on gere ci-dessous.
  const refreshIn = Math.max(0, (remaining - 60) * 1000);

  const timer = setTimeout(async () => {
    try {
      const res = await api.refreshToken(server.url, server.token);
      server.token = res.token;
      persistServers();
      scheduleTokenRefresh(serverId);
    } catch (e) {
      // 401 = token deja expire/invalide cote serveur : /auth/refresh exige un JWT valide,
      // donc inutile de retenter — l'utilisateur devra se reconnecter.
      // Autre erreur (reseau, serveur down) : retry dans 30s.
      if (e instanceof Error && e.message === "401") return;
      const retry = setTimeout(() => scheduleTokenRefresh(serverId), 30_000);
      refreshTimers.set(serverId, retry);
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
  state.channelOverwrites = snapshot.channel_overwrites ?? [];
  state.webhooks.clear();
  for (const wh of snapshot.webhooks ?? []) {
    state.webhooks.set(wh.id, wh);
  }
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

  // Hydrate les fingerprints persistées (TOFU baseline), puis confronte au snapshot :
  // si la fingerprint serveur diffère de ce qu'on connaissait, on alerte (rotation/MITM potentiel).
  // Pour les peers jamais vus, on trust-on-first-use.
  state.knownFingerprints = loadFingerprints(serverId);
  const myId = snapshot.user.id;
  for (const u of snapshot.users) {
    if (!u.key_fingerprint) continue;
    if (u.id === myId) continue; // on ne TOFU-check pas soi-même
    const known = state.knownFingerprints.get(u.id);
    if (known && known !== u.key_fingerprint) {
      showToast(
        `La cle DM de ${u.display_name} a change. Verifiez son empreinte avant de lui ecrire.`,
        "warning",
        8000,
      );
    }
    state.knownFingerprints.set(u.id, u.key_fingerprint);
  }
  saveFingerprints(serverId, state.knownFingerprints);

  // Init DM keypair + chargement des conversations (non bloquant).
  if (server) {
    ensureKeypair(server, state)
      .then(() => loadConversations(server, state))
      .catch((err) => {
        console.warn("[DM] Keypair init failed", err);
      });
  }

  // Restore pending channel from nav, or select first text channel
  if (!state.activeChannelId) {
    const pending = pendingChannels.get(serverId);
    const channelExists = pending && snapshot.channels.some((c) => c.id === pending);
    const targetChannel = channelExists ? pending : snapshot.channels.find((c) => c.kind === "text")?.id;
    pendingChannels.delete(serverId);

    if (targetChannel) {
      state.activeChannelId = targetChannel;
      if (server) {
        api.listMessages(server.url, server.token, targetChannel)
          .then((msgs) => {
            const s = store.serverStates.get(serverId);
            if (s) s.messages.set(targetChannel, msgs.reverse());
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
  if (existing?.connected || existing?.wsConnection) {
    store.activeServerId = serverId;
    return;
  }

  // Token mort : inutile de tenter le WS (boucle de reconnexion infinie sur 401).
  // On declenche le prompt de re-login a la place.
  if (isTokenExpired(server.token)) {
    if (!store.serverStates.has(serverId)) {
      store.serverStates.set(serverId, createServerState());
    }
    store.reauthServerId = serverId;
    showToast(`Session expiree sur ${server.name}, reconnecte-toi.`, "warning", 5000);
    return;
  }

  store.serverStates.set(serverId, createServerState());
  // Always use the proxy reference so Vue reactivity tracks mutations
  const state = store.serverStates.get(serverId)!;

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
        const s = store.serverStates.get(serverId);
        if (!s) return;
        s.wsState = wsState;
        if (wsState === "connected") {
          s.connected = true;
        } else if (wsState === "disconnected") {
          s.connected = false;
        }
      },
    });

    const timeout = new Promise<void>((_, reject) =>
      setTimeout(() => reject(new Error("snapshot timeout")), 15000)
    );
    await Promise.race([ready, timeout]);

    store.activeServerId = serverId;
    persistNav();
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
  if (state) {
    for (const msgs of state.messages.values()) revokeAllOptimisticBlobs(msgs);
  }
  store.serverStates.delete(serverId);
  store.savedServers = store.savedServers.filter((s) => s.id !== serverId);
  // Cleanup du matériel cryptographique DM (keypair locale + TOFU fingerprints).
  deleteKeypair(serverId);
  deleteFingerprints(serverId);
  persistServers();

  if (store.activeServerId === serverId) {
    store.activeServerId = store.savedServers[0]?.id ?? null;
  }
}
