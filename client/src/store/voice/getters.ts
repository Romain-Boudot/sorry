import { activeState } from "../core";
import type { VoiceUserState } from "../../api";

export function isUserSpeaking(userId: number): boolean {
  const state = activeState();
  if (!state) return false;
  return state.speakingUsers.has(`user-${userId}`);
}

export function getUserVoiceState(channelId: number, userId: number): VoiceUserState | undefined {
  const state = activeState();
  if (!state) return undefined;
  return state.voiceState.get(channelId)?.get(userId);
}
