import { api } from "../../api";
import { activeState, activeServer, persistNav } from "../core";

export async function selectChannel(channelId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;

  state.activeChannelId = channelId;
  state.activeTab = "channels"; // revient à la vue channel (on garde activeDmUserId pour mémoriser le dernier DM)
  state.channelUnread.delete(channelId);
  state.channelMentions.delete(channelId);
  persistNav();
  if (!state.messages.has(channelId)) {
    const msgs = await api.listMessages(server.url, server.token, channelId);
    state.messages.set(channelId, msgs.reverse());
  }
}
