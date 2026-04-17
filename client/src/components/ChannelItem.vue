<template>
  <div class="channel-item-wrapper">
    <div
      v-if="channel.kind === 'text'"
      class="channel-item"
      :class="{ active: channel.id === state?.activeChannelId, 'is-muted': muteLevel !== 'all' }"
      :data-channel-id="channel.id"
      @click="selectChannel(channel.id)"
      @contextmenu.prevent.stop="emit('contextmenu', $event)"
    >
      <Hash class="channel-icon" :size="20" />
      <span class="channel-name" :class="{ muted: muteLevel !== 'all' }">{{ channel.name }}</span>
      <span v-if="mentionCount > 0" class="mention-dot"></span>
      <span v-else-if="unreadCount > 0" class="unread-dot"></span>
      <span class="channel-actions"><slot name="actions" /></span>
      <BellOff v-if="muteLevel === 'nothing'" class="mute-icon" :size="14" />
      <BellMinus v-else-if="muteLevel === 'mentions'" class="mute-icon" :size="14" />
    </div>
    <div
      v-else
      class="channel-item voice"
      :class="{
        active: channel.id === state?.activeChannelId,
        joined: state?.voiceChannelId === channel.id,
        'drop-target': dropHighlight,
      }"
      :data-channel-id="channel.id"
      @click="handleVoiceClick(channel.id)"
      @contextmenu.prevent.stop="emit('contextmenu', $event)"
      @dragenter.prevent="onDragEnter"
      @dragleave.prevent="onDragLeave"
      @dragover.prevent
      @drop.prevent="dropCount = 0; onChannelDrop($event)"
    >
      <Lock v-if="isHiddenChannel" class="channel-icon" :size="20" />
      <Volume2 v-else class="channel-icon" :size="20" />
      <span class="channel-name">{{ channel.name }}<span v-if="channel.user_limit" class="voice-count" :class="{ full: voiceUsers.length >= channel.user_limit }"> {{ voiceUsers.length }}/{{ channel.user_limit }}</span><span v-else-if="voiceUsers.length" class="voice-count"> {{ voiceUsers.length }}</span></span>
      <span class="channel-actions"><slot name="actions" /></span>
    </div>
    <div v-if="channel.kind === 'voice' && voiceUsers.length" class="voice-users">
      <div
        v-for="[uid, vs] in voiceUsers"
        :key="uid"
        class="voice-user"
        :draggable="canMove"
        @dragstart.stop="onUserDragStart(uid, $event)"
        @dragend="onUserDragEnd"
        @click="openCard(uid, $event)"
        @contextmenu.prevent.stop="onVoiceUserContext(uid, vs, $event)"
      >
        <div class="voice-avatar" :class="{ speaking: !vs.muted && !vs.deafened && isUserSpeaking(uid) }">
          <img v-if="resolveAvatarUrl(uid)" :src="resolveAvatarUrl(uid)!" />
          <span v-else>{{ resolveUser(uid)[0]?.toUpperCase() }}</span>
        </div>
        <span class="voice-user-name">{{ resolveUser(uid) }}</span>
        <MicOff v-if="vs.muted || vs.force_muted" class="voice-status-icon" :class="{ forced: vs.force_muted }" :size="12" />
        <Headphones v-if="vs.deafened || vs.force_deafened" class="voice-status-icon" :class="{ forced: vs.force_deafened }" :size="12" />
        <Monitor v-if="vs.screen_sharing" class="voice-status-icon streaming" :size="12" />
        <Video v-if="vs.camera_on" class="voice-status-icon streaming" :size="12" />
      </div>
    </div>

    <UserCard
      v-if="cardUser"
      :user="cardUser"
      :x="cardX"
      :y="cardY"
      @close="cardUser = null"
    />

    <ContextMenu
      v-if="voiceCtx"
      :x="voiceCtx.x"
      :y="voiceCtx.y"
      :items="voiceCtx.items"
      @close="voiceCtx = null"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Hash, Volume2, MicOff, Headphones, HeadphoneOff, PhoneOff, BellOff, BellMinus, Monitor, Video, Lock } from "lucide-vue-next";
