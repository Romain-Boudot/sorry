// Miroir de shared/src/permissions.rs — ne pas réordonner
export const ADMINISTRATOR      = 1 << 0;
export const MANAGE_CHANNELS    = 1 << 1;
export const MANAGE_ROLES       = 1 << 2;
export const MANAGE_SERVER      = 1 << 3;
export const KICK_MEMBERS       = 1 << 4;
export const BAN_MEMBERS        = 1 << 5;
export const CREATE_INVITE      = 1 << 6;
export const CHANGE_NICKNAME    = 1 << 7;
export const MANAGE_NICKNAMES   = 1 << 8;
export const VIEW_CHANNELS      = 1 << 9;
export const SEND_MESSAGES      = 1 << 10;
export const MANAGE_MESSAGES    = 1 << 11;
export const READ_MESSAGE_HISTORY = 1 << 12;
export const ATTACH_FILES       = 1 << 13;
export const MENTION_EVERYONE   = 1 << 14;
export const ADD_REACTIONS      = 1 << 15;
export const EMBED_LINKS        = 1 << 16;
export const CONNECT            = 1 << 17;
export const SPEAK              = 1 << 18;
export const STREAM             = 1 << 19;
export const MUTE_MEMBERS       = 1 << 20;
export const DEAFEN_MEMBERS     = 1 << 21;
export const MOVE_MEMBERS       = 1 << 22;
export const USE_VOICE_ACTIVITY = 1 << 23;
export const PRIORITY_SPEAKER   = 1 << 24;

export function has(permissions: number, permission: number): boolean {
  return (permissions & ADMINISTRATOR) !== 0 || (permissions & permission) === permission;
}
