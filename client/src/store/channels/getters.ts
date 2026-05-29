import { activeState } from "../core";

export function isActiveChannelVoice(): boolean {
  const state = activeState();
  if (!state?.activeChannelId) return false;
  const ch = state.channels.find((c) => c.id === state.activeChannelId);
  return ch?.kind === "voice";
}
