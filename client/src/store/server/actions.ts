import { api, resolveBaseUrl } from "../../api";
import { store, persistServers, persistNav, activeState, activeServer } from "../core";
import { connectToServer } from "../../composables/useConnection";
import type { SavedServer } from "../core";

export async function addServer(
  name: string,
  url: string,
  username: string,
  password: string,
  inviteCode?: string,
  displayName?: string,
  defaultAvatar?: File,
  totpCode?: string
) {
  const baseUrl = await resolveBaseUrl(url);

  const duplicate = store.savedServers.find(
    (s) => s.url === baseUrl && s.username === username
  );
  if (duplicate) {
    await connectToServer(duplicate.id);
    store.activeServerId = duplicate.id;
    return;
  }

  const res = await api.login(baseUrl, username, password, inviteCode, totpCode);

  if (res.totp_required) {
    throw new Error("totp_required");
  }

  if (!res.token || !res.user) {
    throw new Error("401");
  }

  if (displayName) {
    await api.updateDisplayName(baseUrl, res.token, displayName);
  }

  if (defaultAvatar) {
    try { await api.uploadAvatar(baseUrl, res.token, defaultAvatar); } catch {}
  }

  const server: SavedServer = {
    id: crypto.randomUUID(),
    name,
    url: baseUrl,
    username,
    token: res.token,
  };

  store.savedServers.push(server);
  persistServers();

  await connectToServer(server.id);
  store.activeServerId = server.id;
}

export async function addServerGuest(
  name: string,
  url: string,
  inviteCode: string,
  displayName: string,
) {
  const baseUrl = await resolveBaseUrl(url);

  const res = await api.quickLogin(baseUrl, inviteCode, displayName);

  if (!res.token || !res.user) {
    throw new Error("403");
  }

  const server: SavedServer = {
    id: crypto.randomUUID(),
    name,
    url: baseUrl,
    username: `guest-${res.user.id}`,
    token: res.token,
  };

  store.savedServers.push(server);
  persistServers();

  await connectToServer(server.id);
  store.activeServerId = server.id;
}

export function switchToServer(serverId: string) {
  store.activeServerId = serverId;
  const state = store.serverStates.get(serverId);
  if (state?.connected) {
    state.unreadCount = 0;
    if (state.activeChannelId) {
      state.channelUnread.delete(state.activeChannelId);
      state.channelMentions.delete(state.activeChannelId);
    }
  } else {
    connectToServer(serverId);
  }
  persistNav();
}
