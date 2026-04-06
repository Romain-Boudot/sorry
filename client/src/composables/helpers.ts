/**
 * Shared helpers for composables — avoids circular dependencies with store.ts.
 */
import type { ServerState } from "../store";

/** Send a JSON message over the WS connection. */
export function wsSend(state: ServerState, msg: object) {
  const ws = state.wsConnection?.ws;
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));
  }
}

/** Send the current self voice state over WS. */
export function sendVoiceStateUpdate(state: ServerState) {
  if (!state.voiceChannelId) return;
  wsSend(state, {
    type: "UpdateVoiceState",
    data: { muted: state.isMuted, deafened: state.isDeafened },
  });
}
