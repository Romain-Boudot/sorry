import { activeState, activeServer } from "../core";
import * as _voice from "../../composables/useVoice";

export async function joinVoiceChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _voice.joinVoiceChannel(server, state, channelId);
}

export async function leaveVoiceChannel() {
  const state = activeState();
  if (!state) return;
  await _voice.leaveVoiceChannel(state);
}

export function toggleMute() {
  const state = activeState();
  if (!state) return;
  _voice.toggleMute(state);
}

export function toggleDeafen() {
  const state = activeState();
  if (!state) return;
  _voice.toggleDeafen(state);
}

export async function toggleScreenShare() {
  const state = activeState();
  if (!state) return;
  await _voice.toggleScreenShare(state);
}

export async function toggleCamera() {
  const state = activeState();
  if (!state) return;
  await _voice.toggleCamera(state);
}

export function forceMute(userId: number, muted: boolean) {
  const state = activeState();
  if (!state) return;
  _voice.forceMute(state, userId, muted);
}

export function forceDeafen(userId: number, deafened: boolean) {
  const state = activeState();
  if (!state) return;
  _voice.forceDeafen(state, userId, deafened);
}
