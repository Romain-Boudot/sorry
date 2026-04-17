/**
 * Voice channel operations: join, leave, mute, deafen, screen share, camera.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { api } from "../api";
import {
  joinVoice, leaveVoice,
  toggleMute as voiceToggleMute, toggleDeafen as voiceToggleDeafen,
  setDeafened as voiceSetDeafened,
  startScreenShare as voiceStartScreenShare, stopScreenShare as voiceStopScreenShare,
  setCameraEnabled as voiceSetCamera,
  isScreenSharing as voiceIsScreenSharing,
  isCameraEnabled as voiceIsCameraEnabled,
} from "../voice";
import type { ServerState, SavedServer } from "../store";
import { wsSend, sendVoiceStateUpdate } from "./helpers";
import { showToast } from "./useToast";

export async function joinVoiceChannel(
  server: SavedServer,
  state: ServerState,
  channelId: number,
) {
  state.voiceStatus = "connecting";
  state.voiceConnectingChannelId = channelId;

  let token: string;
  let url: string;
  try {
    const resp = await api.getLivekitToken(server.url, server.token, channelId);
    token = resp.token;
    url = resp.url;
  } catch (e: any) {
    state.voiceStatus = "idle";
    state.voiceConnectingChannelId = null;
    if (e?.message === "403") {
      showToast("Channel plein", "error", 3000);
    } else {
      showToast("Erreur de connexion au vocal", "error", 3000);
    }
    return;
  }

  try {
    await joinVoice(url, token, {
      onConnected: () => {
        state.voiceChannelId = channelId;
        state.voiceStatus = "connected";
        if (state.isMuted) voiceToggleMute();
        if (state.isDeafened) voiceToggleDeafen();
        wsSend(state, { type: "JoinVoice", data: { channel_id: channelId } });
        sendVoiceStateUpdate(state);
      },
      onDisconnected: () => {
        const prevChannel = state.voiceChannelId;
        if (!prevChannel) return;
        state.voiceChannelId = null;
        state.voiceConnectingChannelId = null;
        state.voiceStatus = "idle";
        wsSend(state, { type: "LeaveVoice", data: { channel_id: prevChannel } });
      },
      onParticipantJoined: () => {},
      onParticipantLeft: () => {},
      onActiveSpeakersChanged: (identities) => {
        state.speakingUsers = new Set(identities);
      },
      onTrackChanged: () => {
        state.videoTrackVersion++;
        // Sync state with actual LiveKit track state (handles browser-native stop sharing, etc.)
        const actualScreen = voiceIsScreenSharing();
        const actualCamera = voiceIsCameraEnabled();
        if (state.isScreenSharing !== actualScreen || state.isCameraOn !== actualCamera) {
          state.isScreenSharing = actualScreen;
          state.isCameraOn = actualCamera;
          sendVoiceStateUpdate(state);
        }
      },
      onError: (err) => {
        state.voiceStatus = "error";
        console.error("Voice error:", err);
      },
    });
  } catch {
    state.voiceStatus = "idle";
    state.voiceConnectingChannelId = null;
    showToast("Erreur de connexion au vocal", "error", 3000);
  }
}

/** Rejoin a voice channel with a pre-generated token (used for admin move). */
export async function rejoinWithToken(
  state: ServerState,
  channelId: number,
  token: string,
  url: string,
) {
  // Leave current channel first
  const prevChannel = state.voiceChannelId;
  state.voiceChannelId = null;
  state.voiceConnectingChannelId = null;
  state.isScreenSharing = false;
  state.isCameraOn = false;
  await leaveVoice();
  if (prevChannel) {
    wsSend(state, { type: "LeaveVoice", data: { channel_id: prevChannel } });
  }

  state.voiceStatus = "connecting";
  state.voiceConnectingChannelId = channelId;

  try {
    await joinVoice(url, token, {
      onConnected: () => {
        state.voiceChannelId = channelId;
        state.voiceStatus = "connected";
        if (state.isMuted) voiceToggleMute();
        if (state.isDeafened) voiceToggleDeafen();
        wsSend(state, { type: "JoinVoice", data: { channel_id: channelId } });
        sendVoiceStateUpdate(state);
      },
      onDisconnected: () => {
        const prev = state.voiceChannelId;
        if (!prev) return;
        state.voiceChannelId = null;
        state.voiceConnectingChannelId = null;
        state.voiceStatus = "idle";
        wsSend(state, { type: "LeaveVoice", data: { channel_id: prev } });
      },
      onParticipantJoined: () => {},
      onParticipantLeft: () => {},
      onActiveSpeakersChanged: (identities) => {
        state.speakingUsers = new Set(identities);
      },
      onTrackChanged: () => {
        state.videoTrackVersion++;
        const actualScreen = voiceIsScreenSharing();
        const actualCamera = voiceIsCameraEnabled();
        if (state.isScreenSharing !== actualScreen || state.isCameraOn !== actualCamera) {
          state.isScreenSharing = actualScreen;
          state.isCameraOn = actualCamera;
          sendVoiceStateUpdate(state);
        }
      },
      onError: (err) => {
        state.voiceStatus = "error";
        console.error("Voice move error:", err);
      },
    });
  } catch {
    state.voiceStatus = "error";
  }
}

