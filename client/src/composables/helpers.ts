/**
 * Shared helpers for composables — avoids circular dependencies with store.ts.
 */
import type { ServerState } from "../store";

/** Send a JSON message over the WS connection. Returns true if sent. */
export function wsSend(state: ServerState, msg: object): boolean {
  const ws = state.wsConnection?.ws;
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));
    return true;
  }
  return false;
}

/** Send the current self voice state over WS. */
export function sendVoiceStateUpdate(state: ServerState) {
  if (!state.voiceChannelId) return;
  wsSend(state, {
    type: "UpdateVoiceState",
    data: {
      muted: state.isMuted,
      deafened: state.isDeafened,
      screen_sharing: state.isScreenSharing,
      camera_on: state.isCameraOn,
    },
  });
}
