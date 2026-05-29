export { resolveBaseUrl } from "./client";
export * from "./types";
export { createWsConnection } from "./ws";

// Build the unified api object from all domain modules
import * as auth from "./auth";
import * as channels from "./channels";
import * as users from "./users";
import * as roles from "./roles";
import * as server from "./server";
import * as invites from "./invites";
import * as webhooks from "./webhooks";
import * as notifications from "./notifications";
import * as dms from "./dms";
import * as livekit from "./livekit";

export const api = {
  ...auth,
  ...channels,
  ...users,
  ...roles,
  ...server,
  ...invites,
  ...webhooks,
  ...notifications,
  ...dms,
  ...livekit,
};
