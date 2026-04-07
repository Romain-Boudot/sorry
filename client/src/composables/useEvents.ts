/**
 * WS event handler — dispatches ServerEvents to the correct state mutations.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { type ServerEvent, type Message, type User, type Role, type VoiceUserState } from "../api";
import { store, persistServers, type ServerState } from "../store";
import { setDeafened as voiceSetDeafened, setMuted as voiceSetMuted } from "../voice";
import { fireNotification } from "./useNotifications";

function defaultVoiceUserState(): VoiceUserState {
  return { muted: false, deafened: false, force_muted: false, force_deafened: false, screen_sharing: false, camera_on: false };
}

export function handleEvent(serverId: string, event: ServerEvent) {
  const state = store.serverStates.get(serverId);
  if (!state) return;

  switch (event.type) {
    case "MessageCreate": {
      const msg = event.data as Message;
      const msgs = state.messages.get(msg.channel_id);
      if (msgs) {
        msgs.push(msg);
      } else {
        state.messages.set(msg.channel_id, [msg]);
      }
      const isViewingChannel = store.activeServerId === serverId && state.activeChannelId === msg.channel_id;
      if (!isViewingChannel && msg.author_id !== state.user?.id) {
        state.channelUnread.set(msg.channel_id, (state.channelUnread.get(msg.channel_id) ?? 0) + 1);
      }
      if (store.activeServerId !== serverId) {
        state.unreadCount++;
      }
      fireNotification(store, state, serverId, msg);
      break;
    }
    case "MessageDelete": {
      const { id } = event.data as { id: number };
      for (const [, msgs] of state.messages) {
        const idx = msgs.findIndex((m) => m.id === id);
        if (idx >= 0) { msgs.splice(idx, 1); break; }
      }
      break;
    }
    case "MessageUpdate": {
      const msg = event.data as Message;
      const msgs = state.messages.get(msg.channel_id);
      if (msgs) {
        const idx = msgs.findIndex((m) => m.id === msg.id);
        if (idx >= 0) msgs[idx] = msg;
      }
      break;
    }
    case "UserOnline": {
      const { user } = event.data as { user: User };
      state.onlineUsers.add(user.id);
      state.users.set(user.id, user);
      break;
    }
    case "UserOffline": {
      const { user_id } = event.data as { user_id: number };
      state.onlineUsers.delete(user_id);
      break;
    }
    case "UserJoinedVoice": {
      const { user, channel_id, voice_state: vs } = event.data as { user: User; channel_id: number; voice_state: VoiceUserState };
      if (!state.voiceState.has(channel_id)) {
        state.voiceState.set(channel_id, new Map());
      }
      state.voiceState.get(channel_id)!.set(user.id, vs ?? defaultVoiceUserState());
      break;
    }
    case "UserLeftVoice": {
      const { user_id, channel_id } = event.data as { user_id: number; channel_id: number };
      state.voiceState.get(channel_id)?.delete(user_id);
      break;
    }
    case "VoiceStateUpdate": {
      const { user_id, channel_id, voice_state: vs } = event.data as { user_id: number; channel_id: number; voice_state: VoiceUserState };

      const prevVs = state.voiceState.get(channel_id)?.get(user_id);
      const wasForcedDeaf = prevVs?.force_deafened ?? false;

      if (!state.voiceState.has(channel_id)) {
        state.voiceState.set(channel_id, new Map());
      }
      state.voiceState.get(channel_id)!.set(user_id, vs);

      if (user_id === state.user?.id && state.voiceChannelId) {
        if (vs.force_deafened && !wasForcedDeaf) {
          voiceSetDeafened(true);
        }
        if (!vs.force_deafened && wasForcedDeaf) {
          voiceSetDeafened(false);
        }
      }
      break;
    }
    case "RoleCreate": {
      const role = event.data as Role;
      state.roles.push(role);
      break;
    }
    case "RoleUpdate": {
      const role = event.data as Role;
      const idx = state.roles.findIndex((r) => r.id === role.id);
      if (idx >= 0) state.roles[idx] = role;
      break;
    }
    case "RoleDelete": {
      const { id } = event.data as { id: number };
      state.roles = state.roles.filter((r) => r.id !== id);
      break;
    }
    case "UserRoleUpdate": {
      const { user_id, role_ids, permissions } = event.data as { user_id: number; role_ids: number[]; permissions: number };
      state.userRoles.set(user_id, role_ids);
      if (state.user && user_id === state.user.id) {
        state.permissions = permissions;
      }
      break;
    }
    case "UserUpdate": {
      const user = event.data as User;
      state.users.set(user.id, user);
      if (state.user?.id === user.id) {
        state.user = user;
      }
      break;
    }
    case "ServerUpdate": {
      const { name, icon_url, description } = event.data as { name: string; description: string | null; icon_url: string | null };
      const saved = store.savedServers.find((s) => s.id === serverId);
      if (saved) {
        saved.name = name;
        saved.iconUrl = icon_url;
        saved.description = description;
        persistServers();
      }
      break;
    }
    case "ReactionAdded": {
      const { message_id, channel_id, emoji, user_id } = event.data as { message_id: number; channel_id: number; emoji: string; user_id: number };
      const msgs = state.messages.get(channel_id);
      const msg = msgs?.find((m) => m.id === message_id);
      if (msg) {
        const existing = msg.reactions.find((r) => r.emoji === emoji);
        if (existing) {
          if (!existing.user_ids.includes(user_id)) {
            existing.user_ids.push(user_id);
            existing.count++;
          }
        } else {
          msg.reactions.push({ emoji, count: 1, user_ids: [user_id] });
        }
      }
      break;
    }
    case "ReactionRemoved": {
      const { message_id, channel_id, emoji, user_id } = event.data as { message_id: number; channel_id: number; emoji: string; user_id: number };
      const msgs = state.messages.get(channel_id);
      const msg = msgs?.find((m) => m.id === message_id);
      if (msg) {
        const idx = msg.reactions.findIndex((r) => r.emoji === emoji);
        if (idx >= 0) {
          const r = msg.reactions[idx];
          r.user_ids = r.user_ids.filter((id) => id !== user_id);
          r.count--;
          if (r.count <= 0) msg.reactions.splice(idx, 1);
        }
      }
      break;
    }
  }
}