export async function leaveVoiceChannel(state: ServerState) {
  const prevChannel = state.voiceChannelId;

  // Reset state BEFORE disconnect to avoid race with onDisconnected callback
  state.voiceChannelId = null;
  state.voiceConnectingChannelId = null;
  state.voiceStatus = "idle";
  state.isScreenSharing = false;
  state.isCameraOn = false;

  await leaveVoice();

  if (prevChannel) {
    wsSend(state, { type: "LeaveVoice", data: { channel_id: prevChannel } });
  }
}

export function toggleMute(state: ServerState) {
  if (state.voiceChannelId) {
    const micEnabled = voiceToggleMute();
    state.isMuted = !micEnabled;
  } else {
    state.isMuted = !state.isMuted;
  }
  if (!state.isMuted && state.isDeafened) {
    state.isDeafened = false;
    if (state.voiceChannelId) voiceToggleDeafen();
  }
  sendVoiceStateUpdate(state);
}

export function toggleDeafen(state: ServerState) {
  const wasDeafened = state.isDeafened;

  if (!wasDeafened) {
    // Becoming deafened
    state.wasMutedBeforeDeafen = state.isMuted;
    state.isDeafened = true;
    if (!state.isMuted) {
      state.isMuted = true;
      if (state.voiceChannelId) voiceToggleMute();
    }
    if (state.voiceChannelId) voiceSetDeafened(true);
  } else {
    // Undeafening
    state.isDeafened = false;
    if (state.voiceChannelId) voiceSetDeafened(false);
    if (!state.wasMutedBeforeDeafen) {
      state.isMuted = false;
      if (state.voiceChannelId) voiceToggleMute();
    }
  }
  sendVoiceStateUpdate(state);
}

export async function toggleScreenShare(state: ServerState) {
  if (!state.voiceChannelId) return;
  if (state.isScreenSharing) {
    await voiceStopScreenShare();
    state.isScreenSharing = false;
  } else {
    const ok = await voiceStartScreenShare();
    state.isScreenSharing = ok;
  }
  state.videoTrackVersion++;
  sendVoiceStateUpdate(state);
}

export async function toggleCamera(state: ServerState) {
  if (!state.voiceChannelId) return;
  const next = !state.isCameraOn;
  const ok = await voiceSetCamera(next);
  if (ok) state.isCameraOn = next;
  state.videoTrackVersion++;
  sendVoiceStateUpdate(state);
}

export function forceMute(state: ServerState, userId: number, muted: boolean) {
  wsSend(state, { type: "ForceMute", data: { user_id: userId, muted } });
}

export function forceDeafen(state: ServerState, userId: number, deafened: boolean) {
  wsSend(state, { type: "ForceDeafen", data: { user_id: userId, deafened } });
}
