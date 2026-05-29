import { request } from "./client";

export function getLivekitToken(baseUrl: string, token: string, channelId: number) {
  return request<{ token: string; url: string }>(baseUrl, "/livekit/token", token, {
    method: "POST",
    body: JSON.stringify({ channel_id: channelId }),
  });
}