import { activeState, activeServer, selectChannel, resolveUser, joinVoiceChannel, isUserSpeaking, resolveAvatarUrl } from "../store";
import * as perms from "../permissions";
import type { Channel, User, VoiceUserState } from "../api";
import ContextMenu, { type MenuItem } from "./ui/ContextMenu.vue";
import UserCard from "./UserCard.vue";

const props = defineProps<{ channel: Channel }>();
const emit = defineEmits<{ contextmenu: [e: MouseEvent] }>();

const state = computed(() => activeState());
const server = computed(() => activeServer());
const canMove = computed(() => perms.has(state.value?.permissions ?? 0, perms.MOVE_MEMBERS));

const isHiddenChannel = computed(() => {
  const st = state.value;
  if (!st?.user || props.channel.kind !== "voice") return false;
  if (perms.has(st.permissions, perms.ADMINISTRATOR)) return false;
  const userRoleIds = st.userRoles.get(st.user.id) ?? [];
  const channelOws = st.channelOverwrites.filter(o => o.channel_id === props.channel.id);
  if (channelOws.length === 0) return !perms.has(st.permissions, perms.VIEW_CHANNELS);
  const channelPerms = perms.computeChannel(userRoleIds, st.roles, channelOws);
  return !perms.has(channelPerms, perms.VIEW_CHANNELS);
});

const dropCount = ref(0);
const dropHighlight = computed(() => dropCount.value > 0);

const unreadCount = computed(() => state.value?.channelUnread.get(props.channel.id) ?? 0);
const mentionCount = computed(() => state.value?.channelMentions.get(props.channel.id) ?? 0);

const muteLevel = computed(() => {
  const st = state.value;
  if (!st) return "all";
  const now = new Date().toISOString();
  const pref = st.notificationPrefs.find(
    (p) => p.scope === "channel" && p.target_id === props.channel.id
  );
  if (pref && (!pref.mute_until || pref.mute_until >= now)) return pref.level;
  return "all";
});

const voiceUsers = computed((): [number, VoiceUserState][] => {
  const map = state.value?.voiceState.get(props.channel.id);
  return map ? [...map.entries()] : [];
});

function handleVoiceClick(channelId: number) {
  if (state.value?.voiceChannelId === channelId) {
    state.value.activeChannelId = channelId;
  } else {
    joinVoiceChannel(channelId);
  }
}

const cardUser = ref<User | null>(null);
const cardX = ref(0);
const cardY = ref(0);

function openCard(uid: number, e: MouseEvent) {
  const user = state.value?.users.get(uid);
  if (!user) return;
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  cardX.value = rect.right + 8;
  cardY.value = rect.top;
  cardUser.value = user;
}

function isVoiceUserDrag(e: DragEvent): boolean {
  return e.dataTransfer?.types.includes("voice-user-id") ?? false;
}

function onDragEnter(e: DragEvent) {
  if (isVoiceUserDrag(e)) dropCount.value++;
}

function onDragLeave(e: DragEvent) {
  if (isVoiceUserDrag(e)) dropCount.value--;
}

function onUserDragStart(uid: number, e: DragEvent) {
  e.dataTransfer!.setData("voice-user-id", String(uid));
  e.dataTransfer!.setData("voice-from-channel", String(props.channel.id));
  e.dataTransfer!.effectAllowed = "move";
}

function onUserDragEnd() {
  // Reset all voice channel drop highlights via DOM (other channels won't get dragend)
}

function onChannelDrop(e: DragEvent) {
  dropCount.value = 0;
  const st = state.value;
  if (!st) return;
  const uid = Number(e.dataTransfer?.getData("voice-user-id"));
  const fromChannel = Number(e.dataTransfer?.getData("voice-from-channel"));
  if (!uid || !fromChannel || fromChannel === props.channel.id) return;
  st.wsConnection?.ws?.send(JSON.stringify({ type: "MoveVoice", data: { user_id: uid, channel_id: props.channel.id } }));
}

const voiceCtx = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);

function onVoiceUserContext(uid: number, vs: VoiceUserState, e: MouseEvent) {
  const s = server.value;
  const st = state.value;
  if (!s || !st || uid === st.user?.id) return;

  const items: MenuItem[] = [];
  const canMute = perms.has(st.permissions, perms.MUTE_MEMBERS);
  const canDeafen = perms.has(st.permissions, perms.DEAFEN_MEMBERS);

  if (canMute) {
    items.push({
      label: vs.force_muted ? "Unmute" : "Mute",
      icon: vs.force_muted ? Volume2 : MicOff,
      action: () => {
        st.wsConnection?.ws?.send(JSON.stringify({ type: "ForceMute", data: { user_id: uid, muted: !vs.force_muted } }));
      },
    });
  }

  if (canDeafen) {
    items.push({
      label: vs.force_deafened ? "Undeafen" : "Deafen",
      icon: vs.force_deafened ? Headphones : HeadphoneOff,
      action: () => {
        st.wsConnection?.ws?.send(JSON.stringify({ type: "ForceDeafen", data: { user_id: uid, deafened: !vs.force_deafened } }));
      },
    });
  }

  const canMove = perms.has(st.permissions, perms.MOVE_MEMBERS);
  if (canMove) {
    items.push({
      label: "Deconnecter",
      icon: PhoneOff,
      danger: true,
      action: () => {
        st.wsConnection?.ws?.send(JSON.stringify({ type: "KickVoice", data: { user_id: uid } }));
      },
    });
  }

  if (items.length) {
    voiceCtx.value = { x: e.clientX, y: e.clientY, items };
  }
}
</script>

<style scoped>
.channel-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  margin: 3px 8px;
  cursor: pointer;
  color: var(--text-muted);
  border-radius: 8px;
  user-select: none;
  font-size: 0.9375rem;
  font-weight: 500;
  transition: background 0.1s, color 0.1s;
}

.channel-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.channel-item.active {
  background: var(--bg-modifier-active);
  color: var(--header-primary);
}

.channel-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.channel-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  opacity: 0;
  transition: opacity 0.1s;
}

.channel-item:hover .channel-actions { opacity: 1; }

.channel-icon {
  color: var(--text-faint);
  width: 20px;
  flex-shrink: 0;
}

.channel-item.active .channel-icon {
  color: var(--text-normal);
}

.channel-item.joined .channel-icon {
  color: var(--green);
}

.voice-users {
  padding: 0 12px 0 20px;
}

.voice-users.has-users {
  padding-bottom: 2px;
}

/* Drag & drop voice users */
.channel-item.voice.drop-target {
  background: var(--bg-modifier-hover);
  outline: 2px solid var(--accent);
  outline-offset: -2px;
  border-radius: 6px;
}

.voice-user {
  font-size: 0.8125rem;
  color: var(--text-muted);
  padding: 4px 8px;
  margin: 1px 0;
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.1s, color 0.1s;
}
.voice-user:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.voice-user-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.voice-avatar {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
  font-size: 0.5625rem;
  font-weight: 700;
  color: var(--text-faint);
  transition: box-shadow 0.15s;
}

.voice-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.voice-avatar.speaking {
  box-shadow: 0 0 0 2px var(--green);
}

.voice-status-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}

.voice-status-icon.forced {
  color: var(--danger);
}

.voice-status-icon.streaming {
  color: var(--accent);
}

.channel-name.muted {
  color: var(--text-faint);
}

.channel-item.is-muted .channel-icon {
  color: var(--text-faint) !important;
}

.mute-icon {
  width: 24px;
  height: 24px;
  padding: 5px;
  color: var(--text-faint);
  flex-shrink: 0;
}

.unread-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--header-primary);
  flex-shrink: 0;
}

.mention-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--danger);
  flex-shrink: 0;
}

.voice-count {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-faint);
  margin-left: 6px;
}

.voice-count.full {
  color: var(--danger);
}
</style>
